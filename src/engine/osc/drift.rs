use fastrand::Rng;

pub const DRIFT_RANGE_SEMITONES: f32 = 0.2;
pub const DRIFT_DECIMATION: u32 = 8;

const WANDER_RETARGET_SECONDS: f32 = 0.3;
const WANDER_RETARGET_SPREAD: f32 = 0.7;
const WANDER_GLIDE_SECONDS: f32 = 0.05;
const THERMAL_RETARGET_SECONDS: f32 = 8.0;
const THERMAL_RETARGET_SPREAD: f32 = 14.0;
const THERMAL_GLIDE_SECONDS: f32 = 6.0;

#[derive(Debug, Clone)]
pub struct DriftGenerator {
    rng: Rng,
    sample_rate: f32,
    wander_target: f32,
    wander_value: f32,
    wander_counter: u32,
    wander_interval: u32,
    wander_glide: f32,
    thermal_target: f32,
    thermal_value: f32,
    thermal_counter: u32,
    thermal_interval: u32,
    thermal_glide: f32,
}

impl Default for DriftGenerator {
    fn default() -> Self {
        Self::new(0x1234_5678)
    }
}

impl DriftGenerator {
    pub fn new(seed: u32) -> Self {
        let default_control_period = DRIFT_DECIMATION as f32 / 48_000.0;
        Self {
            rng: Rng::with_seed(seed as u64),
            sample_rate: 48_000.0,
            wander_target: 0.0,
            wander_value: 0.0,
            wander_counter: 0,
            wander_interval: 1,
            wander_glide: 1.0 - (-default_control_period / WANDER_GLIDE_SECONDS).exp(),
            thermal_target: 0.0,
            thermal_value: 0.0,
            thermal_counter: 0,
            thermal_interval: 1,
            thermal_glide: 1.0 - (-default_control_period / THERMAL_GLIDE_SECONDS).exp(),
        }
    }

    pub fn next(&mut self, sample_rate: f32) -> (f32, f32) {
        if sample_rate <= 1_000.0 {
            return (0.0, 0.0);
        }
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
            let control_period = DRIFT_DECIMATION as f32 / sample_rate;
            self.wander_glide = 1.0 - (-control_period / WANDER_GLIDE_SECONDS).exp();
            self.thermal_glide = 1.0 - (-control_period / THERMAL_GLIDE_SECONDS).exp();
            self.wander_counter = 0;
            self.thermal_counter = 0;
        }

        if self.wander_counter == 0 {
            let seconds = WANDER_RETARGET_SECONDS + self.rng.f32() * WANDER_RETARGET_SPREAD;
            self.wander_interval =
                (self.sample_rate * seconds / DRIFT_DECIMATION as f32).max(1.0) as u32;
            self.wander_target = self.rng.f32() * 2.0 - 1.0;
        }
        self.wander_counter += 1;
        if self.wander_counter >= self.wander_interval {
            self.wander_counter = 0;
        }
        self.wander_value += (self.wander_target - self.wander_value) * self.wander_glide;

        if self.thermal_counter == 0 {
            let seconds = THERMAL_RETARGET_SECONDS + self.rng.f32() * THERMAL_RETARGET_SPREAD;
            self.thermal_interval =
                (self.sample_rate * seconds / DRIFT_DECIMATION as f32).max(1.0) as u32;
            self.thermal_target = self.rng.f32() * 2.0 - 1.0;
        }
        self.thermal_counter += 1;
        if self.thermal_counter >= self.thermal_interval {
            self.thermal_counter = 0;
        }
        self.thermal_value += (self.thermal_target - self.thermal_value) * self.thermal_glide;

        (self.wander_value, self.thermal_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_scales_stay_bipolar_and_finite() {
        let mut drift = DriftGenerator::new(3);
        for _ in 0..6_000 {
            let (wander, thermal) = drift.next(48_000.0);
            assert!(wander.is_finite() && thermal.is_finite());
            assert!(wander.abs() <= 1.0 + 1e-6);
            assert!(thermal.abs() <= 1.0 + 1e-6);
        }
    }

    #[test]
    fn wander_moves_faster_than_thermal() {
        let mut drift = DriftGenerator::new(5);
        let mut wander_movement = 0.0f32;
        let mut thermal_movement = 0.0f32;
        let mut previous_wander = 0.0f32;
        let mut previous_thermal = 0.0f32;
        for _ in 0..6_000 {
            let (wander, thermal) = drift.next(48_000.0);
            wander_movement += (wander - previous_wander).abs();
            thermal_movement += (thermal - previous_thermal).abs();
            previous_wander = wander;
            previous_thermal = thermal;
        }
        assert!(
            wander_movement > thermal_movement * 2.0,
            "wander should move noticeably more than thermal: {wander_movement} vs {thermal_movement}"
        );
    }

    #[test]
    fn invalid_sample_rate_is_safe() {
        let mut drift = DriftGenerator::new(2);
        let (wander, thermal) = drift.next(0.0);
        assert_eq!(wander, 0.0);
        assert_eq!(thermal, 0.0);
    }
}
