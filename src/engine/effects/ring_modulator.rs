#[derive(Debug, Clone)]
pub struct RingModState {
    phase: f32,
    frequency: f32,
}

impl Default for RingModState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            frequency: 100.0,
        }
    }
}

impl RingModState {
    pub fn process_stereo(
        &mut self,
        input_left: f32,
        input_right: f32,
        frequency: f32,
        amount: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        if sample_rate <= 0.0 {
            return (0.0, 0.0);
        }

        self.frequency = frequency.clamp(0.01, 20_000.0);
        self.phase += self.frequency / sample_rate;
        while self.phase > 1.0 {
            self.phase -= 1.0;
        }
        let carrier = (self.phase * std::f32::consts::TAU).sin();
        let blend = amount.clamp(0.0, 1.0);
        let modulated = carrier * blend + (1.0 - blend);

        (input_left * modulated, input_right * modulated)
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn full_amount_is_amplitude_modulation() {
        let mut state = RingModState::default();
        let mut positive_peaks = 0.0f32;
        for _ in 0..SAMPLE_RATE as usize {
            let (left, _) = state.process_stereo(0.5, 0.5, 100.0, 1.0, SAMPLE_RATE);
            positive_peaks = positive_peaks.max(left.abs());
        }
        assert!(
            positive_peaks <= 0.501,
            "carrier never exceeds unity, got {positive_peaks}"
        );
    }

    #[test]
    fn zero_amount_is_transparent() {
        let mut state = RingModState::default();
        let (left, right) = state.process_stereo(0.37, -0.37, 100.0, 0.0, SAMPLE_RATE);
        assert!((left - 0.37).abs() < 1e-6);
        assert!((right - -0.37).abs() < 1e-6);
    }

    #[test]
    fn carrier_frequency_matches_parameter() {
        let mut state = RingModState::default();
        let mut crossings = 0;
        let mut previous = 0.0f32;
        for _ in 0..(SAMPLE_RATE * 0.1) as usize {
            let (left, _) = state.process_stereo(0.5, 0.5, 100.0, 1.0, SAMPLE_RATE);
            if previous <= 0.0 && left > 0.0 {
                crossings += 1;
            }
            previous = left;
        }
        assert!(
            (crossings as f32 - 10.0).abs() <= 1.0,
            "100 Hz over 0.1 s should cross zero ~10 times, got {crossings}"
        );
    }
}
