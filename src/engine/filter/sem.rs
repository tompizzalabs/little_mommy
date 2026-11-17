const RESONANCE_MIN: f32 = 0.5;
const RESONANCE_MAX: f32 = 25.0;

#[derive(Debug, Clone)]
pub struct SemFilter {
    z1: f32,
    z2: f32,
    alpha: f32,
    alpha_0: f32,
    rho: f32,
    resonance_modded: f32,
}

impl Default for SemFilter {
    fn default() -> Self {
        Self {
            z1: 0.0,
            z2: 0.0,
            alpha: 1.0,
            alpha_0: 1.0,
            rho: 1.0,
            resonance_modded: RESONANCE_MIN,
        }
    }
}

impl SemFilter {
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    pub fn process(
        &mut self,
        input: f32,
        cutoff: f32,
        resonance: f32,
        transition: f32,
        sample_rate: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        let resonance_01 = resonance.clamp(0.0, 1.0);
        self.resonance_modded = (24.5 * resonance_01 * resonance_01 * resonance_01 * resonance_01
            + RESONANCE_MIN)
            .clamp(RESONANCE_MIN, RESONANCE_MAX);
        let r = 1.0 / (2.0 * self.resonance_modded);

        let warped =
            (2.0 * sample_rate) * (cutoff * std::f32::consts::TAU / sample_rate * 0.5).tan();
        let g = warped / sample_rate * 0.5;

        self.alpha_0 = 1.0 / (1.0 + 2.0 * r * g + g * g);
        self.alpha = g;
        self.rho = 2.0 * r + g;

        let high_pass = self.alpha_0 * (input - self.rho * self.z1 - self.z2);
        let band_pass = self.alpha * high_pass + self.z1;
        let low_pass = self.alpha * band_pass + self.z2;
        let band_stop = input - 2.0 * r * band_pass;

        self.z1 = self.alpha * high_pass + band_pass;
        self.z2 = self.alpha * band_pass + low_pass;

        let transition_modded = transition.clamp(-1.0, 1.0);
        if transition_modded < 0.0 {
            (1.0 + transition_modded) * band_stop - transition_modded * low_pass
        } else {
            transition_modded * high_pass + (1.0 - transition_modded) * band_stop
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn full_negative_transition_is_low_pass() {
        let mut filter = SemFilter::default();
        let mut tail = 0.0;
        for _ in 0..SAMPLE_RATE as usize {
            tail = filter.process(0.5, 500.0, 0.3, -1.0, SAMPLE_RATE);
        }
        assert!(
            tail.abs() > 0.1,
            "constant input passes a low pass, got {tail}"
        );
    }

    #[test]
    fn full_positive_transition_is_high_pass() {
        let mut filter = SemFilter::default();
        let mut tail = 0.0;
        for _ in 0..SAMPLE_RATE as usize {
            tail = filter.process(0.5, 500.0, 0.3, 1.0, SAMPLE_RATE);
        }
        assert!(
            tail.abs() < 0.05,
            "constant input dies in a high pass, got {tail}"
        );
    }

    #[test]
    fn output_stays_finite_at_full_resonance() {
        let mut filter = SemFilter::default();
        for _ in 0..SAMPLE_RATE as usize {
            let output = filter.process(0.5, 4_000.0, 1.0, 0.0, SAMPLE_RATE);
            assert!(output.is_finite());
        }
    }
}
