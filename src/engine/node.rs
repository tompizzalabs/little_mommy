use crate::engine::lfo::LfoShape;
use crate::engine::osc::{
    AnalogShape, NoiseColor, OperatorWave,
    unison::{DetuneMode, PanMode, StackMode},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Analog,
    Noise,
    Fm,
    Pm,
    Vector,
    Mix,
    Filter,
    Env,
    Gain,
    Lfo,
    Out,
}

#[derive(Debug, Clone, Copy)]
pub struct ParamDef {
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
}

const ANALOG_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Semi",
        min: -24.0,
        max: 24.0,
    },
    ParamDef {
        name: "Detune",
        min: -100.0,
        max: 100.0,
    },
    ParamDef {
        name: "PW",
        min: 0.02,
        max: 0.98,
    },
    ParamDef {
        name: "Drift",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Uni Det",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Det Pwr",
        min: -5.0,
        max: 5.0,
    },
    ParamDef {
        name: "Width",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Blend",
        min: -1.0,
        max: 1.0,
    },
    ParamDef {
        name: "Swarm",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Swarm Rt",
        min: 0.0,
        max: 5.0,
    },
    ParamDef {
        name: "Phase R",
        min: 0.0,
        max: 1.0,
    },
];
const NOISE_DEFS: &[ParamDef] = &[ParamDef {
    name: "Level",
    min: 0.0,
    max: 1.0,
}];
const FM_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Semi",
        min: -24.0,
        max: 24.0,
    },
    ParamDef {
        name: "Detune",
        min: -100.0,
        max: 100.0,
    },
    ParamDef {
        name: "Amount",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Drift",
        min: 0.0,
        max: 1.0,
    },
];
const PM_DEFS: &[ParamDef] = FM_DEFS;
const VECTOR_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "X",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Y",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Semi",
        min: -24.0,
        max: 24.0,
    },
    ParamDef {
        name: "Detune",
        min: -100.0,
        max: 100.0,
    },
    ParamDef {
        name: "Drift",
        min: 0.0,
        max: 1.0,
    },
];
const FILTER_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Cut",
        min: 20.0,
        max: 20000.0,
    },
    ParamDef {
        name: "Res",
        min: 0.0,
        max: 0.95,
    },
];
const ENV_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "A",
        min: 0.0,
        max: 2000.0,
    },
    ParamDef {
        name: "D",
        min: 0.0,
        max: 2000.0,
    },
    ParamDef {
        name: "S",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "R",
        min: 0.0,
        max: 5000.0,
    },
];
const GAIN_DEFS: &[ParamDef] = &[ParamDef {
    name: "Gain",
    min: 0.0,
    max: 1.0,
}];
const LFO_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Rate",
        min: 0.01,
        max: 20.0,
    },
    ParamDef {
        name: "Phase",
        min: 0.0,
        max: 1.0,
    },
];
const EMPTY_DEFS: &[ParamDef] = &[];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeParams {
    Analog {
        shape: AnalogShape,
        semitones: f32,
        detune: f32,
        pulse_width: f32,
        drift: f32,
        unison_voices: u8,
        unison_detune: f32,
        unison_detune_power: f32,
        unison_detune_mode: DetuneMode,
        unison_pan_mode: PanMode,
        unison_stack_mode: StackMode,
        unison_width: f32,
        unison_blend: f32,
        unison_swarm: f32,
        unison_swarm_rate: f32,
        unison_phase_randomization: f32,
    },
    Noise {
        color: NoiseColor,
        level: f32,
    },
    Fm {
        carrier_wave: OperatorWave,
        modulator_wave: OperatorWave,
        semitones: f32,
        detune: f32,
        amount: f32,
        carrier_ratio: u8,
        modulator_ratio: u8,
        exponential: bool,
        drift: f32,
    },
    Pm {
        carrier_wave: OperatorWave,
        modulator_wave: OperatorWave,
        semitones: f32,
        detune: f32,
        amount: f32,
        carrier_ratio: u8,
        modulator_ratio: u8,
        drift: f32,
    },
    Vector {
        semitones: f32,
        detune: f32,
        x: f32,
        y: f32,
        drift: f32,
    },
    Mix,
    Filter {
        cutoff: f32,
        resonance: f32,
    },
    Env {
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    },
    Gain {
        gain: f32,
    },
    Lfo {
        shape: LfoShape,
        rate: f32,
        phase: f32,
    },
    Out,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: u32,
    pub params: NodeParams,
}

impl NodeParams {
    pub fn default_for(kind: NodeKind) -> Self {
        match kind {
            NodeKind::Analog => NodeParams::Analog {
                shape: AnalogShape::Saw,
                semitones: 0.0,
                detune: 0.0,
                pulse_width: 0.25,
                drift: 0.0,
                unison_voices: 1,
                unison_detune: 0.0,
                unison_detune_power: 0.0,
                unison_detune_mode: DetuneMode::Power,
                unison_pan_mode: PanMode::Linear,
                unison_stack_mode: StackMode::Unison,
                unison_width: 1.0,
                unison_blend: 0.0,
                unison_swarm: 0.0,
                unison_swarm_rate: 0.15,
                unison_phase_randomization: 0.0,
            },
            NodeKind::Noise => NodeParams::Noise {
                color: NoiseColor::White,
                level: 0.2,
            },
            NodeKind::Fm => NodeParams::Fm {
                carrier_wave: OperatorWave::Sine,
                modulator_wave: OperatorWave::Sine,
                semitones: 0.0,
                detune: 0.0,
                amount: 0.2,
                carrier_ratio: 1,
                modulator_ratio: 2,
                exponential: false,
                drift: 0.0,
            },
            NodeKind::Pm => NodeParams::Pm {
                carrier_wave: OperatorWave::Sine,
                modulator_wave: OperatorWave::Sine,
                semitones: 0.0,
                detune: 0.0,
                amount: 0.2,
                carrier_ratio: 1,
                modulator_ratio: 2,
                drift: 0.0,
            },
            NodeKind::Vector => NodeParams::Vector {
                semitones: 0.0,
                detune: 0.0,
                x: 0.5,
                y: 0.5,
                drift: 0.0,
            },
            NodeKind::Mix => NodeParams::Mix,
            NodeKind::Filter => NodeParams::Filter {
                cutoff: 8000.0,
                resonance: 0.2,
            },
            NodeKind::Env => NodeParams::Env {
                attack: 5.0,
                decay: 250.0,
                sustain: 0.7,
                release: 300.0,
            },
            NodeKind::Gain => NodeParams::Gain { gain: 0.6 },
            NodeKind::Lfo => NodeParams::Lfo {
                shape: LfoShape::Sine,
                rate: 2.0,
                phase: 0.0,
            },
            NodeKind::Out => NodeParams::Out,
        }
    }

    pub fn kind(&self) -> NodeKind {
        match self {
            NodeParams::Analog { .. } => NodeKind::Analog,
            NodeParams::Noise { .. } => NodeKind::Noise,
            NodeParams::Fm { .. } => NodeKind::Fm,
            NodeParams::Pm { .. } => NodeKind::Pm,
            NodeParams::Vector { .. } => NodeKind::Vector,
            NodeParams::Mix => NodeKind::Mix,
            NodeParams::Filter { .. } => NodeKind::Filter,
            NodeParams::Env { .. } => NodeKind::Env,
            NodeParams::Gain { .. } => NodeKind::Gain,
            NodeParams::Lfo { .. } => NodeKind::Lfo,
            NodeParams::Out => NodeKind::Out,
        }
    }

    pub fn input_count(&self) -> usize {
        match self {
            NodeParams::Analog { .. }
            | NodeParams::Noise { .. }
            | NodeParams::Fm { .. }
            | NodeParams::Pm { .. }
            | NodeParams::Vector { .. }
            | NodeParams::Lfo { .. } => 0,
            NodeParams::Mix => 2,
            NodeParams::Filter { .. }
            | NodeParams::Env { .. }
            | NodeParams::Gain { .. }
            | NodeParams::Out => 1,
        }
    }

    pub fn label(&self) -> &'static str {
        match self.kind() {
            NodeKind::Analog => "ANALOG",
            NodeKind::Noise => "NOISE",
            NodeKind::Fm => "FM",
            NodeKind::Pm => "PM",
            NodeKind::Vector => "VECTOR",
            NodeKind::Mix => "MIX",
            NodeKind::Filter => "FILTER",
            NodeKind::Env => "ENV",
            NodeKind::Gain => "GAIN",
            NodeKind::Lfo => "LFO",
            NodeKind::Out => "OUT",
        }
    }

    pub fn param_defs(&self) -> &'static [ParamDef] {
        match self {
            NodeParams::Analog { .. } => ANALOG_DEFS,
            NodeParams::Noise { .. } => NOISE_DEFS,
            NodeParams::Fm { .. } => FM_DEFS,
            NodeParams::Pm { .. } => PM_DEFS,
            NodeParams::Vector { .. } => VECTOR_DEFS,
            NodeParams::Mix => EMPTY_DEFS,
            NodeParams::Filter { .. } => FILTER_DEFS,
            NodeParams::Env { .. } => ENV_DEFS,
            NodeParams::Gain { .. } => GAIN_DEFS,
            NodeParams::Lfo { .. } => LFO_DEFS,
            NodeParams::Out => EMPTY_DEFS,
        }
    }

    pub fn param_get(&self, param_index: usize) -> f32 {
        match (self, param_index) {
            (NodeParams::Analog { semitones, .. }, 0) => *semitones,
            (NodeParams::Analog { detune, .. }, 1) => *detune,
            (NodeParams::Analog { pulse_width, .. }, 2) => *pulse_width,
            (NodeParams::Analog { drift, .. }, 3) => *drift,
            (NodeParams::Analog { unison_detune, .. }, 4) => *unison_detune,
            (
                NodeParams::Analog {
                    unison_detune_power,
                    ..
                },
                5,
            ) => *unison_detune_power,
            (NodeParams::Analog { unison_width, .. }, 6) => *unison_width,
            (NodeParams::Analog { unison_blend, .. }, 7) => *unison_blend,
            (NodeParams::Analog { unison_swarm, .. }, 8) => *unison_swarm,
            (
                NodeParams::Analog {
                    unison_swarm_rate, ..
                },
                9,
            ) => *unison_swarm_rate,
            (
                NodeParams::Analog {
                    unison_phase_randomization,
                    ..
                },
                10,
            ) => *unison_phase_randomization,
            (NodeParams::Noise { level, .. }, 0) => *level,
            (NodeParams::Fm { semitones, .. }, 0) => *semitones,
            (NodeParams::Fm { detune, .. }, 1) => *detune,
            (NodeParams::Fm { amount, .. }, 2) => *amount,
            (NodeParams::Fm { drift, .. }, 3) => *drift,
            (NodeParams::Pm { semitones, .. }, 0) => *semitones,
            (NodeParams::Pm { detune, .. }, 1) => *detune,
            (NodeParams::Pm { amount, .. }, 2) => *amount,
            (NodeParams::Pm { drift, .. }, 3) => *drift,
            (NodeParams::Vector { x, .. }, 0) => *x,
            (NodeParams::Vector { y, .. }, 1) => *y,
            (NodeParams::Vector { semitones, .. }, 2) => *semitones,
            (NodeParams::Vector { detune, .. }, 3) => *detune,
            (NodeParams::Vector { drift, .. }, 4) => *drift,
            (NodeParams::Filter { cutoff, .. }, 0) => *cutoff,
            (NodeParams::Filter { resonance, .. }, 1) => *resonance,
            (NodeParams::Env { attack, .. }, 0) => *attack,
            (NodeParams::Env { decay, .. }, 1) => *decay,
            (NodeParams::Env { sustain, .. }, 2) => *sustain,
            (NodeParams::Env { release, .. }, 3) => *release,
            (NodeParams::Gain { gain, .. }, 0) => *gain,
            (NodeParams::Lfo { rate, .. }, 0) => *rate,
            (NodeParams::Lfo { phase, .. }, 1) => *phase,
            _ => 0.0,
        }
    }

    pub fn param_set(&mut self, param_index: usize, value: f32) {
        match (self, param_index) {
            (NodeParams::Analog { semitones, .. }, 0) => *semitones = value,
            (NodeParams::Analog { detune, .. }, 1) => *detune = value,
            (NodeParams::Analog { pulse_width, .. }, 2) => *pulse_width = value,
            (NodeParams::Analog { drift, .. }, 3) => *drift = value,
            (NodeParams::Analog { unison_detune, .. }, 4) => *unison_detune = value,
            (
                NodeParams::Analog {
                    unison_detune_power,
                    ..
                },
                5,
            ) => *unison_detune_power = value,
            (NodeParams::Analog { unison_width, .. }, 6) => *unison_width = value,
            (NodeParams::Analog { unison_blend, .. }, 7) => *unison_blend = value,
            (NodeParams::Analog { unison_swarm, .. }, 8) => *unison_swarm = value,
            (
                NodeParams::Analog {
                    unison_swarm_rate, ..
                },
                9,
            ) => *unison_swarm_rate = value,
            (
                NodeParams::Analog {
                    unison_phase_randomization,
                    ..
                },
                10,
            ) => *unison_phase_randomization = value,
            (NodeParams::Noise { level, .. }, 0) => *level = value,
            (NodeParams::Fm { semitones, .. }, 0) => *semitones = value,
            (NodeParams::Fm { detune, .. }, 1) => *detune = value,
            (NodeParams::Fm { amount, .. }, 2) => *amount = value,
            (NodeParams::Fm { drift, .. }, 3) => *drift = value,
            (NodeParams::Pm { semitones, .. }, 0) => *semitones = value,
            (NodeParams::Pm { detune, .. }, 1) => *detune = value,
            (NodeParams::Pm { amount, .. }, 2) => *amount = value,
            (NodeParams::Pm { drift, .. }, 3) => *drift = value,
            (NodeParams::Vector { x, .. }, 0) => *x = value,
            (NodeParams::Vector { y, .. }, 1) => *y = value,
            (NodeParams::Vector { semitones, .. }, 2) => *semitones = value,
            (NodeParams::Vector { detune, .. }, 3) => *detune = value,
            (NodeParams::Vector { drift, .. }, 4) => *drift = value,
            (NodeParams::Filter { cutoff, .. }, 0) => *cutoff = value,
            (NodeParams::Filter { resonance, .. }, 1) => *resonance = value,
            (NodeParams::Env { attack, .. }, 0) => *attack = value,
            (NodeParams::Env { decay, .. }, 1) => *decay = value,
            (NodeParams::Env { sustain, .. }, 2) => *sustain = value,
            (NodeParams::Env { release, .. }, 3) => *release = value,
            (NodeParams::Gain { gain, .. }, 0) => *gain = value,
            (NodeParams::Lfo { rate, .. }, 0) => *rate = value,
            (NodeParams::Lfo { phase, .. }, 1) => *phase = value,
            _ => {}
        }
    }
}
