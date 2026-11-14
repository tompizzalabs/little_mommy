use crate::engine::graph::{Grid, InputSel, Patch, PlayMode};
use crate::engine::node::NodeParams;
use crate::engine::osc::{AnalogShape, NoiseColor, StockTable, TableSel, unison};

pub struct Preset {
    pub name: &'static str,
    pub cells: Vec<(usize, NodeParams, InputSel, InputSel)>,
    pub play_mode: PlayMode,
}

fn analog(shape: AnalogShape, semitones: f32, detune: f32, drift: f32) -> NodeParams {
    NodeParams::Analog {
        shape,
        semitones,
        detune,
        pulse_width: 0.5,
        drift,
        unison_voices: 1,
        unison_detune: 0.0,
        unison_detune_power: 1.0,
        unison_detune_mode: unison::DetuneMode::Power,
        unison_pan_mode: unison::PanMode::Linear,
        unison_stack_mode: unison::StackMode::Unison,
        unison_width: 1.0,
        unison_blend: 0.0,
        unison_swarm: 0.0,
        unison_swarm_rate: 0.15,
        unison_phase_randomization: 0.0,
    }
}

fn init_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog(AnalogShape::Saw, 0.0, 0.0, 0.0),
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            NodeParams::Filter {
                cutoff: 8000.0,
                resonance: 0.2,
            },
            InputSel::Slot(0),
            InputSel::None,
        ),
        (
            2,
            NodeParams::Env {
                attack: 5.0,
                decay: 250.0,
                sustain: 0.7,
                release: 300.0,
            },
            InputSel::Slot(1),
            InputSel::None,
        ),
        (
            3,
            NodeParams::Gain { gain: 0.6 },
            InputSel::Slot(2),
            InputSel::None,
        ),
        (4, NodeParams::Out, InputSel::Slot(3), InputSel::None),
    ]
}

fn pluck_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog(AnalogShape::Square, 0.0, 0.0, 0.0),
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            NodeParams::Noise {
                color: NoiseColor::Pink,
                level: 0.3,
            },
            InputSel::None,
            InputSel::None,
        ),
        (2, NodeParams::Mix, InputSel::Slot(0), InputSel::Slot(1)),
        (
            3,
            NodeParams::Filter {
                cutoff: 3000.0,
                resonance: 0.35,
            },
            InputSel::Slot(2),
            InputSel::None,
        ),
        (
            4,
            NodeParams::Env {
                attack: 1.0,
                decay: 180.0,
                sustain: 0.0,
                release: 200.0,
            },
            InputSel::Slot(3),
            InputSel::None,
        ),
        (5, NodeParams::Out, InputSel::Slot(4), InputSel::None),
    ]
}

fn pad_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            NodeParams::Wave {
                table: TableSel::Stock(StockTable::Morph),
                position: 0.4,
                semitones: 0.0,
                detune: -7.0,
                formant: 1.0,
                skew_horizontal: 0.0,
                skew_vertical: 0.0,
                saturation: 0.0,
                unison_voices: 3,
                unison_detune: 0.4,
                unison_detune_power: 1.0,
                unison_detune_mode: unison::DetuneMode::Power,
                unison_pan_mode: unison::PanMode::Linear,
                unison_stack_mode: unison::StackMode::Unison,
                unison_width: 1.0,
                unison_blend: 0.0,
                unison_swarm: 0.25,
                unison_swarm_rate: 0.15,
                unison_phase_randomization: 0.0,
            },
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            analog(AnalogShape::Saw, 0.0, 7.0, 0.4),
            InputSel::None,
            InputSel::None,
        ),
        (2, NodeParams::Mix, InputSel::Slot(0), InputSel::Slot(1)),
        (
            3,
            NodeParams::Filter {
                cutoff: 2200.0,
                resonance: 0.15,
            },
            InputSel::Slot(2),
            InputSel::None,
        ),
        (
            4,
            NodeParams::Env {
                attack: 300.0,
                decay: 400.0,
                sustain: 0.8,
                release: 900.0,
            },
            InputSel::Slot(3),
            InputSel::None,
        ),
        (
            5,
            NodeParams::Gain { gain: 0.5 },
            InputSel::Slot(4),
            InputSel::None,
        ),
        (6, NodeParams::Out, InputSel::Slot(5), InputSel::None),
    ]
}

fn bass_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog(AnalogShape::Square, -12.0, 0.0, 0.25),
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            NodeParams::Filter {
                cutoff: 450.0,
                resonance: 0.3,
            },
            InputSel::Slot(0),
            InputSel::None,
        ),
        (
            2,
            NodeParams::Env {
                attack: 2.0,
                decay: 120.0,
                sustain: 0.9,
                release: 150.0,
            },
            InputSel::Slot(1),
            InputSel::None,
        ),
        (
            3,
            NodeParams::Gain { gain: 0.8 },
            InputSel::Slot(2),
            InputSel::None,
        ),
        (4, NodeParams::Out, InputSel::Slot(3), InputSel::None),
    ]
}

fn fm_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            NodeParams::Fm {
                carrier_table: TableSel::Stock(StockTable::Harmonics),
                modulator_table: TableSel::Stock(StockTable::Harmonics),
                carrier_position: 0.3,
                modulator_position: 0.3,
                semitones: 0.0,
                detune: 0.0,
                amount: 0.3,
                carrier_ratio: 1,
                modulator_ratio: 2,
                exponential: true,
            },
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            NodeParams::Env {
                attack: 8.0,
                decay: 300.0,
                sustain: 0.8,
                release: 400.0,
            },
            InputSel::Slot(0),
            InputSel::None,
        ),
        (
            2,
            NodeParams::Gain { gain: 0.5 },
            InputSel::Slot(1),
            InputSel::None,
        ),
        (3, NodeParams::Out, InputSel::Slot(2), InputSel::None),
    ]
}

pub fn factory_presets() -> Vec<Preset> {
    vec![
        Preset {
            name: "Init",
            cells: init_cells(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Pluck",
            cells: pluck_cells(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Pad",
            cells: pad_cells(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Bass",
            cells: bass_cells(),
            play_mode: PlayMode::MonoLegato,
        },
        Preset {
            name: "FM Bells",
            cells: fm_cells(),
            play_mode: PlayMode::Poly,
        },
    ]
}

pub fn default_patch() -> Patch {
    // Spawn with the Init patch (osc → filter → env → gain → out) so the
    // envelope is wired in from the very first load.
    let mut grid = Grid::new();
    for (slot, params, in0, in1) in init_cells() {
        grid.set(slot, Some(params), in0, in1);
    }
    Patch::new(grid)
}
