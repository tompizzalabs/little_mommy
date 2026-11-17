use std::sync::OnceLock;

pub const TABLE_LEN: usize = 2048;
pub const TABLES_PER_OCTAVE: usize = 3;
pub const NUM_TABLES: usize = 33;
pub const BASE_FREQ: f32 = 8.176;
const HARMONIC_CEILING: f32 = 20_000.0;

pub struct AnalogTables {
    pub saw: Vec<Vec<f32>>,
    pub triangle: Vec<Vec<f32>>,
    pub sine: Vec<f32>,
}

static TABLES: OnceLock<AnalogTables> = OnceLock::new();

pub fn get() -> &'static AnalogTables {
    TABLES.get_or_init(build)
}

pub fn mid_freq(index: usize) -> f32 {
    BASE_FREQ * 2.0f32.powf(index as f32 / TABLES_PER_OCTAVE as f32)
}

pub fn table_index(frequency: f32) -> usize {
    let raw = TABLES_PER_OCTAVE as f32 * (frequency / BASE_FREQ).log2();
    (raw.ceil() as isize).clamp(0, (NUM_TABLES - 1) as isize) as usize
}

pub fn band_pos(frequency: f32) -> (usize, f32) {
    let scaled = frequency.max(BASE_FREQ) / BASE_FREQ;
    let raw = TABLES_PER_OCTAVE as f32 * scaled.log2();
    let band = ((raw.floor() as isize).clamp(0, (NUM_TABLES - 2) as isize)) as usize;
    let mix = (raw - band as f32).clamp(0.0, 1.0);
    (band, mix)
}

pub fn read_sine(position: f32) -> f32 {
    read_table(&get().sine, position)
}

pub fn read_band_pair(tables: &[Vec<f32>], band: usize, band_mix: f32, position: f32) -> f32 {
    let lower = read_table(&tables[band], position);
    let upper = read_table(&tables[band + 1], position);
    lower + (upper - lower) * band_mix
}

fn read_table(table: &[f32], position: f32) -> f32 {
    let first_index = (position as usize) & (TABLE_LEN - 1);
    let next_index = (first_index + 1) & (TABLE_LEN - 1);
    let fraction = position - position.floor();
    table[first_index] + (table[next_index] - table[first_index]) * fraction
}

fn build() -> AnalogTables {
    let mut saw = Vec::with_capacity(NUM_TABLES);
    let mut triangle = Vec::with_capacity(NUM_TABLES);

    for table_number in 0..NUM_TABLES {
        let mid_frequency = mid_freq(table_number);
        let max_harmonic = (HARMONIC_CEILING / mid_frequency).floor() as usize;
        let max_harmonic = max_harmonic.clamp(1, TABLE_LEN / 2);
        saw.push(build_saw(max_harmonic));
        triangle.push(build_triangle(max_harmonic));
    }

    let sine = (0..TABLE_LEN)
        .map(|sample_index| (sample_index as f32 / TABLE_LEN as f32 * std::f32::consts::TAU).sin())
        .collect();

    AnalogTables {
        saw,
        triangle,
        sine,
    }
}

fn build_saw(max_harmonic: usize) -> Vec<f32> {
    let mut table = vec![0.0f32; TABLE_LEN];
    for harmonic in 1..=max_harmonic {
        let amplitude = 1.0 / harmonic as f32;
        let angular_step = harmonic as f32 * std::f32::consts::TAU / TABLE_LEN as f32;
        for (sample_index, slot) in table.iter_mut().enumerate() {
            *slot += amplitude * (angular_step * sample_index as f32).sin();
        }
    }
    normalize(&mut table);
    table
}

fn build_triangle(max_harmonic: usize) -> Vec<f32> {
    let mut table = vec![0.0f32; TABLE_LEN];
    let mut harmonic = 1usize;
    let mut sign = 1.0f32;
    while harmonic <= max_harmonic {
        let amplitude = sign / (harmonic * harmonic) as f32;
        let angular_step = harmonic as f32 * std::f32::consts::TAU / TABLE_LEN as f32;
        for (sample_index, slot) in table.iter_mut().enumerate() {
            *slot += amplitude * (angular_step * sample_index as f32).sin();
        }
        sign = -sign;
        harmonic += 2;
    }
    normalize(&mut table);
    table
}

fn normalize(table: &mut [f32]) {
    let peak = table
        .iter()
        .fold(0.0f32, |running_peak, &value| running_peak.max(value.abs()));
    if peak > 1e-6 {
        for value in table.iter_mut() {
            *value /= peak;
        }
    }
}
