#[derive(Debug, Clone)]
pub struct StateVariableFilter {
    low_left: f32,
    band_left: f32,
    low_right: f32,
    band_right: f32,
}

impl Default for StateVariableFilter {
    fn default() -> Self {
        Self {
            low_left: 0.0,
            band_left: 0.0,
            low_right: 0.0,
            band_right: 0.0,
        }
    }
}

impl StateVariableFilter {
    pub fn process_stereo(
        &mut self,
        input_left: f32,
        input_right: f32,
        cutoff: f32,
        resonance: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        if sample_rate <= 0.0 {
            return (0.0, 0.0);
        }

        let normalized_cutoff = (cutoff / sample_rate).clamp(0.0, 0.45);
        let cutoff_coefficient = (2.0 * (std::f32::consts::PI * normalized_cutoff).sin()).min(1.0);
        let damping = 1.0 - resonance.clamp(0.0, 0.95);

        self.low_left += cutoff_coefficient * self.band_left;
        let high_left = input_left - self.low_left - damping * self.band_left;
        self.band_left += cutoff_coefficient * high_left;

        self.low_right += cutoff_coefficient * self.band_right;
        let high_right = input_right - self.low_right - damping * self.band_right;
        self.band_right += cutoff_coefficient * high_right;

        if !self.low_left.is_finite() || !self.band_left.is_finite() {
            self.low_left = 0.0;
            self.band_left = 0.0;
        }
        if !self.low_right.is_finite() || !self.band_right.is_finite() {
            self.low_right = 0.0;
            self.band_right = 0.0;
        }

        (self.low_left, self.low_right)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn low_pass_output_passes_constant_input() {
        let mut filter = StateVariableFilter::default();
        let mut tail = 0.0;
        for _ in 0..SAMPLE_RATE as usize {
            let (left, _) = filter.process_stereo(0.5, 0.5, 5_000.0, 0.2, SAMPLE_RATE);
            tail = left;
        }
        assert!(tail.abs() > 0.1, "constant input should survive, got {tail}");
    }

    #[test]
    fn resonance_never_explodes() {
        let mut filter = StateVariableFilter::default();
        for _ in 0..SAMPLE_RATE as usize {
            let (left, right) = filter.process_stereo(0.5, -0.5, 12_000.0, 0.94, SAMPLE_RATE);
            assert!(left.is_finite() && right.is_finite());
        }
    }
}
