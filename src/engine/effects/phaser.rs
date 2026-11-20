use crate::engine::filter::Allpass;

const ALLPASS_COUNT: usize = 12;
const PHASER_MAX_LFO_AMPLITUDE: f32 = 4000.0;
const FEEDBACK_LIMIT: f32 = 0.97;
const FREQUENCY_MIN: f32 = 400.0;
const FREQUENCY_MAX: f32 = 8000.0;
const WIDTH_SPREAD: [f32; ALLPASS_COUNT] = [
    -0.4, 0.0, 0.2, 0.4, -0.13, -0.34, -0.375, -0.23, 0.13, 0.36, -0.09, -0.29,
];
const ALLPASS_RADIUS: f32 = 1.32;

#[derive(Debug, Clone)]
struct PhaserChannel {
    allpasses: [Allpass; ALLPASS_COUNT],
    lfo_index: f32,
    lfo_sign: f32,
    stored_output: f32,
}

impl PhaserChannel {
    fn new(lfo_offset: f32) -> Self {
        Self {
            allpasses: std::array::from_fn(|_| Allpass::default()),
            lfo_index: lfo_offset,
            lfo_sign: 1.0,
            stored_output: 0.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        input: f32,
        lfo: f32,
        base_frequency: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
        sample_rate: f32,
    ) -> f32 {
        let input_with_feedback = input + self.stored_output * feedback;
        let center_frequency = (base_frequency + lfo * amount * PHASER_MAX_LFO_AMPLITUDE)
            .clamp(FREQUENCY_MIN, FREQUENCY_MAX);
        for (allpass, spread) in self.allpasses.iter_mut().zip(WIDTH_SPREAD) {
            allpass.set_radius(ALLPASS_RADIUS);
            allpass.set_frequency(center_frequency * (1.0 + spread), sample_rate);
        }
        let mut phase_shifted = input_with_feedback;
        for allpass in self.allpasses.iter_mut() {
            phase_shifted = allpass.process(phase_shifted);
        }
        self.stored_output = phase_shifted;
        (1.0 - dry_wet) * input + dry_wet * phase_shifted
    }

    fn step_lfo(&mut self, increment: f32) -> f32 {
        self.lfo_index += increment;
        while self.lfo_index > 1.0 {
            self.lfo_index -= 1.0;
            self.lfo_sign *= -1.0;
        }
        4.0 * (self.lfo_index * (1.0 - self.lfo_index)) * self.lfo_sign
    }

    fn reset(&mut self) {
        for allpass in self.allpasses.iter_mut() {
            allpass.reset();
        }
        self.stored_output = 0.0;
    }
}

#[derive(Debug, Clone)]
pub struct PhaserState {
    left: PhaserChannel,
    right: PhaserChannel,
    rate: f32,
    base_frequency: f32,
}

impl Default for PhaserState {
    fn default() -> Self {
        Self {
            left: PhaserChannel::new(0.0),
            right: PhaserChannel::new(0.5),
            rate: 0.5,
            base_frequency: 4000.0,
        }
    }
}

impl PhaserState {
    #[allow(clippy::too_many_arguments)]
    pub fn process_stereo(
        &mut self,
        input_left: f32,
        input_right: f32,
        rate: f32,
        base_frequency: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        if sample_rate <= 0.0 {
            return (0.0, 0.0);
        }

        self.rate = rate.max(0.001);
        self.base_frequency = base_frequency.clamp(FREQUENCY_MIN, FREQUENCY_MAX);
        let amount_modded = amount.clamp(0.0, 1.5);
        let feedback_modded = (feedback * FEEDBACK_LIMIT).clamp(0.0, FEEDBACK_LIMIT);
        let dry_wet_modded = (dry_wet * 0.5).clamp(0.0, 0.5);

        let increment = 2.0 * self.rate / sample_rate;
        let lfo_left = self.left.step_lfo(increment);
        let lfo_right = self.right.step_lfo(increment);

        let out_left = self.left.process(
            input_left,
            lfo_left,
            self.base_frequency,
            amount_modded,
            feedback_modded,
            dry_wet_modded,
            sample_rate,
        );
        let out_right = self.right.process(
            input_right,
            lfo_right,
            self.base_frequency,
            amount_modded,
            feedback_modded,
            dry_wet_modded,
            sample_rate,
        );
        (out_left, out_right)
    }

    pub fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
        self.left.lfo_index = 0.0;
        self.right.lfo_index = 0.5;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn output_stays_finite_over_lfo_cycle() {
        let mut state = PhaserState::default();
        for step in 0..SAMPLE_RATE as usize {
            let input = ((step as f32 / SAMPLE_RATE * 220.0 * std::f32::consts::TAU).sin()) * 0.5;
            let (left, right) =
                state.process_stereo(input, input, 0.5, 4000.0, 0.3, 0.5, 1.0, SAMPLE_RATE);
            assert!(left.is_finite() && right.is_finite());
        }
    }

    #[test]
    fn dry_wet_zero_is_transparent() {
        let mut state = PhaserState::default();
        let (left, right) =
            state.process_stereo(0.37, -0.37, 0.5, 4000.0, 0.3, 0.5, 0.0, SAMPLE_RATE);
        assert!((left - 0.37).abs() < 1e-6);
        assert!((right - -0.37).abs() < 1e-6);
    }

    #[test]
    fn feedback_never_explodes() {
        let mut state = PhaserState::default();
        for _ in 0..SAMPLE_RATE as usize {
            let (left, _) = state.process_stereo(0.5, 0.5, 2.0, 4000.0, 1.0, 1.0, 1.0, SAMPLE_RATE);
            assert!(left.is_finite());
        }
    }

    #[test]
    fn reset_restores_lfo_offsets() {
        let mut state = PhaserState::default();
        for _ in 0..1_000 {
            state.process_stereo(0.5, 0.5, 0.5, 4000.0, 0.3, 0.5, 1.0, SAMPLE_RATE);
        }
        state.reset();
        assert_eq!(state.left.lfo_index, 0.0);
        assert_eq!(state.right.lfo_index, 0.5);
    }
}
