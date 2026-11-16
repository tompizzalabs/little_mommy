const RESONATOR_GAIN_FACTOR: f32 = 0.08;
const DEFAULT_RESONATOR_RADIUS: f32 = 0.996;
const DEFAULT_RESONATOR_FREQUENCY: f32 = 2000.0;

#[derive(Debug, Clone)]
pub struct Biquad {
    a1: f32,
    a2: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    z1: f32,
    z2: f32,
}

impl Default for Biquad {
    fn default() -> Self {
        Self {
            a1: 0.0,
            a2: 0.0,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }
}

impl Biquad {
    pub fn set_coefficients(&mut self, a1: f32, a2: f32, b0: f32, b1: f32, b2: f32) {
        self.a1 = a1;
        self.a2 = a2;
        self.b0 = b0;
        self.b1 = b1;
        self.b2 = b2;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let feedback = input - self.z2 * self.a2 - self.z1 * self.a1;
        let output = self.b2 * self.z2 + self.b1 * self.z1 + self.b0 * feedback;
        self.z2 = self.z1;
        self.z1 = feedback;
        output
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[derive(Debug, Clone)]
pub struct Resonator {
    biquad: Biquad,
    radius: f32,
    frequency: f32,
}

impl Default for Resonator {
    fn default() -> Self {
        let mut resonator = Self {
            biquad: Biquad::default(),
            radius: DEFAULT_RESONATOR_RADIUS,
            frequency: DEFAULT_RESONATOR_FREQUENCY,
        };
        resonator.recalculate_coefficients(48_000.0);
        resonator
    }
}

impl Resonator {
    pub fn set_radius(&mut self, radius: f32, sample_rate: f32) {
        self.radius = radius;
        self.recalculate_coefficients(sample_rate);
    }

    pub fn set_frequency(&mut self, frequency: f32, sample_rate: f32) {
        self.frequency = frequency;
        self.recalculate_coefficients(sample_rate);
    }

    fn recalculate_coefficients(&mut self, sample_rate: f32) {
        let coefficient = -2.0 * self.radius * (self.frequency * std::f32::consts::TAU / sample_rate).cos();
        self.biquad.set_coefficients(coefficient, self.radius * self.radius, RESONATOR_GAIN_FACTOR, 0.0, -RESONATOR_GAIN_FACTOR);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.biquad.process(input)
    }

    pub fn reset(&mut self) {
        self.biquad.reset();
    }
}

#[derive(Debug, Clone)]
pub struct Allpass {
    biquad: Biquad,
    radius: f32,
}

impl Default for Allpass {
    fn default() -> Self {
        Self {
            biquad: Biquad::default(),
            radius: 1.0,
        }
    }
}

impl Allpass {
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    pub fn set_frequency(&mut self, frequency: f32, sample_rate: f32) {
        let angular = frequency * std::f32::consts::TAU / sample_rate;
        let reciprocal_radius = 1.0 / self.radius;
        self.biquad.set_coefficients(
            -2.0 * reciprocal_radius * angular.cos(),
            reciprocal_radius * reciprocal_radius,
            reciprocal_radius * reciprocal_radius,
            -2.0 * reciprocal_radius * angular.cos(),
            1.0,
        );
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.biquad.process(input)
    }

    pub fn reset(&mut self) {
        self.biquad.reset();
    }
}

#[derive(Debug, Clone)]
pub struct PeakingEq {
    biquad: Biquad,
    q: f32,
    frequency: f32,
    gain_db: f32,
}

impl Default for PeakingEq {
    fn default() -> Self {
        Self {
            biquad: Biquad::default(),
            q: 1.0,
            frequency: 1000.0,
            gain_db: 0.0,
        }
    }
}

impl PeakingEq {
    pub fn set_q(&mut self, q: f32) {
        self.q = q;
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    pub fn set_gain_db(&mut self, gain_db: f32) {
        self.gain_db = gain_db;
    }

    fn recalculate_coefficients(&mut self, sample_rate: f32) {
        let angular = self.frequency * std::f32::consts::TAU / sample_rate;
        let alpha = angular.sin() / (2.0 * self.q);
        let amplitude = 10.0f32.powf(self.gain_db / 40.0);
        let denominator = 1.0 + alpha / amplitude;

        let b0 = (1.0 + alpha * amplitude) / denominator;
        let b1 = -2.0 * angular.cos() / denominator;
        let b2 = (1.0 - alpha * amplitude) / denominator;
        let a2 = (1.0 - alpha / amplitude) / denominator;

        self.biquad.set_coefficients(b1, a2, b0, b1, b2);
    }

    pub fn process(&mut self, input: f32, sample_rate: f32) -> f32 {
        self.recalculate_coefficients(sample_rate);
        self.biquad.process(input)
    }

    pub fn reset(&mut self) {
        self.biquad.reset();
    }
}

#[derive(Debug, Clone)]
pub struct DcBlocker {
    previous_input: f32,
    previous_output: f32,
    coefficient: f32,
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self {
            previous_input: 0.0,
            previous_output: 0.0,
            coefficient: 0.995,
        }
    }
}

impl DcBlocker {
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.coefficient = if sample_rate > 120_000.0 {
            0.997
        } else if sample_rate > 90_000.0 {
            0.9965
        } else {
            0.995
        };
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = input - self.previous_input + self.coefficient * self.previous_output;
        self.previous_input = input;
        self.previous_output = output;
        output
    }

    pub fn reset(&mut self) {
        self.previous_input = 0.0;
        self.previous_output = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn resonator_rings_at_center_frequency() {
        let mut resonator = Resonator::default();
        resonator.set_frequency(1_000.0, SAMPLE_RATE);

        let mut peak = 0.0f32;
        for step in 0..SAMPLE_RATE as usize {
            let impulse = if step == 0 { 1.0 } else { 0.0 };
            let output = resonator.process(impulse);
            assert!(output.is_finite());
            peak = peak.max(output.abs());
        }
        assert!(peak > 0.001, "resonator should ring, peak {peak}");
    }

    #[test]
    fn peaking_eq_boosts_center_band() {
        let mut flat = PeakingEq::default();
        let mut boosted = PeakingEq::default();
        boosted.set_gain_db(24.0);

        let mut energy_flat = 0.0;
        let mut energy_boosted = 0.0;
        for step in 0..SAMPLE_RATE as usize {
            let input = ((step as f32 / SAMPLE_RATE * 1_000.0 * std::f32::consts::TAU).sin()) * 0.5;
            energy_flat += flat.process(input, SAMPLE_RATE).powi(2);
            energy_boosted += boosted.process(input, SAMPLE_RATE).powi(2);
        }
        assert!(
            energy_boosted > energy_flat,
            "boosted band should be louder: {} vs {}",
            energy_boosted,
            energy_flat
        );
    }

    #[test]
    fn allpass_changes_phase_not_magnitude() {
        let mut allpass = Allpass::default();
        allpass.set_radius(1.2);
        allpass.set_frequency(1_000.0, SAMPLE_RATE);

        let mut peak_in = 0.0f32;
        let mut peak_out = 0.0f32;
        for step in 0..SAMPLE_RATE as usize {
            let input = ((step as f32 / SAMPLE_RATE * 1_000.0 * std::f32::consts::TAU).sin()) * 0.5;
            peak_in = peak_in.max(input.abs());
            peak_out = peak_out.max(allpass.process(input).abs());
        }
        assert!(
            (peak_in - peak_out).abs() < 0.15,
            "allpass must keep magnitude, in {peak_in} out {peak_out}"
        );
    }

    #[test]
    fn dc_blocker_removes_offset() {
        let mut blocker = DcBlocker::default();
        let mut tail = 0.0;
        for _ in 0..48_000 {
            tail = blocker.process(1.0);
        }
        assert!(tail.abs() < 0.01, "offset should vanish, got {tail}");
    }
}
