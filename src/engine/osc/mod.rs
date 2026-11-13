pub mod aa;
pub mod analog;
pub mod chiptune;
pub mod fm;
pub mod multi;
pub mod noise;
pub mod pm;
pub mod unison;
pub mod vector;
pub mod wavetable;

pub use analog::{AnalogShape, AnalogVoiceState};
pub use chiptune::ChiptuneState;
pub use fm::{EXP_FM_SEMITONES, FmState};
pub use multi::{MultiState, OSCS_PER_MULTIOSC};
pub use noise::{NoiseColor, NoiseState};
pub use pm::PmState;
pub use unison::{
    MAX_UNISON_VOICES, detune_multipliers, gain_compensation, pan_gains, pan_positions,
};
pub use vector::{VECTOR_EDGES, VectorState};
pub use wavetable::{CustomLibrary, CustomTable, StockTable, TableSel, TableSource, WtVoice};
