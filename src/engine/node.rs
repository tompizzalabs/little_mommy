use crate::engine::effects::DistortionAlgorithm;
use crate::engine::filter::FilterKind;
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
    Delay,
    Chorus,
    Flanger,
    Phaser,
    Bitcrusher,
    RingMod,
    Distortion,
    Equalizer,
    Reverb,
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
        max: 1.0,
    },
    ParamDef {
        name: "Morph",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Vowel L",
        min: 0.0,
        max: 7.0,
    },
    ParamDef {
        name: "Vowel R",
        min: 0.0,
        max: 7.0,
    },
    ParamDef {
        name: "Sat",
        min: 0.0,
        max: 1.0,
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
const DELAY_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Time",
        min: 0.0,
        max: 4.0,
    },
    ParamDef {
        name: "Feed",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Dry",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Wet",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Duck",
        min: 0.0,
        max: 1.0,
    },
];
const CHORUS_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Rate",
        min: 0.01,
        max: 8.0,
    },
    ParamDef {
        name: "Depth",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Feed",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Mix",
        min: 0.0,
        max: 1.0,
    },
];
const FLANGER_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Rate",
        min: 0.01,
        max: 8.0,
    },
    ParamDef {
        name: "Depth",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Feed",
        min: -1.0,
        max: 1.0,
    },
    ParamDef {
        name: "Mix",
        min: 0.0,
        max: 1.0,
    },
];
const PHASER_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Rate",
        min: 0.01,
        max: 8.0,
    },
    ParamDef {
        name: "Freq",
        min: 400.0,
        max: 8000.0,
    },
    ParamDef {
        name: "Depth",
        min: 0.0,
        max: 1.5,
    },
    ParamDef {
        name: "Feed",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Mix",
        min: 0.0,
        max: 1.0,
    },
];
const BITCRUSHER_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Bits",
        min: 1.0,
        max: 16.0,
    },
    ParamDef {
        name: "Edge",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Mix",
        min: 0.0,
        max: 1.0,
    },
];
const RING_MOD_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Freq",
        min: 0.01,
        max: 20000.0,
    },
    ParamDef {
        name: "Amount",
        min: 0.0,
        max: 1.0,
    },
];
const DISTORTION_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Thresh",
        min: 0.0,
        max: 1.0,
    },
    ParamDef {
        name: "Mix",
        min: 0.0,
        max: 1.0,
    },
];
const EQUALIZER_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Freq",
        min: 20.0,
        max: 20000.0,
    },
    ParamDef {
        name: "Q",
        min: 0.1,
        max: 10.0,
    },
    ParamDef {
        name: "Gain",
        min: -24.0,
        max: 24.0,
    },
];
const REVERB_DEFS: &[ParamDef] = &[
    ParamDef {
        name: "Delay",
        min: 0.02,
        max: 0.1,
    },
    ParamDef {
        name: "Xover",
        min: 20.0,
        max: 2000.0,
    },
    ParamDef {
        name: "Rt Low",
        min: 0.1,
        max: 10.0,
    },
    ParamDef {
        name: "Rt Mid",
        min: 0.1,
        max: 10.0,
    },
    ParamDef {
        name: "Damp",
        min: 100.0,
        max: 18000.0,
    },
    ParamDef {
        name: "Mix",
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
        kind: FilterKind,
        cutoff: f32,
        resonance: f32,
        transition: f32,
        vowel_left: f32,
        vowel_right: f32,
        saturation: f32,
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
    Delay {
        time: f32,
        feedback: f32,
        dry: f32,
        wet: f32,
        ducking: f32,
        ping_pong: bool,
    },
    Chorus {
        rate: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
    },
    Flanger {
        rate: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
    },
    Phaser {
        rate: f32,
        base_frequency: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
    },
    Bitcrusher {
        bits: f32,
        edge: f32,
        dry_wet: f32,
    },
    RingMod {
        frequency: f32,
        amount: f32,
    },
    Distortion {
        algorithm: DistortionAlgorithm,
        threshold: f32,
        dry_wet: f32,
    },
    Equalizer {
        frequency: f32,
        q: f32,
        gain_db: f32,
    },
    Reverb {
        pre_delay: f32,
        xover: f32,
        low_decay: f32,
        mid_decay: f32,
        damping: f32,
        mix: f32,
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
                kind: FilterKind::StateVariable,
                cutoff: 8000.0,
                resonance: 0.2,
                transition: 0.5,
                vowel_left: 0.0,
                vowel_right: 1.0,
                saturation: 0.0,
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
            NodeKind::Delay => NodeParams::Delay {
                time: 0.25,
                feedback: 0.45,
                dry: 1.0,
                wet: 0.5,
                ducking: 0.0,
                ping_pong: false,
            },
            NodeKind::Chorus => NodeParams::Chorus {
                rate: 0.15,
                amount: 0.4,
                feedback: 0.0,
                dry_wet: 0.7,
            },
            NodeKind::Flanger => NodeParams::Flanger {
                rate: 0.2,
                amount: 0.3,
                feedback: 0.6,
                dry_wet: 0.7,
            },
            NodeKind::Phaser => NodeParams::Phaser {
                rate: 0.5,
                base_frequency: 4000.0,
                amount: 0.3,
                feedback: 0.5,
                dry_wet: 1.0,
            },
            NodeKind::Bitcrusher => NodeParams::Bitcrusher {
                bits: 8.0,
                edge: 1.0,
                dry_wet: 0.7,
            },
            NodeKind::RingMod => NodeParams::RingMod {
                frequency: 100.0,
                amount: 1.0,
            },
            NodeKind::Distortion => NodeParams::Distortion {
                algorithm: DistortionAlgorithm::Clamp,
                threshold: 0.5,
                dry_wet: 1.0,
            },
            NodeKind::Equalizer => NodeParams::Equalizer {
                frequency: 1000.0,
                q: 1.0,
                gain_db: 0.0,
            },
            NodeKind::Reverb => NodeParams::Reverb {
                pre_delay: 0.04,
                xover: 200.0,
                low_decay: 3.0,
                mid_decay: 2.0,
                damping: 3000.0,
                mix: 0.25,
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
            NodeParams::Delay { .. } => NodeKind::Delay,
            NodeParams::Chorus { .. } => NodeKind::Chorus,
            NodeParams::Flanger { .. } => NodeKind::Flanger,
            NodeParams::Phaser { .. } => NodeKind::Phaser,
            NodeParams::Bitcrusher { .. } => NodeKind::Bitcrusher,
            NodeParams::RingMod { .. } => NodeKind::RingMod,
            NodeParams::Distortion { .. } => NodeKind::Distortion,
            NodeParams::Equalizer { .. } => NodeKind::Equalizer,
            NodeParams::Reverb { .. } => NodeKind::Reverb,
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
            | NodeParams::Delay { .. }
            | NodeParams::Chorus { .. }
            | NodeParams::Flanger { .. }
            | NodeParams::Phaser { .. }
            | NodeParams::Bitcrusher { .. }
            | NodeParams::RingMod { .. }
            | NodeParams::Distortion { .. }
            | NodeParams::Equalizer { .. }
            | NodeParams::Reverb { .. }
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
            NodeKind::Delay => "DELAY",
            NodeKind::Chorus => "CHORUS",
            NodeKind::Flanger => "FLANGER",
            NodeKind::Phaser => "PHASER",
            NodeKind::Bitcrusher => "CRUSH",
            NodeKind::RingMod => "RINGMOD",
            NodeKind::Distortion => "DISTORT",
            NodeKind::Equalizer => "EQ",
            NodeKind::Reverb => "VERB",
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
            NodeParams::Delay { .. } => DELAY_DEFS,
            NodeParams::Chorus { .. } => CHORUS_DEFS,
            NodeParams::Flanger { .. } => FLANGER_DEFS,
            NodeParams::Phaser { .. } => PHASER_DEFS,
            NodeParams::Bitcrusher { .. } => BITCRUSHER_DEFS,
            NodeParams::RingMod { .. } => RING_MOD_DEFS,
            NodeParams::Distortion { .. } => DISTORTION_DEFS,
            NodeParams::Equalizer { .. } => EQUALIZER_DEFS,
            NodeParams::Reverb { .. } => REVERB_DEFS,
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
            (NodeParams::Filter { transition, .. }, 2) => *transition,
            (NodeParams::Filter { vowel_left, .. }, 3) => *vowel_left,
            (NodeParams::Filter { vowel_right, .. }, 4) => *vowel_right,
            (NodeParams::Filter { saturation, .. }, 5) => *saturation,
            (NodeParams::Env { attack, .. }, 0) => *attack,
            (NodeParams::Env { decay, .. }, 1) => *decay,
            (NodeParams::Env { sustain, .. }, 2) => *sustain,
            (NodeParams::Env { release, .. }, 3) => *release,
            (NodeParams::Gain { gain, .. }, 0) => *gain,
            (NodeParams::Lfo { rate, .. }, 0) => *rate,
            (NodeParams::Lfo { phase, .. }, 1) => *phase,
            (NodeParams::Delay { time, .. }, 0) => *time,
            (NodeParams::Delay { feedback, .. }, 1) => *feedback,
            (NodeParams::Delay { dry, .. }, 2) => *dry,
            (NodeParams::Delay { wet, .. }, 3) => *wet,
            (NodeParams::Delay { ducking, .. }, 4) => *ducking,
            (NodeParams::Chorus { rate, .. }, 0) => *rate,
            (NodeParams::Chorus { amount, .. }, 1) => *amount,
            (NodeParams::Chorus { feedback, .. }, 2) => *feedback,
            (NodeParams::Chorus { dry_wet, .. }, 3) => *dry_wet,
            (NodeParams::Flanger { rate, .. }, 0) => *rate,
            (NodeParams::Flanger { amount, .. }, 1) => *amount,
            (NodeParams::Flanger { feedback, .. }, 2) => *feedback,
            (NodeParams::Flanger { dry_wet, .. }, 3) => *dry_wet,
            (NodeParams::Phaser { rate, .. }, 0) => *rate,
            (NodeParams::Phaser { base_frequency, .. }, 1) => *base_frequency,
            (NodeParams::Phaser { amount, .. }, 2) => *amount,
            (NodeParams::Phaser { feedback, .. }, 3) => *feedback,
            (NodeParams::Phaser { dry_wet, .. }, 4) => *dry_wet,
            (NodeParams::Bitcrusher { bits, .. }, 0) => *bits,
            (NodeParams::Bitcrusher { edge, .. }, 1) => *edge,
            (NodeParams::Bitcrusher { dry_wet, .. }, 2) => *dry_wet,
            (NodeParams::RingMod { frequency, .. }, 0) => *frequency,
            (NodeParams::RingMod { amount, .. }, 1) => *amount,
            (NodeParams::Distortion { threshold, .. }, 0) => *threshold,
            (NodeParams::Distortion { dry_wet, .. }, 1) => *dry_wet,
            (NodeParams::Equalizer { frequency, .. }, 0) => *frequency,
            (NodeParams::Equalizer { q, .. }, 1) => *q,
            (NodeParams::Equalizer { gain_db, .. }, 2) => *gain_db,
            (NodeParams::Reverb { pre_delay, .. }, 0) => *pre_delay,
            (NodeParams::Reverb { xover, .. }, 1) => *xover,
            (NodeParams::Reverb { low_decay, .. }, 2) => *low_decay,
            (NodeParams::Reverb { mid_decay, .. }, 3) => *mid_decay,
            (NodeParams::Reverb { damping, .. }, 4) => *damping,
            (NodeParams::Reverb { mix, .. }, 5) => *mix,
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
            (NodeParams::Filter { transition, .. }, 2) => *transition = value,
            (NodeParams::Filter { vowel_left, .. }, 3) => *vowel_left = value,
            (NodeParams::Filter { vowel_right, .. }, 4) => *vowel_right = value,
            (NodeParams::Filter { saturation, .. }, 5) => *saturation = value,
            (NodeParams::Env { attack, .. }, 0) => *attack = value,
            (NodeParams::Env { decay, .. }, 1) => *decay = value,
            (NodeParams::Env { sustain, .. }, 2) => *sustain = value,
            (NodeParams::Env { release, .. }, 3) => *release = value,
            (NodeParams::Gain { gain, .. }, 0) => *gain = value,
            (NodeParams::Lfo { rate, .. }, 0) => *rate = value,
            (NodeParams::Lfo { phase, .. }, 1) => *phase = value,
            (NodeParams::Delay { time, .. }, 0) => *time = value,
            (NodeParams::Delay { feedback, .. }, 1) => *feedback = value,
            (NodeParams::Delay { dry, .. }, 2) => *dry = value,
            (NodeParams::Delay { wet, .. }, 3) => *wet = value,
            (NodeParams::Delay { ducking, .. }, 4) => *ducking = value,
            (NodeParams::Chorus { rate, .. }, 0) => *rate = value,
            (NodeParams::Chorus { amount, .. }, 1) => *amount = value,
            (NodeParams::Chorus { feedback, .. }, 2) => *feedback = value,
            (NodeParams::Chorus { dry_wet, .. }, 3) => *dry_wet = value,
            (NodeParams::Flanger { rate, .. }, 0) => *rate = value,
            (NodeParams::Flanger { amount, .. }, 1) => *amount = value,
            (NodeParams::Flanger { feedback, .. }, 2) => *feedback = value,
            (NodeParams::Flanger { dry_wet, .. }, 3) => *dry_wet = value,
            (NodeParams::Phaser { rate, .. }, 0) => *rate = value,
            (NodeParams::Phaser { base_frequency, .. }, 1) => *base_frequency = value,
            (NodeParams::Phaser { amount, .. }, 2) => *amount = value,
            (NodeParams::Phaser { feedback, .. }, 3) => *feedback = value,
            (NodeParams::Phaser { dry_wet, .. }, 4) => *dry_wet = value,
            (NodeParams::Bitcrusher { bits, .. }, 0) => *bits = value,
            (NodeParams::Bitcrusher { edge, .. }, 1) => *edge = value,
            (NodeParams::Bitcrusher { dry_wet, .. }, 2) => *dry_wet = value,
            (NodeParams::RingMod { frequency, .. }, 0) => *frequency = value,
            (NodeParams::RingMod { amount, .. }, 1) => *amount = value,
            (NodeParams::Distortion { threshold, .. }, 0) => *threshold = value,
            (NodeParams::Distortion { dry_wet, .. }, 1) => *dry_wet = value,
            (NodeParams::Equalizer { frequency, .. }, 0) => *frequency = value,
            (NodeParams::Equalizer { q, .. }, 1) => *q = value,
            (NodeParams::Equalizer { gain_db, .. }, 2) => *gain_db = value,
            (NodeParams::Reverb { pre_delay, .. }, 0) => *pre_delay = value,
            (NodeParams::Reverb { xover, .. }, 1) => *xover = value,
            (NodeParams::Reverb { low_decay, .. }, 2) => *low_decay = value,
            (NodeParams::Reverb { mid_decay, .. }, 3) => *mid_decay = value,
            (NodeParams::Reverb { damping, .. }, 4) => *damping = value,
            (NodeParams::Reverb { mix, .. }, 5) => *mix = value,
            _ => {}
        }
    }
}
