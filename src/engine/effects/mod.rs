pub mod bitcrusher;
pub mod chorus;
pub mod delay;
pub mod distortion;
pub mod flanger;
pub mod parametric_eq;
pub mod phaser;
pub mod ring_modulator;
pub mod zita_reverb;

pub use bitcrusher::BitcrusherState;
pub use chorus::ChorusState;
pub use delay::DelayState;
pub use distortion::{DistortionAlgorithm, DistortionState};
pub use flanger::FlangerState;
pub use parametric_eq::ParametricEqState;
pub use phaser::PhaserState;
pub use ring_modulator::RingModState;
pub use zita_reverb::ZitaReverbState;

pub const MAX_SAMPLE_RATE: f32 = 192_000.0;
pub const MAX_DELAY_TIME_SECONDS: f32 = 4.0;
