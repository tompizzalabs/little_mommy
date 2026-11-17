pub mod analog_tables;
pub mod character;
pub mod kernels;
pub mod oscillator;

pub use character::OscillatorModel;
pub use oscillator::{Oscillator, SyncIn, SyncOut};

pub const SQUARE_VOLUME_SCALAR: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AnalogShape {
    Saw,
    ReverseSaw,
    Square,
    Triangle,
    Sharktooth,
    Sine,
    Morph,
}

impl AnalogShape {
    pub const ALL: [AnalogShape; 7] = [
        AnalogShape::Saw,
        AnalogShape::ReverseSaw,
        AnalogShape::Square,
        AnalogShape::Triangle,
        AnalogShape::Sharktooth,
        AnalogShape::Sine,
        AnalogShape::Morph,
    ];
}

impl std::fmt::Display for AnalogShape {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            AnalogShape::Saw => "Saw",
            AnalogShape::ReverseSaw => "Rev Saw",
            AnalogShape::Square => "Square",
            AnalogShape::Triangle => "Triangle",
            AnalogShape::Sharktooth => "Shark",
            AnalogShape::Sine => "Sine",
            AnalogShape::Morph => "Morph",
        };
        write!(formatter, "{}", name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyncMode {
    Off,
    Hard,
    Soft,
}

impl SyncMode {
    pub const ALL: [SyncMode; 3] = [SyncMode::Off, SyncMode::Hard, SyncMode::Soft];
}

impl std::fmt::Display for SyncMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            SyncMode::Off => "Off",
            SyncMode::Hard => "Hard",
            SyncMode::Soft => "Soft",
        };
        write!(formatter, "{}", name)
    }
}

#[derive(Debug, Clone)]
pub struct AnalogVoiceState {
    oscillator: Oscillator,
}

impl Default for AnalogVoiceState {
    fn default() -> Self {
        Self {
            oscillator: Oscillator::new(0x1234_5678),
        }
    }
}

impl AnalogVoiceState {
    pub fn reseed(&mut self, seed: u32) {
        self.oscillator.reseed(seed);
    }

    pub fn phase(&self) -> f32 {
        self.oscillator.phase()
    }

    pub fn set_phase(&mut self, phase: f32) {
        self.oscillator.set_phase(phase);
    }

    pub fn sync_out(&self) -> SyncOut {
        self.oscillator.sync_out()
    }

    pub fn soft_sync_threshold(&self) -> f32 {
        self.oscillator.soft_sync_threshold()
    }

    pub fn reset(&mut self) {
        self.oscillator.reset();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn next(
        &mut self,
        shape: AnalogShape,
        freq: f32,
        sample_rate: f32,
        pulse_width: f32,
        drift: f32,
        morph: f32,
        sync: Option<SyncIn>,
    ) -> f32 {
        self.oscillator
            .process(shape, freq, pulse_width, drift, morph, sample_rate, sync)
    }
}
