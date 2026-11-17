#[derive(Debug, Clone)]
pub struct ParametricEqState {
    z1: f32,
    z2: f32,
}

impl Default for ParametricEqState {
    fn default() -> Self {
        Self { z1: 0.0, z2: 0.0 }
    }
}

impl ParametricEqState {
    pub fn process(&mut self, input: f32, frequency: f32, gain_db: f32, sample_rate: f32) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        let gain = 10.0f32.powf(gain_db / 20.0).max(1e-4);
        let normalized_frequency = frequency.max(1.0) * std::f32::consts::PI / sample_rate;
        let bandwidth = 2.0 * normalized_frequency / gain.sqrt();
        let coefficient_1 = -(2.0 * normalized_frequency).cos();
        let coefficient_2 = ((1.0 - bandwidth) / (1.0 + bandwidth)).clamp(-0.999, 0.999);
        let gain_term = 0.5 * (gain - 1.0);

        let y = input - coefficient_2 * self.z2;
        let output = input - gain_term * (self.z2 + coefficient_2 * y - input);
        let y = y - coefficient_1 * self.z1;
        self.z2 = self.z1 + coefficient_1 * y;
        self.z1 = y + 1e-20;
        output
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    fn tone_energy(frequency: f32, eq_frequency: f32, gain_db: f32) -> f32 {
        let mut state = ParametricEqState::default();
        let mut energy = 0.0;
        for step in 0..SAMPLE_RATE as usize {
            let input = (step as f32 / SAMPLE_RATE * frequency * std::f32::consts::TAU).sin() * 0.5;
            let output = state.process(input, eq_frequency, gain_db, SAMPLE_RATE);
            energy += output * output;
        }
        energy
    }

    #[test]
    fn boost_louder_than_cut_at_center() {
        let boosted = tone_energy(1_000.0, 1_000.0, 12.0);
        let cut = tone_energy(1_000.0, 1_000.0, -12.0);
        assert!(
            boosted > cut,
            "boost {boosted} should exceed cut {cut} at the center frequency"
        );
    }

    #[test]
    fn zero_gain_is_near_transparent() {
        let mut state = ParametricEqState::default();
        let mut peak = 0.0f32;
        for step in 0..SAMPLE_RATE as usize {
            let input = (step as f32 / SAMPLE_RATE * 500.0 * std::f32::consts::TAU).sin() * 0.5;
            let output = state.process(input, 1_000.0, 0.0, SAMPLE_RATE);
            peak = peak.max(output.abs());
        }
        assert!(
            (peak - 0.5).abs() < 0.05,
            "0 dB should be near transparent, got {peak}"
        );
    }

    #[test]
    fn output_stays_finite_at_extremes() {
        let mut state = ParametricEqState::default();
        for step in 0..1_000 {
            let output = state.process(0.5, 20.0 + step as f32, 24.0, SAMPLE_RATE);
            assert!(output.is_finite());
        }
    }
}
