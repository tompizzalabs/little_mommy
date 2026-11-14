//! Shared operator voice for FM and PM oscillators.
//!
//! Operators are simple bandlimited oscillators reading the analog module's
//! mip-mapped tables (saw / triangle / sine), each with its own drift
//! generator so both oscillators wander independently.

use super::analog::analog_tables::{self, TABLE_LEN};
use super::drift::{DRIFT_RANGE_SEMITONES, DriftGenerator};

/// Waveform for an FM/PM operator. Only the table-backed shapes are
/// available; pulse-family shapes belong to the analog oscillator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OperatorWave {
    Saw,
    Triangle,
    Sine,
}

impl OperatorWave {
    pub const ALL: [OperatorWave; 3] = [OperatorWave::Saw, OperatorWave::Triangle, OperatorWave::Sine];
}

impl std::fmt::Display for OperatorWave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            OperatorWave::Saw => "Saw",
            OperatorWave::Triangle => "Triangle",
            OperatorWave::Sine => "Sine",
        };
        write!(f, "{}", name)
    }
}

#[inline(always)]
fn table_for(wave: OperatorWave, freq: f32) -> &'static [f32] {
    let tables = analog_tables::get();
    let idx = analog_tables::table_index(freq);

    match wave {
        OperatorWave::Saw => &tables.saw[idx],
        OperatorWave::Triangle => &tables.triangle[idx],
        OperatorWave::Sine => &tables.sine,
    }
}

#[inline(always)]
fn read(table: &[f32], phase: f32) -> f32 {
    let pos = phase * TABLE_LEN as f32;
    let i0 = pos as usize & (TABLE_LEN - 1);
    let i1 = (i0 + 1) & (TABLE_LEN - 1);
    let frac = pos - pos.floor();
    table[i0] + (table[i1] - table[i0]) * frac
}

#[derive(Debug, Clone)]
pub struct OperatorVoice {
    pub phase: f32,
    pub drift: DriftGenerator,
}

impl Default for OperatorVoice {
    fn default() -> Self {
        Self {
            phase: 0.0,
            drift: DriftGenerator::default(),
        }
    }
}

impl OperatorVoice {
    /// Apply the drift layer and advance the phase. Returns nothing; the
    /// caller reads the table. Keeps drift handling identical across FM/PM.
    #[inline(always)]
    fn advance(&mut self, freq: f32, sample_rate: f32, drift_amount: f32) -> (f32, f32) {
        if sample_rate <= 0.0 {
            return (freq, self.phase);
        }

        let drift_semitones = self.drift.next(sample_rate) * drift_amount * DRIFT_RANGE_SEMITONES;
        let drifted = freq * 2.0f32.powf(drift_semitones / 12.0);

        self.phase += drifted / sample_rate;
        self.phase -= self.phase.floor();

        (drifted, self.phase)
    }

    /// Sample one output step, drifting by `drift_amount` around `freq`.
    pub fn next(&mut self, wave: OperatorWave, freq: f32, sample_rate: f32, drift_amount: f32) -> f32 {
        let (drifted, phase) = self.advance(freq, sample_rate, drift_amount);
        read(table_for(wave, drifted), phase)
    }

    /// Same as [`OperatorVoice::next`] but reads the table at
    /// `phase + phase_mod` (phase modulation). The underlying phase is still
    /// advanced by the carrier frequency only.
    pub fn next_mod(
        &mut self,
        wave: OperatorWave,
        freq: f32,
        sample_rate: f32,
        drift_amount: f32,
        phase_mod: f32,
    ) -> f32 {
        let (drifted, phase) = self.advance(freq, sample_rate, drift_amount);
        read(table_for(wave, drifted), (phase + phase_mod).rem_euclid(1.0))
    }

    /// Re-seed per-voice phase and drift (deterministic, per voice/node).
    pub fn seed(&mut self, seed: u32) {
        self.phase = (seed.wrapping_mul(0x9E37_79B9) % 991) as f32 / 991.0;
        self.drift = DriftGenerator::new(seed);
    }
}
