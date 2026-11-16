use super::one_pole::VaOnePole;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LadderMode {
    LowPass4,
    LowPass2,
    BandPass4,
    BandPass2,
    HighPass4,
    HighPass2,
}

const RESONANCE_MAX: f32 = 3.88;
const OVERDRIVE_TANH_FACTOR: f32 = 3.5;

#[derive(Debug, Clone)]
pub struct LadderFilter {
    stages: [VaOnePole; 4],
    mode: LadderMode,
    resonance: f32,
    gamma: f32,
    alpha_0: f32,
    mix: [f32; 5],
    saturation: f32,
}

impl Default for LadderFilter {
    fn default() -> Self {
        let mut stages = [
            VaOnePole::new(),
            VaOnePole::new(),
            VaOnePole::new(),
            VaOnePole::new(),
        ];
        for stage in stages.iter_mut() {
            stage.set_lowpass();
        }
        Self {
            stages,
            mode: LadderMode::LowPass4,
            resonance: 0.0,
            gamma: 0.0,
            alpha_0: 1.0,
            mix: [0.0; 5],
            saturation: 0.0,
        }
    }
}

impl LadderFilter {
    pub fn set_mode(&mut self, mode: LadderMode) {
        self.mode = mode;
    }

    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation.max(0.0);
    }

    fn update(&mut self, cutoff: f32, resonance: f32, sample_rate: f32) {
        let resonance_modded = (self.resonance + 4.0 * resonance.max(0.0)).clamp(0.0, RESONANCE_MAX);

        let warped = (2.0 * sample_rate)
            * (cutoff * std::f32::consts::TAU / sample_rate * 0.5).tan();
        let g = warped / sample_rate * 0.5;
        let stage_gain = g / (1.0 + g);

        self.stages[0].alpha = stage_gain;
        self.stages[1].alpha = stage_gain;
        self.stages[2].alpha = stage_gain;
        self.stages[3].alpha = stage_gain;

        self.stages[0].beta = stage_gain * stage_gain * stage_gain / (1.0 + g);
        self.stages[1].beta = stage_gain * stage_gain / (1.0 + g);
        self.stages[2].beta = stage_gain / (1.0 + g);
        self.stages[3].beta = 1.0 / (1.0 + g);

        self.gamma = stage_gain * stage_gain * stage_gain * stage_gain;
        self.alpha_0 = 1.0 / (1.0 + resonance_modded * self.gamma);

        self.mix = match self.mode {
            LadderMode::LowPass4 => [0.0, 0.0, 0.0, 0.0, 1.0],
            LadderMode::LowPass2 => [0.0, 0.0, 1.0, 0.0, 0.0],
            LadderMode::BandPass4 => [0.0, 0.0, 4.0, -8.0, 4.0],
            LadderMode::BandPass2 => [0.0, 2.0, -2.0, 0.0, 0.0],
            LadderMode::HighPass4 => [1.0, -4.0, 6.0, -4.0, 1.0],
            LadderMode::HighPass2 => [1.0, -2.0, 1.0, 0.0, 0.0],
        };
    }

    pub fn process(&mut self, input: f32, cutoff: f32, resonance: f32, sample_rate: f32) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }
        self.resonance = RESONANCE_MAX * resonance.clamp(0.0, 1.0);
        self.update(cutoff, resonance, sample_rate);

        let feedback_sum = self.stages[0].feedback_output()
            + self.stages[1].feedback_output()
            + self.stages[2].feedback_output()
            + self.stages[3].feedback_output();
        let stage_input = (input - self.resonance * feedback_sum) * self.alpha_0;

        let stage1 = self.stages[0].process(stage_input);
        let stage2 = self.stages[1].process(stage1);
        let stage3 = self.stages[2].process(stage2);
        let stage4 = self.stages[3].process(stage3);

        let output = self.mix[0] * stage_input
            + self.mix[1] * stage1
            + self.mix[2] * stage2
            + self.mix[3] * stage3
            + self.mix[4] * stage4;

        self.apply_overdrive(output, OVERDRIVE_TANH_FACTOR)
    }

    fn apply_overdrive(&self, input: f32, tanh_factor: f32) -> f32 {
        let saturation = self.saturation;
        if saturation > 0.01 && saturation < 1.0 {
            input * (1.0 - saturation) + saturation * (tanh_factor * input).tanh()
        } else if saturation >= 1.0 {
            (tanh_factor * input).tanh()
        } else {
            input
        }
    }

    pub fn reset(&mut self) {
        for stage in self.stages.iter_mut() {
            stage.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn low_pass_mode_passes_low_frequencies() {
        let mut filter = LadderFilter::default();
        filter.set_mode(LadderMode::LowPass4);

        let mut energy = 0.0;
        for _ in 0..SAMPLE_RATE as usize {
            energy += filter.process(0.5, 500.0, 0.1, SAMPLE_RATE).powi(2);
        }
        assert!(energy.sqrt() > 0.2, "constant tone should survive low pass");
    }

    #[test]
    fn high_pass_mode_blocks_low_frequencies() {
        let mut filter = LadderFilter::default();
        filter.set_mode(LadderMode::HighPass4);

        let mut tail = 0.0;
        for _ in 0..SAMPLE_RATE as usize {
            tail = filter.process(0.5, 500.0, 0.1, SAMPLE_RATE);
        }
        assert!(tail.abs() < 0.05, "constant input should vanish, got {tail}");
    }

    #[test]
    fn resonance_stays_finite_at_maximum() {
        let mut filter = LadderFilter::default();
        for _ in 0..SAMPLE_RATE as usize {
            let output = filter.process(0.5, 5_000.0, 1.0, SAMPLE_RATE);
            assert!(output.is_finite(), "self oscillation must not explode");
        }
    }

    #[test]
    fn reset_clears_stages() {
        let mut filter = LadderFilter::default();
        for _ in 0..1_000 {
            filter.process(1.0, 1_000.0, 0.5, SAMPLE_RATE);
        }
        filter.reset();
        let mut fresh = LadderFilter::default();
        let reset_output = filter.process(0.5, 500.0, 0.1, SAMPLE_RATE);
        let fresh_output = fresh.process(0.5, 500.0, 0.1, SAMPLE_RATE);
        assert_eq!(reset_output, fresh_output);
    }
}
