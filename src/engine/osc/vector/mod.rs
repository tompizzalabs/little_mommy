use super::analog::analog_tables::{self, TABLE_LEN};
use super::drift::{DRIFT_RANGE_SEMITONES, DriftGenerator};

const VECTOR_SMOOTH: f32 = 0.001;

#[derive(Debug, Clone)]
pub struct VectorState {
    pub phase: f32,
    pub drift: DriftGenerator,
    pub x_smooth: f32,
    pub y_smooth: f32,
}

impl Default for VectorState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            drift: DriftGenerator::default(),
            x_smooth: 0.5,
            y_smooth: 0.5,
        }
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

const VECTOR_SEED_MIX: u32 = 0x9E37_79B9;
const VECTOR_PHASE_SPREAD: u32 = 983;

impl VectorState {
    pub fn snap_xy(&mut self, x: f32, y: f32) {
        self.x_smooth = x;
        self.y_smooth = y;
    }

    pub fn next(&mut self, freq: f32, sample_rate: f32, x: f32, y: f32, drift_amount: f32) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        self.x_smooth += (x - self.x_smooth) * VECTOR_SMOOTH;
        self.y_smooth += (y - self.y_smooth) * VECTOR_SMOOTH;
        let morph_x = self.x_smooth.clamp(0.0, 1.0);
        let morph_y = self.y_smooth.clamp(0.0, 1.0);

        let (wander, _) = self.drift.next(sample_rate);
        let drift_semitones = wander * drift_amount * DRIFT_RANGE_SEMITONES;
        let drifted = freq * 2.0f32.powf(drift_semitones / 12.0);
        self.phase += drifted / sample_rate;
        self.phase -= self.phase.floor();

        let tables = analog_tables::get();
        let mip_index = analog_tables::table_index(drifted.max(1.0));
        let saw = &tables.saw[mip_index];

        let bottom_left_wave = read(saw, self.phase);
        let top_left_wave = read(&tables.triangle[mip_index], self.phase);
        let top_right_wave = read(&tables.sine, self.phase);
        let bottom_right_wave = -bottom_left_wave;

        let bottom_row = (1.0 - morph_x) * bottom_left_wave + morph_x * bottom_right_wave;
        let top_row = (1.0 - morph_x) * top_left_wave + morph_x * top_right_wave;
        (1.0 - morph_y) * bottom_row + morph_y * top_row
    }

    pub fn seed(&mut self, seed: u32) {
        self.phase = (seed.wrapping_mul(VECTOR_SEED_MIX) % VECTOR_PHASE_SPREAD) as f32
            / VECTOR_PHASE_SPREAD as f32;
        self.drift = DriftGenerator::new(seed);
        self.snap_xy(0.5, 0.5);
    }
}
