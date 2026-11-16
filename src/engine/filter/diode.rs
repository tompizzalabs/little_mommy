use super::one_pole::VaOnePole;

const RESONANCE_MAX: f32 = 16.0;
const OVERDRIVE_TANH_FACTOR: f32 = 3.5;
const PASSBAND_GAIN_COMPENSATION: f32 = 0.3;

#[derive(Debug, Clone)]
pub struct DiodeFilter {
    stages: [VaOnePole; 4],
    k: f32,
    gamma: f32,
    feedback_scalars: [f32; 4],
    saturation: f32,
}

impl Default for DiodeFilter {
    fn default() -> Self {
        Self {
            stages: [
                VaOnePole::new(),
                VaOnePole::new(),
                VaOnePole::new(),
                VaOnePole::new(),
            ],
            k: 0.0,
            gamma: 0.0,
            feedback_scalars: [0.0; 4],
            saturation: 0.0,
        }
    }
}

impl DiodeFilter {
    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation.max(0.0);
    }

    fn update(&mut self, cutoff: f32, sample_rate: f32) {
        let warped =
            (2.0 * sample_rate) * (cutoff * std::f32::consts::TAU / sample_rate * 0.5).tan();
        let g = warped / sample_rate / 2.0;

        let g4 = 0.5 * g / (1.0 + g);
        let g3 = 0.5 * g / (1.0 + g - 0.5 * g * g4);
        let g2 = 0.5 * g / (1.0 + g - 0.5 * g * g3);
        let g1 = g / (1.0 + g - g * g2);
        self.gamma = g4 * g3 * g2 * g1;

        self.feedback_scalars = [g4 * g3 * g2, g4 * g3, g4, 1.0];

        let stage_gain = g / (1.0 + g);

        self.stages[0].alpha = stage_gain;
        self.stages[1].alpha = stage_gain;
        self.stages[2].alpha = stage_gain;
        self.stages[3].alpha = stage_gain;

        self.stages[0].beta = 1.0 / (1.0 + g - g * g2);
        self.stages[1].beta = 1.0 / (1.0 + g - 0.5 * g * g3);
        self.stages[2].beta = 1.0 / (1.0 + g - 0.5 * g * g4);
        self.stages[3].beta = 1.0 / (1.0 + g);

        self.stages[0].delta = g;
        self.stages[1].delta = 0.5 * g;
        self.stages[2].delta = 0.5 * g;
        self.stages[3].delta = 0.0;

        self.stages[0].gamma = 1.0 + g1 * g2;
        self.stages[1].gamma = 1.0 + g2 * g3;
        self.stages[2].gamma = 1.0 + g3 * g4;
        self.stages[3].gamma = 1.0;

        self.stages[0].epsilon = g2;
        self.stages[1].epsilon = g3;
        self.stages[2].epsilon = g4;
        self.stages[3].epsilon = 0.0;

        self.stages[0].input_gain = 1.0;
        self.stages[1].input_gain = 0.5;
        self.stages[2].input_gain = 0.5;
        self.stages[3].input_gain = 0.5;
    }

    pub fn process(&mut self, input: f32, cutoff: f32, resonance: f32, sample_rate: f32) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }
        self.k = RESONANCE_MAX * resonance.clamp(0.0, 1.0);
        self.update(cutoff, sample_rate);

        self.stages[3].feedback = 0.0;
        self.stages[2].feedback = self.stages[3].feedback_output();
        self.stages[1].feedback = self.stages[2].feedback_output();
        self.stages[0].feedback = self.stages[1].feedback_output();

        let feedback_sum = self.feedback_scalars[0] * self.stages[0].feedback_output()
            + self.feedback_scalars[1] * self.stages[1].feedback_output()
            + self.feedback_scalars[2] * self.stages[2].feedback_output()
            + self.feedback_scalars[3] * self.stages[3].feedback_output();

        let compensated = input * (1.0 + PASSBAND_GAIN_COMPENSATION * self.k);
        let stage_input = (compensated - self.k * feedback_sum) / (1.0 + self.k * self.gamma);

        let stage1 = self.stages[0].process(stage_input);
        let stage2 = self.stages[1].process(stage1);
        let stage3 = self.stages[2].process(stage2);
        let output = self.stages[3].process(stage3);

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
    fn passes_constant_input() {
        let mut filter = DiodeFilter::default();
        let mut tail = 0.0;
        for _ in 0..SAMPLE_RATE as usize {
            tail = filter.process(0.5, 500.0, 0.3, SAMPLE_RATE);
        }
        assert!(
            tail.abs() > 0.1,
            "constant input should survive, got {tail}"
        );
    }

    #[test]
    fn output_stays_finite_at_full_resonance() {
        let mut filter = DiodeFilter::default();
        for _ in 0..SAMPLE_RATE as usize {
            let output = filter.process(0.5, 2_000.0, 1.0, SAMPLE_RATE);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn reset_clears_stages() {
        let mut filter = DiodeFilter::default();
        for _ in 0..1_000 {
            filter.process(1.0, 1_000.0, 0.5, SAMPLE_RATE);
        }
        filter.reset();
        let mut fresh = DiodeFilter::default();
        let reset_output = filter.process(0.5, 500.0, 0.1, SAMPLE_RATE);
        let fresh_output = fresh.process(0.5, 500.0, 0.1, SAMPLE_RATE);
        assert_eq!(reset_output, fresh_output);
    }
}
