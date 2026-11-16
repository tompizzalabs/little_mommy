pub mod dsp;
pub mod filter;
pub mod graph;
pub mod lfo;
pub mod node;
pub mod osc;

pub use dsp::{EnvStage, NodeState};
pub use filter::FilterKind;
pub use graph::{CompiledGraph, CompiledNode, Edge, Graph, Patch};
pub use lfo::{LfoShape, LfoState};
pub use node::{Node, NodeKind, NodeParams};
pub use osc::{AnalogShape, AnalogVoiceState, NoiseColor};
