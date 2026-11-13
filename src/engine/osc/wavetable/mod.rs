use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock, RwLockReadGuard};

use crate::engine::osc::aa::AntiAlias;

#[derive(Debug, Clone)]
pub struct WtVoice {
    pub phase: f32,
    pub aa: AntiAlias,
}

impl Default for WtVoice {
    fn default() -> Self {
        Self {
            phase: 0.0,
            aa: AntiAlias::default(),
        }
    }
}

impl WtVoice {
    /// Sample one output step, 3x oversampled, then advance the phase by
    /// `freq / sample_rate` cycles.
    pub fn next(
        &mut self,
        source: &TableSource,
        position: f32,
        freq: f32,
        sample_rate: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }
        let oversampled_rate = sample_rate * 3.0;
        let mut up = [0.0f32; 3];
        for slot in up.iter_mut() {
            *slot = source.sample(self.phase, position);
            self.phase += freq / oversampled_rate;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
        self.aa.process(up)
    }

    /// Same as [`WtVoice::next`] but reads the table at `phase + phase_mod`
    /// (phase modulation, as in Odin's `PMCarrierOsc`). The underlying phase is
    /// still advanced by the carrier frequency only.
    pub fn next_mod(
        &mut self,
        source: &TableSource,
        position: f32,
        freq: f32,
        sample_rate: f32,
        phase_mod: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }
        let oversampled_rate = sample_rate * 3.0;
        let mut up = [0.0f32; 3];
        for slot in up.iter_mut() {
            *slot = source.sample((self.phase + phase_mod).rem_euclid(1.0), position);
            self.phase += freq / oversampled_rate;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
        self.aa.process(up)
    }

    /// Sample a table at an explicit phase without advancing the voice.
    pub fn sample_phase(&self, source: &TableSource, position: f32, phase: f32) -> f32 {
        source.sample(phase.rem_euclid(1.0), position)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StockTable {
    Morph,
    Harmonics,
    Fold,
}

impl StockTable {
    pub const ALL: [StockTable; 3] = [StockTable::Morph, StockTable::Harmonics, StockTable::Fold];
}

impl std::fmt::Display for StockTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            StockTable::Morph => "Morph",
            StockTable::Harmonics => "Harmonics",
            StockTable::Fold => "Fold",
        };
        write!(f, "{}", name)
    }
}

pub fn sample_stock(table: StockTable, phase: f32, position: f32) -> f32 {
    let position = position.clamp(0.0, 1.0);

    match table {
        StockTable::Morph => {
            let sine = (phase * std::f32::consts::TAU).sin();
            let saw = phase * 2.0 - 1.0;
            let square = if phase < 0.5 { 1.0 } else { -1.0 };

            if position < 0.5 {
                let t = position * 2.0;
                sine * (1.0 - t) + saw * t
            } else {
                let t = (position - 0.5) * 2.0;
                saw * (1.0 - t) + square * t
            }
        }
        StockTable::Harmonics => {
            let count = 1 + (position * 11.0).round() as i32;
            let mut sum = 0.0;
            let mut norm = 0.0;
            for k in 1..=count {
                let kf = k as f32;
                sum += (phase * kf * std::f32::consts::TAU).sin() / kf;
                norm += 1.0 / kf;
            }
            if norm > 0.0 { sum / norm } else { 0.0 }
        }
        StockTable::Fold => {
            let amp = 1.0 + position * 7.0;
            fold((phase * std::f32::consts::TAU).sin() * amp)
        }
    }
}

fn fold(x: f32) -> f32 {
    let mut x = (x + 1.0).rem_euclid(4.0) - 1.0;
    if x > 1.0 {
        x = 2.0 - x;
    }
    x
}

/// Patch-stored table selection: a stock table, or a custom table by name.
#[derive(Debug, Clone, PartialEq)]
pub enum TableSel {
    Stock(StockTable),
    Custom(String),
}

/// Audio-side resolved source, fixed at compile time.
#[derive(Debug, Clone)]
pub enum TableSource {
    Stock(StockTable),
    Custom(Arc<CustomTable>),
}

impl TableSource {
    pub fn sample(&self, phase: f32, position: f32) -> f32 {
        match self {
            TableSource::Stock(table) => sample_stock(*table, phase, position),
            TableSource::Custom(table) => table.sample(phase, position),
        }
    }
}

pub fn resolve(sel: &TableSel, library: &CustomLibrary) -> TableSource {
    match sel {
        TableSel::Stock(table) => TableSource::Stock(*table),
        TableSel::Custom(name) => match library.find(name) {
            Some(table) => TableSource::Custom(table),
            None => TableSource::Stock(StockTable::Morph),
        },
    }
}

/// Default selection for new Wave nodes.
pub fn default_sel() -> TableSel {
    TableSel::Stock(StockTable::Morph)
}

// ---------------------------------------------------------------------------------------------------
// Custom table library (format TBD).
// ---------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CustomTable {
    pub name: String,
    pub frame_size: usize,
    pub data: Vec<f32>,
}

impl CustomTable {
    pub fn frame_count(&self) -> usize {
        self.data.len().checked_div(self.frame_size).unwrap_or(0)
    }

    pub fn sample(&self, phase: f32, position: f32) -> f32 {
        let frames = self.frame_count();
        if frames == 0 || self.frame_size == 0 {
            return 0.0;
        }

        let size = self.frame_size;
        let pos = position.clamp(0.0, 1.0) * (frames - 1) as f32;
        let f0 = pos as usize;
        let f1 = (f0 + 1).min(frames - 1);
        let ft = pos - f0 as f32;

        let p = (phase - phase.floor()) * size as f32;
        let i0 = p as usize % size;
        let i1 = (i0 + 1) % size;
        let pt = p - p.floor();

        let a = lerp(self.data[f0 * size + i0], self.data[f0 * size + i1], pt);
        let b = lerp(self.data[f1 * size + i0], self.data[f1 * size + i1], pt);
        lerp(a, b, ft)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[derive(Debug, Clone, Default)]
pub struct CustomLibrary {
    pub tables: Vec<Arc<CustomTable>>,
}

impl CustomLibrary {
    pub fn find(&self, name: &str) -> Option<Arc<CustomTable>> {
        self.tables.iter().find(|t| t.name == name).cloned()
    }
}

static LIBRARY: OnceLock<RwLock<CustomLibrary>> = OnceLock::new();
static LIBRARY_VERSION: AtomicU64 = AtomicU64::new(0);

pub fn library() -> &'static RwLock<CustomLibrary> {
    LIBRARY.get_or_init(|| RwLock::new(CustomLibrary::default()))
}

pub fn read_guard() -> RwLockReadGuard<'static, CustomLibrary> {
    library().read().unwrap_or_else(|e| e.into_inner())
}

pub fn library_version() -> u64 {
    LIBRARY_VERSION.load(Ordering::Relaxed)
}

pub fn rescan() {
    // Placeholder for future custom format loading.
    LIBRARY_VERSION.fetch_add(1, Ordering::Relaxed);
}
