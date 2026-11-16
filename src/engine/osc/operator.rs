use super::analog::analog_tables::{self, TABLE_LEN};
use super::drift::{DRIFT_RANGE_SEMITONES, DriftGenerator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OperatorWave {
    Saw,
    Triangle,
    Sine,
}

impl OperatorWave {
    pub const ALL: [OperatorWave; 3] = [
        OperatorWave::Saw,
        OperatorWave::Triangle,
        OperatorWave::Sine,
    ];
}

impl std::fmt::Display for OperatorWave {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            OperatorWave::Saw => "Saw",
            OperatorWave::Triangle => "Triangle",
            OperatorWave::Sine => "Sine",
        };
        write!(formatter, "{}", name)
    }
}

#[inline(always)]
fn table_for(wave: OperatorWave, freq: f32) -> &'static [f32] {
    let tables = analog_tables::get();
    let mip_index = analog_tables::table_index(freq);

    match wave {
        OperatorWave::Saw => &tables.saw[mip_index],
        OperatorWave::Triangle => &tables.triangle[mip_index],
        OperatorWave::Sine => &tables.sine,
    }
}

#[inline(always)]
fn read(table: &[f32], phase: f32) -> f32 {
    let position = phase * TABLE_LEN as f32;
    let first_index = position as usize & (TABLE_LEN - 1);
    let next_index = (first_index + 1) & (TABLE_LEN - 1);
    let fraction = position - position.floor();
    table[first_index] + (table[next_index] - table[first_index]) * fraction
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

const OPERATOR_SEED_MIX: u32 = 0x9E37_79B9;
const OPERATOR_PHASE_SPREAD: u32 = 991;

impl OperatorVoice {
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

    pub fn next(
        &mut self,
        wave: OperatorWave,
        freq: f32,
        sample_rate: f32,
        drift_amount: f32,
    ) -> f32 {
        let (drifted, phase) = self.advance(freq, sample_rate, drift_amount);
        read(table_for(wave, drifted), phase)
    }

    pub fn next_mod(
        &mut self,
        wave: OperatorWave,
        freq: f32,
        sample_rate: f32,
        drift_amount: f32,
        phase_mod: f32,
    ) -> f32 {
        let (drifted, phase) = self.advance(freq, sample_rate, drift_amount);
        read(
            table_for(wave, drifted),
            (phase + phase_mod).rem_euclid(1.0),
        )
    }

    pub fn seed(&mut self, seed: u32) {
        self.phase = (seed.wrapping_mul(OPERATOR_SEED_MIX) % OPERATOR_PHASE_SPREAD) as f32
            / OPERATOR_PHASE_SPREAD as f32;
        self.drift = DriftGenerator::new(seed);
    }
}
