pub mod biquad;
pub mod comb;
pub mod diode;
pub mod formant;
pub mod korg35;
pub mod ladder;
pub mod one_pole;
pub mod sem;
pub mod state_variable;

pub use biquad::{Allpass, DcBlocker, PeakingEq, Resonator};
pub use comb::CombFilter;
pub use diode::DiodeFilter;
pub use formant::{FormantFilter, Vowel};
pub use korg35::Korg35Filter;
pub use ladder::{LadderFilter, LadderMode};
pub use one_pole::VaOnePole;
pub use sem::SemFilter;
pub use state_variable::StateVariableFilter;

pub const FREQUENCY_MIN: f32 = 20.0;
pub const FREQUENCY_MAX: f32 = 20000.0;

pub fn clamp_frequency(frequency: f32) -> f32 {
    frequency.clamp(FREQUENCY_MIN, FREQUENCY_MAX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FilterKind {
    Ladder4LowPass,
    Ladder2LowPass,
    Ladder4BandPass,
    Ladder2BandPass,
    Ladder4HighPass,
    Ladder2HighPass,
    Sem,
    Korg35LowPass,
    Korg35HighPass,
    Diode,
    Formant,
    Comb,
    StateVariable,
}

impl FilterKind {
    pub const ALL: [FilterKind; 13] = [
        FilterKind::Ladder4LowPass,
        FilterKind::Ladder2LowPass,
        FilterKind::Ladder4BandPass,
        FilterKind::Ladder2BandPass,
        FilterKind::Ladder4HighPass,
        FilterKind::Ladder2HighPass,
        FilterKind::Sem,
        FilterKind::Korg35LowPass,
        FilterKind::Korg35HighPass,
        FilterKind::Diode,
        FilterKind::Formant,
        FilterKind::Comb,
        FilterKind::StateVariable,
    ];
}

impl std::fmt::Display for FilterKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            FilterKind::Ladder4LowPass => "Ladder LP4",
            FilterKind::Ladder2LowPass => "Ladder LP2",
            FilterKind::Ladder4BandPass => "Ladder BP4",
            FilterKind::Ladder2BandPass => "Ladder BP2",
            FilterKind::Ladder4HighPass => "Ladder HP4",
            FilterKind::Ladder2HighPass => "Ladder HP2",
            FilterKind::Sem => "SEM",
            FilterKind::Korg35LowPass => "K35 LP",
            FilterKind::Korg35HighPass => "K35 HP",
            FilterKind::Diode => "Diode",
            FilterKind::Formant => "Formant",
            FilterKind::Comb => "Comb",
            FilterKind::StateVariable => "SVF",
        };
        write!(formatter, "{}", name)
    }
}
