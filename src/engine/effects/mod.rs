pub mod bitcrusher;
pub mod chorus;
pub mod delay;
pub mod distortion;
pub mod flanger;
pub mod parametric_eq;
pub mod phaser;
pub mod ring_modulator;
pub mod zita_reverb;

pub use bitcrusher::{Bitcrusher, DistortionShape};
pub use chorus::Chorus;
pub use delay::Delay;
pub use distortion::Distortion;
pub use flanger::Flanger;
pub use parametric_eq::ParametricEq;
pub use phaser::Phaser;
pub use ring_modulator::RingModulator;
pub use zita_reverb::ZitaReverb;

const MAX_SAMPLE_RATE: f32 = 192_000.0;
