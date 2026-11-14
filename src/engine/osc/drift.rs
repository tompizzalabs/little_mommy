//! Global oscillator drift generator.
//!
//! Shared by every oscillator type (analog, FM, PM): a random-walk drift in
//! semitones, re-rolled every few hundred milliseconds and smoothed. Seeded
//! per oscillator voice so variation is deterministic and voice-to-voice
//! independent.

use fastrand::Rng;

/// Full-scale drift depth at `drift = 1.0`, in semitones.
pub const DRIFT_RANGE_SEMITONES: f32 = 0.2;

#[derive(Debug, Clone)]
pub struct DriftGenerator {
    rng: Rng,
    target: f32,
    current: f32,
    counter: u32,
    interval: u32,
}

impl Default for DriftGenerator {
    fn default() -> Self {
        Self::new(0x1234_5678)
    }
}

impl DriftGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            rng: Rng::with_seed(seed as u64),
            target: 0.0,
            current: 0.0,
            counter: 0,
            interval: 0,
        }
    }

    fn rand_unit(&mut self) -> f32 {
        self.rng.f32()
    }

    /// Per-sample drift value in [-1, 1].
    pub fn next(&mut self, sample_rate: f32) -> f32 {
        self.counter += 1;
        if self.counter >= self.interval {
            self.counter = 0;
            let seconds = 0.3 + self.rand_unit() * 0.7;
            self.interval = (sample_rate * seconds).max(1.0) as u32;
            self.target = self.rand_unit() * 2.0 - 1.0;
        }

        let glide = 1.0 - (-1.0 / (sample_rate * 0.05).max(1.0)).exp();
        self.current += (self.target - self.current) * glide;
        self.current
    }
}
