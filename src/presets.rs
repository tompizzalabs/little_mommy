use crate::engine::graph::{Grid, InputSel, ModLink, Patch, PlayMode};
use crate::engine::lfo::LfoShape;
use crate::engine::node::NodeParams;
use crate::engine::osc::{
    AnalogShape, NoiseColor, OperatorWave, OscillatorModel, SyncMode, unison,
};

pub struct Preset {
    pub name: &'static str,
    pub author: &'static str,
    pub category: &'static str,
    pub cells: Vec<(usize, NodeParams, InputSel, InputSel)>,
    pub mods: Vec<(usize, ModLink)>,
    pub play_mode: PlayMode,
}

impl Preset {
    pub fn into_patch(&self) -> Patch {
        let mut patch = default_patch();
        patch.grid = Grid::new();
        for (slot, params, in0, in1) in &self.cells {
            patch.grid.set(*slot, Some(params.clone()), *in0, *in1);
        }
        for (slot, link) in &self.mods {
            if let Some(cell) = patch.grid.cells.get_mut(*slot) {
                cell.mods.push(link.clone());
            }
        }
        patch.play_mode = self.play_mode;
        patch
    }
}

fn analog(shape: AnalogShape, semitones: f32, detune: f32, drift: f32) -> NodeParams {
    analog_stack(shape, semitones, detune, drift, 1, 0.0, 0.0)
}

#[allow(clippy::too_many_arguments)]
fn analog_stack(
    shape: AnalogShape,
    semitones: f32,
    detune: f32,
    drift: f32,
    voices: u8,
    unison_detune: f32,
    swarm: f32,
) -> NodeParams {
    NodeParams::Analog {
        shape,
        model: OscillatorModel::Discrete,
        semitones,
        detune,
        pulse_width: 0.5,
        morph: 0.0,
        drift,
        unison_voices: voices,
        unison_detune,
        unison_detune_power: 1.0,
        unison_detune_mode: unison::DetuneMode::Power,
        unison_pan_mode: unison::PanMode::Linear,
        unison_stack_mode: unison::StackMode::Unison,
        unison_width: 1.0,
        unison_blend: 0.0,
        unison_swarm: swarm,
        unison_swarm_rate: 0.15,
        unison_phase_randomization: 0.0,
        sync_mode: SyncMode::Off,
        sync_source: 0,
        sync_depth: 0.6,
    }
}

fn low_pass(cutoff: f32, resonance: f32) -> NodeParams {
    NodeParams::Filter {
        kind: crate::engine::filter::FilterKind::StateVariable,
        cutoff,
        resonance,
        transition: 0.5,
        vowel_left: 0.0,
        vowel_right: 1.0,
        saturation: 0.0,
    }
}

fn ladder(cutoff: f32, resonance: f32) -> NodeParams {
    NodeParams::Filter {
        kind: crate::engine::filter::FilterKind::Ladder4LowPass,
        cutoff,
        resonance,
        transition: 0.5,
        vowel_left: 0.0,
        vowel_right: 1.0,
        saturation: 0.0,
    }
}

fn envelope(attack: f32, decay: f32, sustain: f32, release: f32) -> NodeParams {
    NodeParams::Env {
        attack,
        decay,
        sustain,
        release,
    }
}

fn level(gain: f32) -> NodeParams {
    NodeParams::Gain { gain }
}

fn lfo(shape: LfoShape, rate: f32) -> NodeParams {
    NodeParams::Lfo {
        shape,
        rate,
        phase: 0.0,
    }
}

fn cutoff_sweep(source: u32, depth: f32) -> (usize, ModLink) {
    (
        1,
        ModLink {
            param: 0,
            source,
            depth,
        },
    )
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
                kind: crate::engine::filter::FilterKind::StateVariable,
                cutoff: 8000.0,
                resonance: 0.2,
                transition: 0.5,
                vowel_left: 0.0,
                vowel_right: 1.0,
                saturation: 0.0,
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
                kind: crate::engine::filter::FilterKind::StateVariable,
                cutoff: 3000.0,
                resonance: 0.35,
                transition: 0.5,
                vowel_left: 0.0,
                vowel_right: 1.0,
                saturation: 0.0,
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
            NodeParams::Analog {
                shape: AnalogShape::Saw,
                model: crate::engine::osc::OscillatorModel::Discrete,
                semitones: 0.0,
                detune: -7.0,
                pulse_width: 0.5,
                morph: 0.0,
                drift: 0.0,
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
                sync_mode: crate::engine::osc::SyncMode::Off,
                sync_source: 0,
                sync_depth: 0.6,
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
                kind: crate::engine::filter::FilterKind::StateVariable,
                cutoff: 2200.0,
                resonance: 0.15,
                transition: 0.5,
                vowel_left: 0.0,
                vowel_right: 1.0,
                saturation: 0.0,
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
                kind: crate::engine::filter::FilterKind::StateVariable,
                cutoff: 450.0,
                resonance: 0.3,
                transition: 0.5,
                vowel_left: 0.0,
                vowel_right: 1.0,
                saturation: 0.0,
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
                carrier_wave: OperatorWave::Sine,
                modulator_wave: OperatorWave::Sine,
                semitones: 0.0,
                detune: 0.0,
                amount: 0.5,
                carrier_ratio: 1,
                modulator_ratio: 12,
                exponential: true,
                drift: 0.0,
            },
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            NodeParams::Env {
                attack: 8.0,
                decay: 450.0,
                sustain: 0.0,
                release: 800.0,
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

fn vector_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            NodeParams::Vector {
                semitones: 0.0,
                detune: 0.0,
                x: 0.35,
                y: 0.4,
                drift: 0.3,
            },
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            NodeParams::Filter {
                kind: crate::engine::filter::FilterKind::StateVariable,
                cutoff: 3500.0,
                resonance: 0.2,
                transition: 0.5,
                vowel_left: 0.0,
                vowel_right: 1.0,
                saturation: 0.0,
            },
            InputSel::Slot(0),
            InputSel::None,
        ),
        (
            2,
            NodeParams::Env {
                attack: 60.0,
                decay: 400.0,
                sustain: 0.7,
                release: 500.0,
            },
            InputSel::Slot(1),
            InputSel::None,
        ),
        (
            3,
            NodeParams::Gain { gain: 0.55 },
            InputSel::Slot(2),
            InputSel::None,
        ),
        (4, NodeParams::Out, InputSel::Slot(3), InputSel::None),
    ]
}

fn dream_pluck_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog_stack(AnalogShape::Saw, 0.0, 0.0, 0.3, 2, 0.15, 0.0),
            InputSel::None,
            InputSel::None,
        ),
        (1, low_pass(2600.0, 0.3), InputSel::Slot(0), InputSel::None),
        (
            2,
            envelope(2.0, 260.0, 0.0, 450.0),
            InputSel::Slot(1),
            InputSel::None,
        ),
        (3, level(0.55), InputSel::Slot(2), InputSel::None),
        (4, NodeParams::Out, InputSel::Slot(3), InputSel::None),
    ]
}

fn crystal_pluck_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog_stack(AnalogShape::Triangle, 12.0, 0.0, 0.2, 1, 0.0, 0.0),
            InputSel::None,
            InputSel::None,
        ),
        (1, low_pass(4200.0, 0.55), InputSel::Slot(0), InputSel::None),
        (
            2,
            envelope(1.0, 140.0, 0.0, 320.0),
            InputSel::Slot(1),
            InputSel::None,
        ),
        (3, level(0.6), InputSel::Slot(2), InputSel::None),
        (4, NodeParams::Out, InputSel::Slot(3), InputSel::None),
    ]
}

fn neon_dusk_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog_stack(AnalogShape::Saw, 0.0, 0.0, 0.4, 5, 0.5, 0.3),
            InputSel::None,
            InputSel::None,
        ),
        (1, lfo(LfoShape::Sine, 0.09), InputSel::None, InputSel::None),
        (2, low_pass(1700.0, 0.25), InputSel::Slot(0), InputSel::None),
        (
            3,
            envelope(800.0, 500.0, 0.85, 1600.0),
            InputSel::Slot(2),
            InputSel::None,
        ),
        (4, level(0.5), InputSel::Slot(3), InputSel::None),
        (5, NodeParams::Out, InputSel::Slot(4), InputSel::None),
    ]
}

fn neon_dusk_mods() -> Vec<(usize, ModLink)> {
    vec![cutoff_sweep(1, 0.3)]
}

fn chrome_dreams_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog_stack(AnalogShape::Saw, 0.0, -5.0, 0.3, 3, 0.35, 0.2),
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            analog_stack(AnalogShape::Triangle, -12.0, 0.0, 0.4, 1, 0.0, 0.0),
            InputSel::None,
            InputSel::None,
        ),
        (2, NodeParams::Mix, InputSel::Slot(0), InputSel::Slot(1)),
        (
            3,
            lfo(LfoShape::Triangle, 0.14),
            InputSel::None,
            InputSel::None,
        ),
        (4, low_pass(2300.0, 0.2), InputSel::Slot(2), InputSel::None),
        (
            5,
            envelope(450.0, 400.0, 0.8, 1300.0),
            InputSel::Slot(4),
            InputSel::None,
        ),
        (6, level(0.5), InputSel::Slot(5), InputSel::None),
        (7, NodeParams::Out, InputSel::Slot(6), InputSel::None),
    ]
}

fn chrome_dreams_mods() -> Vec<(usize, ModLink)> {
    vec![cutoff_sweep(3, 0.35)]
}

fn miami_mist_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog_stack(AnalogShape::Saw, 0.0, 0.0, 0.6, 7, 0.3, 0.45),
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            NodeParams::Noise {
                color: NoiseColor::Pink,
                level: 0.12,
            },
            InputSel::None,
            InputSel::None,
        ),
        (2, NodeParams::Mix, InputSel::Slot(0), InputSel::Slot(1)),
        (3, low_pass(2900.0, 0.18), InputSel::Slot(2), InputSel::None),
        (
            4,
            envelope(650.0, 600.0, 0.75, 2100.0),
            InputSel::Slot(3),
            InputSel::None,
        ),
        (5, level(0.48), InputSel::Slot(4), InputSel::None),
        (6, NodeParams::Out, InputSel::Slot(5), InputSel::None),
    ]
}

fn nightdrive_bass_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog_stack(AnalogShape::Square, -12.0, 0.0, 0.15, 1, 0.0, 0.0),
            InputSel::None,
            InputSel::None,
        ),
        (1, ladder(680.0, 0.4), InputSel::Slot(0), InputSel::None),
        (
            2,
            envelope(3.0, 220.0, 0.6, 140.0),
            InputSel::Slot(1),
            InputSel::None,
        ),
        (3, level(0.7), InputSel::Slot(2), InputSel::None),
        (4, NodeParams::Out, InputSel::Slot(3), InputSel::None),
    ]
}

fn cyber_pulse_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog_stack(AnalogShape::Saw, -12.0, 0.0, 0.2, 2, 0.1, 0.0),
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            lfo(LfoShape::Triangle, 3.5),
            InputSel::None,
            InputSel::None,
        ),
        (2, low_pass(950.0, 0.45), InputSel::Slot(0), InputSel::None),
        (
            3,
            envelope(2.0, 160.0, 0.55, 120.0),
            InputSel::Slot(2),
            InputSel::None,
        ),
        (4, level(0.65), InputSel::Slot(3), InputSel::None),
        (5, NodeParams::Out, InputSel::Slot(4), InputSel::None),
    ]
}

fn cyber_pulse_mods() -> Vec<(usize, ModLink)> {
    vec![cutoff_sweep(1, 0.12)]
}

fn glass_keys_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            NodeParams::Fm {
                carrier_wave: OperatorWave::Sine,
                modulator_wave: OperatorWave::Sine,
                semitones: 0.0,
                detune: 0.0,
                amount: 0.35,
                carrier_ratio: 1,
                modulator_ratio: 4,
                exponential: true,
                drift: 0.0,
            },
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            envelope(2.0, 900.0, 0.15, 600.0),
            InputSel::Slot(0),
            InputSel::None,
        ),
        (2, level(0.55), InputSel::Slot(1), InputSel::None),
        (3, NodeParams::Out, InputSel::Slot(2), InputSel::None),
    ]
}

fn dream_celeste_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            NodeParams::Fm {
                carrier_wave: OperatorWave::Sine,
                modulator_wave: OperatorWave::Triangle,
                semitones: 0.0,
                detune: 0.0,
                amount: 0.2,
                carrier_ratio: 1,
                modulator_ratio: 2,
                exponential: true,
                drift: 0.0,
            },
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            analog_stack(AnalogShape::Sine, 12.0, 0.0, 0.3, 1, 0.0, 0.0),
            InputSel::None,
            InputSel::None,
        ),
        (2, NodeParams::Mix, InputSel::Slot(0), InputSel::Slot(1)),
        (
            3,
            envelope(4.0, 700.0, 0.1, 900.0),
            InputSel::Slot(2),
            InputSel::None,
        ),
        (4, level(0.5), InputSel::Slot(3), InputSel::None),
        (5, NodeParams::Out, InputSel::Slot(4), InputSel::None),
    ]
}

fn vapor_lead_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            analog_stack(AnalogShape::Saw, 0.0, 0.0, 0.25, 3, 0.2, 0.0),
            InputSel::None,
            InputSel::None,
        ),
        (1, low_pass(3600.0, 0.25), InputSel::Slot(0), InputSel::None),
        (
            2,
            envelope(12.0, 320.0, 0.8, 280.0),
            InputSel::Slot(1),
            InputSel::None,
        ),
        (3, level(0.55), InputSel::Slot(2), InputSel::None),
        (4, NodeParams::Out, InputSel::Slot(3), InputSel::None),
    ]
}

fn mermaid_bells_cells() -> Vec<(usize, NodeParams, InputSel, InputSel)> {
    vec![
        (
            0,
            NodeParams::Pm {
                carrier_wave: OperatorWave::Sine,
                modulator_wave: OperatorWave::Sine,
                semitones: 0.0,
                detune: 0.0,
                amount: 0.45,
                carrier_ratio: 1,
                modulator_ratio: 3,
                drift: 0.0,
            },
            InputSel::None,
            InputSel::None,
        ),
        (
            1,
            envelope(1.0, 750.0, 0.0, 1100.0),
            InputSel::Slot(0),
            InputSel::None,
        ),
        (2, level(0.5), InputSel::Slot(1), InputSel::None),
        (3, NodeParams::Out, InputSel::Slot(2), InputSel::None),
    ]
}

pub fn factory_presets() -> Vec<Preset> {
    const AUTHOR: &str = "Tom Pizza Labs";
    vec![
        Preset {
            name: "Init",
            author: AUTHOR,
            category: "Init",
            cells: init_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Pluck",
            author: AUTHOR,
            category: "Plucks",
            cells: pluck_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Dream Pluck",
            author: AUTHOR,
            category: "Plucks",
            cells: dream_pluck_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Crystal Pluck",
            author: AUTHOR,
            category: "Plucks",
            cells: crystal_pluck_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Pad",
            author: AUTHOR,
            category: "Pads",
            cells: pad_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Neon Dusk",
            author: AUTHOR,
            category: "Pads",
            cells: neon_dusk_cells(),
            mods: neon_dusk_mods(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Chrome Dreams",
            author: AUTHOR,
            category: "Pads",
            cells: chrome_dreams_cells(),
            mods: chrome_dreams_mods(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Miami Mist",
            author: AUTHOR,
            category: "Pads",
            cells: miami_mist_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Bass",
            author: AUTHOR,
            category: "Basses",
            cells: bass_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::MonoLegato,
        },
        Preset {
            name: "Nightdrive Bass",
            author: AUTHOR,
            category: "Basses",
            cells: nightdrive_bass_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::MonoLegato,
        },
        Preset {
            name: "Cyber Pulse",
            author: AUTHOR,
            category: "Basses",
            cells: cyber_pulse_cells(),
            mods: cyber_pulse_mods(),
            play_mode: PlayMode::MonoRetrig,
        },
        Preset {
            name: "FM Bells",
            author: AUTHOR,
            category: "Keys",
            cells: fm_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Glass Keys",
            author: AUTHOR,
            category: "Keys",
            cells: glass_keys_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Dream Celeste",
            author: AUTHOR,
            category: "Keys",
            cells: dream_celeste_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Vector",
            author: AUTHOR,
            category: "Misc",
            cells: vector_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Vapor Lead",
            author: AUTHOR,
            category: "Misc",
            cells: vapor_lead_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
        Preset {
            name: "Mermaid Bells",
            author: AUTHOR,
            category: "Misc",
            cells: mermaid_bells_cells(),
            mods: Vec::new(),
            play_mode: PlayMode::Poly,
        },
    ]
}

pub fn default_patch() -> Patch {
    let mut grid = Grid::new();
    for (slot, params, in0, in1) in init_cells() {
        grid.set(slot, Some(params), in0, in1);
    }
    Patch::new(grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_factory_preset_round_trips_through_json() {
        for preset in factory_presets() {
            let file = crate::preset_bank::PresetFile {
                format: crate::preset_bank::PRESET_FORMAT_VERSION,
                name: preset.name.to_owned(),
                author: preset.author.to_owned(),
                category: preset.category.to_owned(),
                patch: preset.into_patch(),
            };
            let json = serde_json::to_string(&file).expect("serialization should work");
            let restored: crate::preset_bank::PresetFile =
                serde_json::from_str(&json).expect("parsing should work");
            assert_eq!(restored.name, preset.name);
            assert_eq!(restored.patch.play_mode, preset.play_mode);
        }
    }

    #[test]
    fn mod_links_reference_occupied_cells() {
        for preset in factory_presets() {
            for (slot, _) in &preset.mods {
                assert!(
                    preset
                        .cells
                        .iter()
                        .any(|(cell_slot, _, _, _)| cell_slot == slot),
                    "{} mod targets empty slot {slot}",
                    preset.name
                );
            }
        }
    }
}
