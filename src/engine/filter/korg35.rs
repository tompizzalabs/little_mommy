use super::one_pole::VaOnePole;

const RESONANCE_MAX: f32 = 1.96;
const RESONANCE_MIN: f32 = 0.01;
const OVERDRIVE_TANH_FACTOR: f32 = 3.0;

#[derive(Debug, Clone)]
pub struct Korg35Filter {
    low_pass_1: VaOnePole,
    low_pass_2: VaOnePole,
    high_pass_1: VaOnePole,
    high_pass_2: VaOnePole,
    is_lowpass: bool,
    k: f32,
    k_modded: f32,
    alpha: f32,
    saturation: f32,
}

impl Default for Korg35Filter {
    fn default() -> Self {
        let mut low_pass_1 = VaOnePole::new();
        let mut low_pass_2 = VaOnePole::new();
        let mut high_pass_1 = VaOnePole::new();
        let mut high_pass_2 = VaOnePole::new();
        low_pass_1.set_lowpass();
        low_pass_2.set_lowpass();
        high_pass_1.set_highpass();
        high_pass_2.set_highpass();
        Self {
            low_pass_1,
            low_pass_2,
            high_pass_1,
            high_pass_2,
            is_lowpass: true,
            k: RESONANCE_MIN,
            k_modded: RESONANCE_MIN,
            alpha: 0.0,
            saturation: 0.0,
        }
    }
}

impl Korg35Filter {
    pub fn set_lowpass(&mut self) {
        self.is_lowpass = true;
    }

    pub fn set_highpass(&mut self) {
        self.is_lowpass = false;
    }

    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation.max(0.0);
    }

    fn update(&mut self, cutoff: f32, resonance: f32, sample_rate: f32) {
        self.k_modded = (self.k + resonance * 2.0).clamp(RESONANCE_MIN, RESONANCE_MAX);

        let warped = (2.0 * sample_rate) * (cutoff * std::f32::consts::TAU / sample_rate * 0.5).tan();
        let g = warped / sample_rate * 0.5;
        let stage_gain = g / (1.0 + g);

        self.low_pass_1.alpha = stage_gain;
        self.low_pass_2.alpha = stage_gain;
        self.high_pass_1.alpha = stage_gain;
        self.high_pass_2.alpha = stage_gain;

        self.alpha = 1.0 / (1.0 - self.k_modded * stage_gain + self.k_modded * stage_gain * stage_gain);

        if self.is_lowpass {
            self.low_pass_2.beta = (self.k_modded - self.k_modded * stage_gain) / (1.0 + g);
            self.high_pass_1.beta = -1.0 / (1.0 + g);
        } else {
            self.high_pass_2.beta = -stage_gain / (1.0 + g);
            self.low_pass_1.beta = 1.0 / (1.0 + g);
        }
    }

    pub fn process(&mut self, input: f32, cutoff: f32, resonance: f32, sample_rate: f32) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }
        self.k = resonance.clamp(0.0, 1.0) * 1.95 + RESONANCE_MIN;
        self.update(cutoff, resonance, sample_rate);

        let output;
        if self.is_lowpass {
            let first = self.low_pass_1.process(input);
            let feedback_sum = self.low_pass_2.feedback_output() + self.high_pass_1.feedback_output();
            let stage_input = self.alpha * (first + feedback_sum);
            output = self.k_modded * self.low_pass_2.process(stage_input);
            self.high_pass_1.process(output);
        } else {
            let first = self.high_pass_1.process(input);
            let feedback_sum = self.high_pass_2.feedback_output() + self.low_pass_1.feedback_output();
            let stage_input = self.alpha * (first + feedback_sum);
            let scaled = self.k_modded * stage_input;
            self.low_pass_1.process(self.high_pass_2.process(scaled));
            output = scaled;
        }

        let normalized = output / self.k_modded;
        self.apply_overdrive(normalized, OVERDRIVE_TANH_FACTOR)
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
        self.low_pass_1.reset();
        self.low_pass_2.reset();
        self.high_pass_1.reset();
        self.high_pass_2.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn low_pass_passes_constant_input() {
        let mut filter = Korg35Filter::default();
        let mut tail = 0.0;
        for _ in 0..SAMPLE_RATE as usize {
            tail = filter.process(0.5, 500.0, 0.2, SAMPLE_RATE);
        }
        assert!(tail.abs() > 0.1, "constant input should survive, got {tail}");
    }

    #[test]
    fn high_pass_blocks_constant_input() {
        let mut filter = Korg35Filter::default();
        filter.set_highpass();
        let mut tail = 0.0;
        for _ in 0..SAMPLE_RATE as usize {
            tail = filter.process(0.5, 500.0, 0.2, SAMPLE_RATE);
        }
        assert!(tail.abs() < 0.05, "constant input should vanish, got {tail}");
    }

    #[test]
    fn output_stays_finite_at_full_resonance() {
        let mut filter = Korg35Filter::default();
        for _ in 0..SAMPLE_RATE as usize {
            let output = filter.process(0.5, 3_000.0, 1.0, SAMPLE_RATE);
            assert!(output.is_finite());
        }
    }
}
