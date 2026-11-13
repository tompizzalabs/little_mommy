use crate::engine::lfo::LfoShape;
use crate::engine::osc::{AnalogShape, NoiseColor, TableSel, wavetable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Analog,
    Noise,
    Wave,
    Chiptune,
    Multi,
    Vector,
    Fm,
    Pm,
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
];
const NOISE_DEFS: &[ParamDef] = &[ParamDef {
    name: "Level",
    min: 0.0,
    max: 1.0,
}];
const WAVE_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Pos",
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
        name: "Form",
        min: 0.25,
        max: 4.0,
    },
    ParamDef {
        name: "SkewH",
        min: -0.95,
        max: 0.95,
    },
    ParamDef {
        name: "SkewV",
        min: -0.9,
        max: 0.9,
    },
    ParamDef {
        name: "Sat",
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
];
const CHIPTUNE_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Pos",
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
        name: "Arp Speed",
        min: 0.01,
        max: 40.0,
    },
];
const MULTI_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Pos",
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
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Spread",
        min: 0.0,
        max: 1.0,
    },
];
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
        name: "P0",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "P1",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "P2",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "P3",
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
];
const FM_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "C Pos",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "M Pos",
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
        name: "Amount",
        min: 0.0,
        max: 1.0,
    },
];
const PM_DEFS: &[ParamDef] = &FM_DEFS;
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

#[derive(Debug, Clone, PartialEq)]
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
    },
    Noise {
        color: NoiseColor,
        level: f32,
    },
    Wave {
        table: TableSel,
        position: f32,
        semitones: f32,
        detune: f32,
        formant: f32,
        skew_horizontal: f32,
        skew_vertical: f32,
        saturation: f32,
        unison_voices: u8,
        unison_detune: f32,
        unison_detune_power: f32,
    },
    Chiptune {
        table: TableSel,
        position: f32,
        semitones: f32,
        detune: f32,
        arp_enabled: bool,
        arp_three: bool,
        arp_speed: f32,
        arp_steps: [i32; 3],
        noise: bool,
    },
    Multi {
        table: TableSel,
        position: f32,
        semitones: f32,
        detune: f32,
        spread: f32,
    },
    Vector {
        tables: [TableSel; 4],
        positions: [f32; 4],
        semitones: f32,
        detune: f32,
        x: f32,
        y: f32,
    },
    Fm {
        carrier_table: TableSel,
        modulator_table: TableSel,
        carrier_position: f32,
        modulator_position: f32,
        semitones: f32,
        detune: f32,
        amount: f32,
        carrier_ratio: u8,
        modulator_ratio: u8,
        exponential: bool,
    },
    Pm {
        carrier_table: TableSel,
        modulator_table: TableSel,
        carrier_position: f32,
        modulator_position: f32,
        semitones: f32,
        detune: f32,
        amount: f32,
        carrier_ratio: u8,
        modulator_ratio: u8,
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
            },
            NodeKind::Noise => NodeParams::Noise {
                color: NoiseColor::White,
                level: 0.2,
            },
            NodeKind::Wave => NodeParams::Wave {
                table: wavetable::default_sel(),
                position: 0.35,
                semitones: 0.0,
                detune: 0.0,
                formant: 1.0,
                skew_horizontal: 0.0,
                skew_vertical: 0.0,
                saturation: 0.0,
                unison_voices: 1,
                unison_detune: 0.0,
                unison_detune_power: 0.0,
            },
            NodeKind::Chiptune => NodeParams::Chiptune {
                table: wavetable::default_sel(),
                position: 0.5,
                semitones: 0.0,
                detune: 0.0,
                arp_enabled: true,
                arp_three: true,
                arp_speed: 20.0,
                arp_steps: [0, 12, 7],
                noise: false,
            },
            NodeKind::Multi => NodeParams::Multi {
                table: wavetable::default_sel(),
                position: 0.5,
                semitones: 0.0,
                detune: 0.2,
                spread: 0.4,
            },
            NodeKind::Vector => NodeParams::Vector {
                tables: std::array::from_fn(|_| wavetable::default_sel()),
                positions: [0.5; 4],
                semitones: 0.0,
                detune: 0.0,
                x: 0.5,
                y: 0.5,
            },
            NodeKind::Fm => NodeParams::Fm {
                carrier_table: wavetable::default_sel(),
                modulator_table: wavetable::default_sel(),
                carrier_position: 0.5,
                modulator_position: 0.5,
                semitones: 0.0,
                detune: 0.0,
                amount: 0.2,
                carrier_ratio: 1,
                modulator_ratio: 2,
                exponential: false,
            },
            NodeKind::Pm => NodeParams::Pm {
                carrier_table: wavetable::default_sel(),
                modulator_table: wavetable::default_sel(),
                carrier_position: 0.5,
                modulator_position: 0.5,
                semitones: 0.0,
                detune: 0.0,
                amount: 0.2,
                carrier_ratio: 1,
                modulator_ratio: 2,
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
            NodeParams::Wave { .. } => NodeKind::Wave,
            NodeParams::Chiptune { .. } => NodeKind::Chiptune,
            NodeParams::Multi { .. } => NodeKind::Multi,
            NodeParams::Vector { .. } => NodeKind::Vector,
            NodeParams::Fm { .. } => NodeKind::Fm,
            NodeParams::Pm { .. } => NodeKind::Pm,
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
            | NodeParams::Wave { .. }
            | NodeParams::Chiptune { .. }
            | NodeParams::Multi { .. }
            | NodeParams::Vector { .. }
            | NodeParams::Fm { .. }
            | NodeParams::Pm { .. }
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
            NodeKind::Wave => "WAVE",
            NodeKind::Chiptune => "CHIPTUNE",
            NodeKind::Multi => "MULTI",
            NodeKind::Vector => "VECTOR",
            NodeKind::Fm => "FM",
            NodeKind::Pm => "PM",
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
            NodeParams::Wave { .. } => WAVE_DEFS,
            NodeParams::Chiptune { .. } => CHIPTUNE_DEFS,
            NodeParams::Multi { .. } => MULTI_DEFS,
            NodeParams::Vector { .. } => VECTOR_DEFS,
            NodeParams::Fm { .. } => FM_DEFS,
            NodeParams::Pm { .. } => PM_DEFS,
            NodeParams::Mix => EMPTY_DEFS,
            NodeParams::Filter { .. } => FILTER_DEFS,
            NodeParams::Env { .. } => ENV_DEFS,
            NodeParams::Gain { .. } => GAIN_DEFS,
            NodeParams::Lfo { .. } => LFO_DEFS,
            NodeParams::Out => EMPTY_DEFS,
        }
    }

    pub fn param_get(&self, i: usize) -> f32 {
        match (self, i) {
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
            (NodeParams::Noise { level, .. }, 0) => *level,
            (NodeParams::Wave { position, .. }, 0) => *position,
            (NodeParams::Wave { semitones, .. }, 1) => *semitones,
            (NodeParams::Wave { detune, .. }, 2) => *detune,
            (NodeParams::Wave { formant, .. }, 3) => *formant,
            (
                NodeParams::Wave {
                    skew_horizontal, ..
                },
                4,
            ) => *skew_horizontal,
            (NodeParams::Wave { skew_vertical, .. }, 5) => *skew_vertical,
            (NodeParams::Wave { saturation, .. }, 6) => *saturation,
            (NodeParams::Wave { unison_detune, .. }, 7) => *unison_detune,
            (
                NodeParams::Wave {
                    unison_detune_power,
                    ..
                },
                8,
            ) => *unison_detune_power,
            (NodeParams::Chiptune { position, .. }, 0) => *position,
            (NodeParams::Chiptune { semitones, .. }, 1) => *semitones,
            (NodeParams::Chiptune { detune, .. }, 2) => *detune,
            (NodeParams::Chiptune { arp_speed, .. }, 3) => *arp_speed,
            (NodeParams::Multi { position, .. }, 0) => *position,
            (NodeParams::Multi { semitones, .. }, 1) => *semitones,
            (NodeParams::Multi { detune, .. }, 2) => *detune,
            (NodeParams::Multi { spread, .. }, 3) => *spread,
            (NodeParams::Vector { x, .. }, 0) => *x,
            (NodeParams::Vector { y, .. }, 1) => *y,
            (NodeParams::Vector { positions, .. }, 2) => positions[0],
            (NodeParams::Vector { positions, .. }, 3) => positions[1],
            (NodeParams::Vector { positions, .. }, 4) => positions[2],
            (NodeParams::Vector { positions, .. }, 5) => positions[3],
            (NodeParams::Vector { semitones, .. }, 6) => *semitones,
            (NodeParams::Vector { detune, .. }, 7) => *detune,
            (
                NodeParams::Fm {
                    carrier_position, ..
                },
                0,
            ) => *carrier_position,
            (
                NodeParams::Fm {
                    modulator_position, ..
                },
                1,
            ) => *modulator_position,
            (NodeParams::Fm { semitones, .. }, 2) => *semitones,
            (NodeParams::Fm { detune, .. }, 3) => *detune,
            (NodeParams::Fm { amount, .. }, 4) => *amount,
            (
                NodeParams::Pm {
                    carrier_position, ..
                },
                0,
            ) => *carrier_position,
            (
                NodeParams::Pm {
                    modulator_position, ..
                },
                1,
            ) => *modulator_position,
            (NodeParams::Pm { semitones, .. }, 2) => *semitones,
            (NodeParams::Pm { detune, .. }, 3) => *detune,
            (NodeParams::Pm { amount, .. }, 4) => *amount,
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

    pub fn param_set(&mut self, i: usize, v: f32) {
        match (self, i) {
            (NodeParams::Analog { semitones, .. }, 0) => *semitones = v,
            (NodeParams::Analog { detune, .. }, 1) => *detune = v,
            (NodeParams::Analog { pulse_width, .. }, 2) => *pulse_width = v,
            (NodeParams::Analog { drift, .. }, 3) => *drift = v,
            (NodeParams::Analog { unison_detune, .. }, 4) => *unison_detune = v,
            (
                NodeParams::Analog {
                    unison_detune_power,
                    ..
                },
                5,
            ) => *unison_detune_power = v,
            (NodeParams::Noise { level, .. }, 0) => *level = v,
            (NodeParams::Wave { position, .. }, 0) => *position = v,
            (NodeParams::Wave { semitones, .. }, 1) => *semitones = v,
            (NodeParams::Wave { detune, .. }, 2) => *detune = v,
            (NodeParams::Wave { formant, .. }, 3) => *formant = v,
            (
                NodeParams::Wave {
                    skew_horizontal, ..
                },
                4,
            ) => *skew_horizontal = v,
            (NodeParams::Wave { skew_vertical, .. }, 5) => *skew_vertical = v,
            (NodeParams::Wave { saturation, .. }, 6) => *saturation = v,
            (NodeParams::Wave { unison_detune, .. }, 7) => *unison_detune = v,
            (
                NodeParams::Wave {
                    unison_detune_power,
                    ..
                },
                8,
            ) => *unison_detune_power = v,
            (NodeParams::Chiptune { position, .. }, 0) => *position = v,
            (NodeParams::Chiptune { semitones, .. }, 1) => *semitones = v,
            (NodeParams::Chiptune { detune, .. }, 2) => *detune = v,
            (NodeParams::Chiptune { arp_speed, .. }, 3) => *arp_speed = v,
            (NodeParams::Multi { position, .. }, 0) => *position = v,
            (NodeParams::Multi { semitones, .. }, 1) => *semitones = v,
            (NodeParams::Multi { detune, .. }, 2) => *detune = v,
            (NodeParams::Multi { spread, .. }, 3) => *spread = v,
            (NodeParams::Vector { x, .. }, 0) => *x = v,
            (NodeParams::Vector { y, .. }, 1) => *y = v,
            (NodeParams::Vector { positions, .. }, 2) => positions[0] = v,
            (NodeParams::Vector { positions, .. }, 3) => positions[1] = v,
            (NodeParams::Vector { positions, .. }, 4) => positions[2] = v,
            (NodeParams::Vector { positions, .. }, 5) => positions[3] = v,
            (NodeParams::Vector { semitones, .. }, 6) => *semitones = v,
            (NodeParams::Vector { detune, .. }, 7) => *detune = v,
            (
                NodeParams::Fm {
                    carrier_position, ..
                },
                0,
            ) => *carrier_position = v,
            (
                NodeParams::Fm {
                    modulator_position, ..
                },
                1,
            ) => *modulator_position = v,
            (NodeParams::Fm { semitones, .. }, 2) => *semitones = v,
            (NodeParams::Fm { detune, .. }, 3) => *detune = v,
            (NodeParams::Fm { amount, .. }, 4) => *amount = v,
            (
                NodeParams::Pm {
                    carrier_position, ..
                },
                0,
            ) => *carrier_position = v,
            (
                NodeParams::Pm {
                    modulator_position, ..
                },
                1,
            ) => *modulator_position = v,
            (NodeParams::Pm { semitones, .. }, 2) => *semitones = v,
            (NodeParams::Pm { detune, .. }, 3) => *detune = v,
            (NodeParams::Pm { amount, .. }, 4) => *amount = v,
            (NodeParams::Filter { cutoff, .. }, 0) => *cutoff = v,
            (NodeParams::Filter { resonance, .. }, 1) => *resonance = v,
            (NodeParams::Env { attack, .. }, 0) => *attack = v,
            (NodeParams::Env { decay, .. }, 1) => *decay = v,
            (NodeParams::Env { sustain, .. }, 2) => *sustain = v,
            (NodeParams::Env { release, .. }, 3) => *release = v,
            (NodeParams::Gain { gain, .. }, 0) => *gain = v,
            (NodeParams::Lfo { rate, .. }, 0) => *rate = v,
            (NodeParams::Lfo { phase, .. }, 1) => *phase = v,
            _ => {}
        }
    }
}
