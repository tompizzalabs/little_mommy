use crate::engine::filter::CombFilter;

const LFO_MAX_RANGE: f32 = 0.0095;
const FEEDBACK_LIMIT: f32 = 0.97;

#[derive(Debug, Clone)]
pub struct FlangerState {
    comb: CombFilter,
    lfo_index: f32,
    lfo_sign: f32,
    lfo_frequency: f32,
    base_time: f32,
}

impl Default for FlangerState {
    fn default() -> Self {
        Self {
            comb: CombFilter::default(),
            lfo_index: 0.0,
            lfo_sign: 1.0,
            lfo_frequency: 0.2,
            base_time: 0.0105,
        }
    }
}

impl FlangerState {
    pub fn process(
        &mut self,
        input: f32,
        rate: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
        sample_rate: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        self.lfo_frequency = rate.max(0.001);
        self.lfo_index += 2.0 * self.lfo_frequency / sample_rate;
        while self.lfo_index > 1.0 {
            self.lfo_index -= 1.0;
            self.lfo_sign *= -1.0;
        }
        let lfo = 4.0 * (self.lfo_index * (1.0 - self.lfo_index)) * self.lfo_sign;

        let amount_modded = amount.clamp(0.0, 1.0);
        let delay_time = self.base_time + lfo * amount_modded * LFO_MAX_RANGE;
        let frequency = 1.0 / delay_time.max(1e-5);
        let limited_feedback = feedback.clamp(-FEEDBACK_LIMIT, FEEDBACK_LIMIT);
        let dry_wet_modded = dry_wet.clamp(0.0, 1.0);

        let flanged = self
            .comb
            .process(input, frequency, limited_feedback, sample_rate);
        flanged * dry_wet_modded + (1.0 - dry_wet_modded) * input
    }

    pub fn reset(&mut self) {
        self.comb.reset();
        self.lfo_index = 0.0;
        self.lfo_sign = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn output_stays_finite_over_lfo_cycle() {
        let mut state = FlangerState::default();
        for step in 0..SAMPLE_RATE as usize {
            let input = ((step as f32 / SAMPLE_RATE * 220.0 * std::f32::consts::TAU).sin()) * 0.5;
            let output = state.process(input, 0.2, 0.3, 0.6, 1.0, SAMPLE_RATE);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn dry_wet_zero_is_transparent() {
        let mut state = FlangerState::default();
        let output = state.process(0.37, 0.2, 0.3, 0.6, 0.0, SAMPLE_RATE);
        assert!((output - 0.37).abs() < 1e-6);
    }

    #[test]
    fn reset_clears_lfo() {
        let mut state = FlangerState::default();
        for _ in 0..1_000 {
            state.process(0.5, 0.2, 0.3, 0.6, 1.0, SAMPLE_RATE);
        }
        state.reset();
        assert_eq!(state.lfo_index, 0.0);
        assert_eq!(state.lfo_sign, 1.0);
    }
}
