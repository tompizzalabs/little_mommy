use crate::engine::graph::CompiledGraph;
use crate::engine::osc::analog::AnalogVoiceState;
use crate::engine::osc::{
    AnalogShape, DetuneMode, DriftGenerator, FmState, NoiseColor, NoiseState, OperatorWave,
    PanMode, PmState, StackMode, SyncIn, UnisonConfig, UnisonEngine, VectorState,
};
use crate::voice::Voice;

const SAMPLE_RATE: f32 = 48_000.0;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn hash_mono(samples: impl IntoIterator<Item = f32>) -> u64 {
    let mut hash = FNV_OFFSET;
    for sample in samples {
        hash ^= sample.to_bits() as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn hash_stereo(samples: impl IntoIterator<Item = (f32, f32)>) -> u64 {
    let mut hash = FNV_OFFSET;
    for (left, right) in samples {
        hash ^= left.to_bits() as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= right.to_bits() as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn render_oscillator(
    shape: AnalogShape,
    frequency: f32,
    pulse_width: f32,
    character: f32,
    morph: f32,
    samples: usize,
) -> u64 {
    let mut oscillator = AnalogVoiceState::default();
    oscillator.reseed(11);
    hash_mono((0..samples).map(|_| {
        oscillator.next(
            shape, frequency, SAMPLE_RATE, pulse_width, character, morph, None,
        )
    }))
}

fn render_synced_slaves(samples: usize) -> u64 {
    let mut master = AnalogVoiceState::default();
    let mut slave = AnalogVoiceState::default();
    master.reseed(21);
    slave.reseed(22);
    hash_mono((0..samples).map(|_| {
        master.next(AnalogShape::Saw, 440.0, SAMPLE_RATE, 0.5, 0.0, 0.5, None);
        let master_out = master.sync_out();
        let remaining = 1.0 - master_out.phase;
        let sync = if remaining <= master_out.increment {
            Some(SyncIn {
                time: (remaining / master_out.increment).clamp(1e-6, 1.0),
                depth: 1.0,
                threshold: 0.0,
            })
        } else {
            None
        };
        slave.next(AnalogShape::Saw, 200.0, SAMPLE_RATE, 0.5, 0.0, 0.5, sync)
    }))
}

fn render_unison(config: UnisonConfig, shape: AnalogShape, samples: usize) -> u64 {
    let mut state = crate::engine::dsp::NodeState::default();
    hash_stereo((0..samples).map(|_| {
        state.analog(shape, 220.0, SAMPLE_RATE, 0.5, 0.3, 0.5, &config, None)
    }))
}

fn render_unison_with_retrigger(config: UnisonConfig, shape: AnalogShape, samples: usize) -> u64 {
    let mut state = crate::engine::dsp::NodeState::default();
    hash_stereo((0..samples).map(|sample_index| {
        if sample_index % 480 == 0 {
            state.retrigger_unison();
        }
        state.analog(shape, 220.0, SAMPLE_RATE, 0.5, 0.3, 0.5, &config, None)
    }))
}

fn render_fm(exponential: bool, samples: usize) -> u64 {
    let mut state = FmState::default();
    state.seed(33, 2);
    hash_mono((0..samples).map(|_| {
        state.next(
            OperatorWave::Saw,
            OperatorWave::Sine,
            220.0,
            SAMPLE_RATE,
            0.4,
            2.0,
            7.0,
            exponential,
            0.3,
        )
    }))
}

fn render_pm(samples: usize) -> u64 {
    let mut state = PmState::default();
    state.seed(44, 3);
    hash_mono((0..samples).map(|_| {
        state.next(
            OperatorWave::Sine,
            OperatorWave::Triangle,
            330.0,
            SAMPLE_RATE,
            0.5,
            1.0,
            3.0,
            0.2,
        )
    }))
}

fn render_vector(samples: usize) -> u64 {
    let mut state = VectorState::default();
    state.seed(55);
    hash_mono((0..samples).map(|sample_index| {
        let sweep = sample_index as f32 / samples as f32;
        state.next(180.0, SAMPLE_RATE, sweep, 1.0 - sweep, 0.4)
    }))
}

fn render_noise(color: NoiseColor, frequency: f32, samples: usize) -> u64 {
    let mut state = NoiseState::default();
    state.seed(9);
    hash_mono((0..samples).map(|_| state.sample(color, frequency, SAMPLE_RATE)))
}

fn render_drift(samples: usize) -> u64 {
    let mut drift = DriftGenerator::new(77);
    hash_mono((0..samples).map(|_| {
        let (wander, thermal) = drift.next(SAMPLE_RATE);
        wander + thermal
    }))
}

fn render_unison_frames(config: UnisonConfig, detune: f32, samples: usize) -> u64 {
    let mut engine = UnisonEngine::new(1);
    engine.sync(config, SAMPLE_RATE);
    hash_stereo((0..samples).map(|sample_index| {
        let voice_index = sample_index % engine.voice_count();
        let frame = engine.frame(voice_index, detune, crate::engine::osc::MAX_DETUNE_CENTS);
        (frame.gain_left * frame.pitch_multiplier, frame.gain_right)
    }))
}

fn render_voice_patch(name: &str, samples: usize) -> u64 {
    let preset = crate::presets::factory_presets()
        .into_iter()
        .find(|preset| preset.name == name)
        .unwrap_or_else(|| panic!("{name} should be a factory preset"));
    let compiled = CompiledGraph::compile(&preset.into_patch().grid.to_graph());
    let mut voice = Voice::new(5, 60, 0.8, compiled.nodes.len());
    hash_stereo((0..samples).map(|_| voice.render(&compiled, SAMPLE_RATE, 120.0)))
}

fn render_all_cases() -> Vec<(&'static str, u64)> {
    vec![
        ("osc saw", render_oscillator(AnalogShape::Saw, 220.0, 0.5, 0.3, 0.5, 2_048)),
        ("osc reverse saw", render_oscillator(AnalogShape::ReverseSaw, 220.0, 0.5, 0.3, 0.5, 2_048)),
        ("osc square", render_oscillator(AnalogShape::Square, 220.0, 0.35, 0.3, 0.5, 2_048)),
        ("osc square wide", render_oscillator(AnalogShape::Square, 1_800.0, 0.85, 0.8, 0.5, 2_048)),
        ("osc triangle", render_oscillator(AnalogShape::Triangle, 220.0, 0.5, 0.3, 0.5, 2_048)),
        ("osc sharktooth", render_oscillator(AnalogShape::Sharktooth, 220.0, 0.5, 0.6, 0.5, 2_048)),
        ("osc sine", render_oscillator(AnalogShape::Sine, 220.0, 0.5, 0.3, 0.5, 2_048)),
        ("osc sine high", render_oscillator(AnalogShape::Sine, 6_000.0, 0.5, 0.0, 0.5, 2_048)),
        ("osc morph start", render_oscillator(AnalogShape::Morph, 220.0, 0.5, 0.3, 0.0, 2_048)),
        ("osc morph end", render_oscillator(AnalogShape::Morph, 220.0, 0.5, 0.3, 1.0, 2_048)),
        ("osc saw sub", render_oscillator(AnalogShape::Saw, 27.5, 0.5, 1.0, 0.5, 2_048)),
        ("osc saw high", render_oscillator(AnalogShape::Saw, 7_000.0, 0.5, 0.0, 0.5, 2_048)),
        ("synced slaves", render_synced_slaves(4_096)),
        ("unison single", render_unison(
            UnisonConfig { voices: 1, ..UnisonConfig::default() },
            AnalogShape::Saw,
            2_048,
        )),
        ("unison five", render_unison(
            UnisonConfig {
                voices: 5,
                detune: 0.4,
                ..UnisonConfig::default()
            },
            AnalogShape::Saw,
            2_048,
        )),
        ("unison five swarm", render_unison(
            UnisonConfig {
                voices: 5,
                detune: 0.5,
                swarm: 0.4,
                swarm_rate_hz: 1.7,
                ..UnisonConfig::default()
            },
            AnalogShape::Saw,
            2_048,
        )),
        ("unison analog stack", render_unison(
            UnisonConfig {
                voices: 8,
                detune: 0.6,
                detune_mode: DetuneMode::Analog,
                pan_mode: PanMode::Outer,
                stack_mode: StackMode::OctavesAndFifths,
                phase_randomization: 1.0,
                ..UnisonConfig::default()
            },
            AnalogShape::Saw,
            2_048,
        )),
        ("unison random detune", render_unison(
            UnisonConfig {
                voices: 6,
                detune: 0.8,
                detune_mode: DetuneMode::Random,
                pan_mode: PanMode::Random,
                width: 0.7,
                ..UnisonConfig::default()
            },
            AnalogShape::Square,
            2_048,
        )),
        ("unison reverse saw", render_unison(
            UnisonConfig {
                voices: 3,
                detune: 0.3,
                ..UnisonConfig::default()
            },
            AnalogShape::ReverseSaw,
            2_048,
        )),
        ("unison sharktooth", render_unison(
            UnisonConfig {
                voices: 3,
                detune: 0.3,
                ..UnisonConfig::default()
            },
            AnalogShape::Sharktooth,
            2_048,
        )),
        ("unison retrigger", render_unison_with_retrigger(
            UnisonConfig {
                voices: 4,
                detune: 0.5,
                phase_randomization: 1.0,
                ..UnisonConfig::default()
            },
            AnalogShape::Saw,
            2_048,
        )),
        ("unison frames static", render_unison_frames(
            UnisonConfig {
                voices: 7,
                detune: 0.5,
                stack_mode: StackMode::Octaves,
                ..UnisonConfig::default()
            },
            0.5,
            4_096,
        )),
        ("unison frames swarm", render_unison_frames(
            UnisonConfig {
                voices: 7,
                detune: 0.5,
                swarm: 0.6,
                swarm_rate_hz: 2.5,
                ..UnisonConfig::default()
            },
            0.5,
            4_096,
        )),
        ("fm exponential", render_fm(true, 2_048)),
        ("fm linear", render_fm(false, 2_048)),
        ("pm", render_pm(2_048)),
        ("vector", render_vector(2_048)),
        ("noise white audio", render_noise(NoiseColor::White, 20_000.0, 2_048)),
        ("noise pink slow", render_noise(NoiseColor::Pink, 5.0, 2_048)),
        ("noise brown", render_noise(NoiseColor::Brown, 400.0, 2_048)),
        ("drift", render_drift(8_192)),
        ("voice init", render_voice_patch("Init", 4_096)),
        ("voice wu wei", render_voice_patch("Wu Wei", 4_096)),
        ("voice valley spirit", render_voice_patch("Valley Spirit", 4_096)),
        ("voice river of stars", render_voice_patch("River of Stars", 4_096)),
    ]
}

fn recorded_hashes() -> &'static [(&'static str, u64)] {
    &[
        ("osc saw", 0x3bf8_2e87_34e7_745c),
        ("osc reverse saw", 0xf767_6f83_34e7_745c),
        ("osc square", 0xd989_3b85_cd3e_47f0),
        ("osc square wide", 0x13b3_5ff1_c9aa_e8da),
        ("osc triangle", 0x493a_f80e_aebd_db96),
        ("osc sharktooth", 0x89e4_166e_a899_31e4),
        ("osc sine", 0xf162_38f4_37ce_41f1),
        ("osc sine high", 0xc988_0e91_db13_c5c6),
        ("osc morph start", 0xe327_cdb8_bbce_0242),
        ("osc morph end", 0x6b81_38e2_4f77_71f9),
        ("osc saw sub", 0x3154_6eb2_74e7_2125),
        ("osc saw high", 0x067f_99a1_648b_7e35),
        ("synced slaves", 0x7ca8_f2d7_f984_c5ff),
        ("unison single", 0xc6e8_f3e7_01a5_0c2f),
        ("unison five", 0x51b7_1463_3fb0_8531),
        ("unison five swarm", 0xe46a_f03c_ba75_d6d1),
        ("unison analog stack", 0xac07_8496_4910_0eb7),
        ("unison random detune", 0xb108_e566_2ecb_b20b),
        ("unison reverse saw", 0x9407_ee89_d9e5_3b61),
        ("unison sharktooth", 0xb02c_e347_061b_480a),
        ("unison retrigger", 0x5385_0b4e_dcc6_fb2a),
        ("unison frames static", 0xf50e_75be_f3b2_9f97),
        ("unison frames swarm", 0xd003_4543_ba11_17e6),
        ("fm exponential", 0xeec6_67f7_bef4_a0ad),
        ("fm linear", 0x5395_47f1_6f74_babb),
        ("pm", 0x789d_b739_b594_4875),
        ("vector", 0x996f_2f7f_f4fa_220f),
        ("noise white audio", 0xe9a7_3921_9155_8ee8),
        ("noise pink slow", 0x28c3_1cf8_df2e_c325),
        ("noise brown", 0xa666_acf8_71ac_1852),
        ("drift", 0x3a20_c982_cff6_ff44),
        ("voice init", 0xfde0_121c_94d7_2135),
        ("voice wu wei", 0x8afe_6d68_d7dd_9c9f),
        ("voice valley spirit", 0xbf2b_1c9f_aaf4_a653),
        ("voice river of stars", 0xb554_2fad_e69a_28b6),
    ]
}

#[test]
fn oscillator_output_matches_golden_references() {
    let rendered = render_all_cases();
    let failures: Vec<String> = rendered
        .iter()
        .filter_map(|(name, actual)| {
            let expected = recorded_hashes()
                .iter()
                .find(|(recorded_name, _)| recorded_name == name)
                .map(|(_, hash)| *hash);
            match expected {
                Some(value) if value == *actual => None,
                Some(value) => {
                    Some(format!("{name}: expected {value:#018x}, got {actual:#018x}"))
                }
                None => Some(format!("{name}: {actual:#018x}")),
            }
        })
        .collect();
    assert!(
        failures.is_empty(),
        "golden drift detected:\n{}",
        failures.join("\n")
    );
}
