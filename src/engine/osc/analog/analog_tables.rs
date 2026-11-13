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

pub fn table_index(f: f32) -> usize {
    let raw = TABLES_PER_OCTAVE as f32 * (f / BASE_FREQ).log2();
    (raw.ceil() as isize).clamp(0, (NUM_TABLES - 1) as isize) as usize
}

fn build() -> AnalogTables {
    let mut saw = Vec::with_capacity(NUM_TABLES);
    let mut triangle = Vec::with_capacity(NUM_TABLES);

    for i in 0..NUM_TABLES {
        let f_mid = mid_freq(i);
        let k_max = (HARMONIC_CEILING / f_mid).floor() as usize;
        let k_max = k_max.clamp(1, TABLE_LEN / 2);
        saw.push(build_saw(k_max));
        triangle.push(build_triangle(k_max));
    }

    let sine = (0..TABLE_LEN)
        .map(|p| (p as f32 / TABLE_LEN as f32 * std::f32::consts::TAU).sin())
        .collect();

    AnalogTables {
        saw,
        triangle,
        sine,
    }
}

fn build_saw(k_max: usize) -> Vec<f32> {
    let mut table = vec![0.0f32; TABLE_LEN];
    for k in 1..=k_max {
        let amp = 1.0 / k as f32;
        let w = k as f32 * std::f32::consts::TAU / TABLE_LEN as f32;
        for (p, slot) in table.iter_mut().enumerate() {
            *slot += amp * (w * p as f32).sin();
        }
    }
    normalize(&mut table);
    table
}

fn build_triangle(k_max: usize) -> Vec<f32> {
    let mut table = vec![0.0f32; TABLE_LEN];
    let mut k = 1usize;
    let mut sign = 1.0f32;
    while k <= k_max {
        let amp = sign / (k * k) as f32;
        let w = k as f32 * std::f32::consts::TAU / TABLE_LEN as f32;
        for (p, slot) in table.iter_mut().enumerate() {
            *slot += amp * (w * p as f32).sin();
        }
        sign = -sign;
        k += 2;
    }
    normalize(&mut table);
    table
}

fn normalize(table: &mut [f32]) {
    let peak = table.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
    if peak > 1e-6 {
        for v in table.iter_mut() {
            *v /= peak;
        }
    }
}
