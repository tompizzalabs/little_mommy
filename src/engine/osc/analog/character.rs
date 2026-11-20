use fastrand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OscillatorModel {
    Discrete,
    Precision,
}

impl OscillatorModel {
    pub const ALL: [OscillatorModel; 2] = [OscillatorModel::Discrete, OscillatorModel::Precision];
}

impl std::fmt::Display for OscillatorModel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            OscillatorModel::Discrete => "Discrete",
            OscillatorModel::Precision => "Precision",
        };
        write!(formatter, "{}", name)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ModelSpec {
    pub reset_fraction: f32,
    pub ramp_curvature: f32,
    pub shark_split: f32,
    pub shark_power: u8,
    pub duty_pitch_coupling: f32,
    pub drift_cents: f32,
    pub thermal_cents: f32,
    pub period_jitter: f32,
    pub tolerance_scale: f32,
    pub stage_cutoff: f32,
    pub stage_resonance: f32,
    pub soft_sync_threshold: f32,
}

pub const DISCRETE_SPEC: ModelSpec = ModelSpec {
    reset_fraction: 0.006,
    ramp_curvature: 0.16,
    shark_split: 0.78,
    shark_power: 2,
    duty_pitch_coupling: 0.030,
    drift_cents: 6.0,
    thermal_cents: 3.5,
    period_jitter: 0.0016,
    tolerance_scale: 1.0,
    stage_cutoff: 13_000.0,
    stage_resonance: 0.95,
    soft_sync_threshold: 0.45,
};

pub const PRECISION_SPEC: ModelSpec = ModelSpec {
    reset_fraction: 0.0,
    ramp_curvature: 0.0,
    shark_split: 0.72,
    shark_power: 3,
    duty_pitch_coupling: 0.0,
    drift_cents: 1.2,
    thermal_cents: 0.8,
    period_jitter: 0.0002,
    tolerance_scale: 0.25,
    stage_cutoff: 21_000.0,
    stage_resonance: 0.5,
    soft_sync_threshold: 0.5,
};

pub fn model_spec(model: OscillatorModel) -> &'static ModelSpec {
    match model {
        OscillatorModel::Discrete => &DISCRETE_SPEC,
        OscillatorModel::Precision => &PRECISION_SPEC,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tolerances {
    pub tune_cents: f32,
    pub duty: f32,
    pub curvature: f32,
    pub triangle_peak: f32,
    pub reset: f32,
    pub gain: f32,
    pub drift_depth: f32,
    pub thermal_depth: f32,
}

impl Tolerances {
    pub fn draw(rng: &mut Rng, scale: f32) -> Self {
        let bipolar = |rng: &mut Rng| rng.f32() * 2.0 - 1.0;
        Self {
            tune_cents: bipolar(rng) * 8.0 * scale,
            duty: bipolar(rng) * 0.020 * scale,
            curvature: bipolar(rng) * 0.12 * scale,
            triangle_peak: bipolar(rng) * 0.030 * scale,
            reset: bipolar(rng) * 0.5 * scale,
            gain: bipolar(rng) * 0.06 * scale,
            drift_depth: 0.6 + rng.f32() * 0.8,
            thermal_depth: 0.5 + rng.f32() * 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerances_stay_within_design_bounds() {
        let mut rng = Rng::with_seed(7);
        for _ in 0..100 {
            let tolerances = Tolerances::draw(&mut rng, 1.0);
            assert!(tolerances.tune_cents.abs() <= 8.0);
            assert!(tolerances.duty.abs() <= 0.02);
            assert!(tolerances.curvature.abs() <= 0.12);
            assert!(tolerances.triangle_peak.abs() <= 0.03);
            assert!(tolerances.reset.abs() <= 0.5);
            assert!(tolerances.gain.abs() <= 0.06);
            assert!((0.6..=1.4).contains(&tolerances.drift_depth));
            assert!((0.5..=1.5).contains(&tolerances.thermal_depth));
        }
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn precision_model_is_tighter_than_discrete() {
        assert!(PRECISION_SPEC.drift_cents < DISCRETE_SPEC.drift_cents);
        assert!(PRECISION_SPEC.period_jitter < DISCRETE_SPEC.period_jitter);
        assert!(PRECISION_SPEC.tolerance_scale < DISCRETE_SPEC.tolerance_scale);
    }
}
