use crate::engine::dsp::{EnvStage, NodeState, cent_ratio, semitone_ratio};
use crate::engine::graph::{CompiledGraph, ModLink};
use crate::engine::node::NodeParams;
use crate::engine::osc::unison::UnisonConfig;
use crate::engine::osc::{SyncIn, SyncMode, SyncOut};
use nice_plug::util;

const STEAL_FADE_TIME_MS: f32 = 50.0;

const NOISE_SEED_MIX: u32 = 0x9E37_79B9;
const NOISE_NODE_INDEX_MIX: u32 = 0x85EB_CA6B;
const LFO_SEED_MIX: u32 = 0x2545_F491;
const LFO_NODE_INDEX_MIX: u32 = 7;
const LFO_PHASE_SPREAD: u32 = 991;
const LFO_VALUE_SPREAD: u32 = 401;
const UNISON_SEED_MIX: u32 = 0x9E37_79B9;
const UNISON_NODE_INDEX_MIX: u32 = 0x5BF0_3635;
const ANALOG_PHASE_SPREAD: u32 = 997;
const ANALOG_PHASE_VOICE_STEP: u32 = 13;
const ANALOG_SEED_MIX: u32 = 0xC2B2_AE35;
const ANALOG_NODE_INDEX_MIX: u32 = 0x27D4_EB2F;
const ANALOG_UNISON_MIX: u32 = 0x1656_67B1;
const VECTOR_SEED_MIX: u32 = 0x6A09_E667;
const VECTOR_NODE_INDEX_MIX: u32 = 0xBB67_AE85;

fn unison_config(params: &NodeParams) -> UnisonConfig {
    match params {
        NodeParams::Analog {
            unison_voices,
            unison_detune,
            unison_detune_power,
            unison_detune_mode,
            unison_pan_mode,
            unison_stack_mode,
            unison_width,
            unison_blend,
            unison_swarm,
            unison_swarm_rate,
            unison_phase_randomization,
            ..
        } => UnisonConfig {
            voices: *unison_voices,
            detune: *unison_detune,
            detune_power: *unison_detune_power,
            detune_mode: *unison_detune_mode,
            pan_mode: *unison_pan_mode,
            stack_mode: *unison_stack_mode,
            width: *unison_width,
            blend: *unison_blend,
            swarm: *unison_swarm,
            swarm_rate_hz: *unison_swarm_rate,
            phase_randomization: *unison_phase_randomization,
        },
        _ => UnisonConfig::default(),
    }
}

#[derive(Debug, Clone)]
pub struct Voice {
    pub internal_voice_id: u64,
    pub note_number: u8,
    pub velocity_sqrt: f32,
    pub releasing: bool,
    pub env_active: bool,
    pub stealing: bool,
    pub steal_gain: f32,
    pub states: Box<[NodeState]>,
    outputs_l: Vec<f32>,
    outputs_r: Vec<f32>,
    mod_values: Vec<f32>,
}

fn mod_offset(links: &[ModLink], param: usize, mods: &[f32]) -> f32 {
    let mut sum = 0.0;
    for link in links {
        if link.param as usize == param {
            sum += mods[link.source as usize] * link.depth;
        }
    }
    sum
}

fn modulated(params: &NodeParams, links: &[ModLink], mods: &[f32], param_index: usize) -> f32 {
    let offset = mod_offset(links, param_index, mods);
    params.apply_modulation(param_index, offset)
}

fn sync_event(
    master: Option<SyncOut>,
    mode: SyncMode,
    depth: f32,
    slave_threshold: f32,
) -> Option<SyncIn> {
    let master = master?;
    if mode == SyncMode::Off || master.increment <= 1e-12 {
        return None;
    }
    let remaining = 1.0 - master.phase;
    if remaining > master.increment {
        return None;
    }
    let time = (remaining / master.increment).clamp(1e-6, 1.0);
    let (depth, threshold) = match mode {
        SyncMode::Hard => (1.0, 0.0),
        SyncMode::Soft => (depth.clamp(0.0, 1.0), slave_threshold),
        SyncMode::Off => return None,
    };
    Some(SyncIn {
        time,
        depth,
        threshold,
    })
}

impl Voice {
    pub fn new(internal_voice_id: u64, note_number: u8, velocity: f32, num_nodes: usize) -> Self {
        let voice_seed = internal_voice_id as u32;
        let mut states: Box<[NodeState]> = vec![NodeState::default(); num_nodes].into_boxed_slice();

        for (node_index, state) in states.iter_mut().enumerate() {
            let node_index_seed = node_index as u32;
            state.noise.seed(
                voice_seed
                    .wrapping_mul(NOISE_SEED_MIX)
                    .wrapping_add(node_index_seed.wrapping_mul(NOISE_NODE_INDEX_MIX)),
            );
            state.lfo.rng = fastrand::Rng::with_seed(
                voice_seed.wrapping_mul(LFO_SEED_MIX).wrapping_add(
                    node_index_seed
                        .wrapping_mul(LFO_NODE_INDEX_MIX)
                        .wrapping_add(1),
                ) as u64,
            );
            state.lfo.read_index = ((voice_seed.wrapping_add(node_index_seed) % LFO_PHASE_SPREAD)
                as f32)
                / LFO_PHASE_SPREAD as f32
                * 512.0;
            state.lfo.sample_hold_value = ((voice_seed
                .wrapping_add(node_index_seed.wrapping_mul(3))
                % LFO_VALUE_SPREAD) as f32)
                / LFO_VALUE_SPREAD as f32
                * 2.0
                - 1.0;
            state.unison.seed(
                voice_seed
                    .wrapping_mul(UNISON_SEED_MIX)
                    .wrapping_add(node_index_seed.wrapping_mul(UNISON_NODE_INDEX_MIX))
                    as u64,
            );
            state.fm.seed(voice_seed, node_index_seed);
            state.pm.seed(voice_seed, node_index_seed);
            state.vector.seed(
                voice_seed
                    .wrapping_mul(VECTOR_SEED_MIX)
                    .wrapping_add(node_index_seed.wrapping_mul(VECTOR_NODE_INDEX_MIX)),
            );
            for (unison_index, analog_voice) in state.analog.iter_mut().enumerate() {
                let unison_index_seed = unison_index as u32;
                analog_voice.reseed(
                    voice_seed
                        .wrapping_mul(ANALOG_SEED_MIX)
                        .wrapping_add(node_index_seed.wrapping_mul(ANALOG_NODE_INDEX_MIX))
                        .wrapping_add(unison_index_seed.wrapping_mul(ANALOG_UNISON_MIX)),
                );
                analog_voice.set_phase(
                    ((voice_seed
                        .wrapping_add(node_index_seed)
                        .wrapping_add(unison_index_seed.wrapping_mul(ANALOG_PHASE_VOICE_STEP))
                        % ANALOG_PHASE_SPREAD) as f32)
                        / ANALOG_PHASE_SPREAD as f32,
                );
            }
            state.env_stage = EnvStage::Attack;
            state.env_value = 0.0;
        }

        Self {
            internal_voice_id,
            note_number,
            velocity_sqrt: velocity.clamp(0.0, 1.0).sqrt(),
            releasing: false,
            env_active: true,
            stealing: false,
            steal_gain: 1.0,
            states,
            outputs_l: vec![0.0; num_nodes],
            outputs_r: vec![0.0; num_nodes],
            mod_values: vec![0.0; num_nodes],
        }
    }

    pub fn begin_steal(&mut self) {
        self.stealing = true;
    }

    pub fn is_finished(&self) -> bool {
        if self.stealing && self.steal_gain <= 0.0 {
            return true;
        }
        self.releasing && !self.env_active
    }

    pub fn render(&mut self, compiled: &CompiledGraph, sample_rate: f32) -> (f32, f32) {
        if compiled.nodes.is_empty() {
            return (0.0, 0.0);
        }

        if self.stealing {
            let fade_samples = STEAL_FADE_TIME_MS * 0.001 * sample_rate;
            let fade_step = 1.0 / fade_samples.max(1.0);
            self.steal_gain = (self.steal_gain - fade_step).max(0.0);
            if self.steal_gain <= 0.0 {
                return (0.0, 0.0);
            }
        }

        let base_freq = util::midi_key_to_freq(self.note_number);
        let node_count = compiled.nodes.len();
        if self.outputs_l.len() < node_count {
            self.outputs_l.resize(node_count, 0.0);
            self.outputs_r.resize(node_count, 0.0);
            self.mod_values.resize(node_count, 0.0);
        }
        let outputs_l = &mut self.outputs_l[..node_count];
        let outputs_r = &mut self.outputs_r[..node_count];
        let mod_values = &mut self.mod_values[..node_count];
        let mut env_present = false;
        let mut env_active = false;

        for &slot in compiled.order.iter() {
            let node = &compiled.nodes[slot];
            let first_input_left = node.inputs[0]
                .map(|source| outputs_l[source as usize])
                .unwrap_or(0.0);
            let first_input_right = node.inputs[0]
                .map(|source| outputs_r[source as usize])
                .unwrap_or(0.0);
            let second_input_left = node.inputs[1]
                .map(|source| outputs_l[source as usize])
                .unwrap_or(0.0);
            let second_input_right = node.inputs[1]
                .map(|source| outputs_r[source as usize])
                .unwrap_or(0.0);
            let sync_master = match &node.params {
                NodeParams::Analog { sync_source, .. } if (*sync_source as usize) < slot => self
                    .states
                    .get(*sync_source as usize)
                    .map(|master_state| master_state.sync_out),
                _ => None,
            };
            let state = &mut self.states[slot];
            let links = &node.mod_links;

            let (left, right) = match &node.params {
                NodeParams::Analog {
                    shape, sync_mode, ..
                } => {
                    let semitones = modulated(&node.params, links, mod_values, 0);
                    let detune = modulated(&node.params, links, mod_values, 1);
                    let pulse_width = modulated(&node.params, links, mod_values, 2);
                    let drift = modulated(&node.params, links, mod_values, 3);
                    let unison_detune = modulated(&node.params, links, mod_values, 4);
                    let unison_detune_power = modulated(&node.params, links, mod_values, 5);
                    let morph = modulated(&node.params, links, mod_values, 11);
                    let sync_depth = modulated(&node.params, links, mod_values, 12);
                    let frequency = base_freq * semitone_ratio(semitones) * cent_ratio(detune);
                    let mut config = unison_config(&node.params);
                    config.detune = unison_detune;
                    config.detune_power = unison_detune_power;
                    let slave_threshold = state.analog[0].soft_sync_threshold();
                    let sync = sync_event(sync_master, *sync_mode, sync_depth, slave_threshold);
                    state.analog(
                        *shape,
                        frequency,
                        sample_rate,
                        pulse_width,
                        drift,
                        morph,
                        &config,
                        sync,
                    )
                }
                NodeParams::Noise { color, .. } => {
                    let level = modulated(&node.params, links, mod_values, 0);
                    let (noise_left, noise_right) = state.noise(*color);
                    (noise_left * level, noise_right * level)
                }
                NodeParams::Fm { .. } => {
                    let semitones = modulated(&node.params, links, mod_values, 0);
                    let detune = modulated(&node.params, links, mod_values, 1);
                    let amount = modulated(&node.params, links, mod_values, 2);
                    let drift = modulated(&node.params, links, mod_values, 3);
                    let frequency = base_freq * semitone_ratio(semitones) * cent_ratio(detune);
                    if let NodeParams::Fm {
                        carrier_wave,
                        modulator_wave,
                        carrier_ratio,
                        modulator_ratio,
                        exponential,
                        ..
                    } = &node.params
                    {
                        state.fm(
                            *carrier_wave,
                            *modulator_wave,
                            frequency,
                            sample_rate,
                            amount,
                            *carrier_ratio as f32,
                            *modulator_ratio as f32,
                            *exponential,
                            drift,
                        )
                    } else {
                        (0.0, 0.0)
                    }
                }
                NodeParams::Pm { .. } => {
                    let semitones = modulated(&node.params, links, mod_values, 0);
                    let detune = modulated(&node.params, links, mod_values, 1);
                    let amount = modulated(&node.params, links, mod_values, 2);
                    let drift = modulated(&node.params, links, mod_values, 3);
                    let frequency = base_freq * semitone_ratio(semitones) * cent_ratio(detune);
                    if let NodeParams::Pm {
                        carrier_wave,
                        modulator_wave,
                        carrier_ratio,
                        modulator_ratio,
                        ..
                    } = &node.params
                    {
                        state.pm(
                            *carrier_wave,
                            *modulator_wave,
                            frequency,
                            sample_rate,
                            amount,
                            *carrier_ratio as f32,
                            *modulator_ratio as f32,
                            drift,
                        )
                    } else {
                        (0.0, 0.0)
                    }
                }
                NodeParams::Vector { .. } => {
                    let x = modulated(&node.params, links, mod_values, 0);
                    let y = modulated(&node.params, links, mod_values, 1);
                    let semitones = modulated(&node.params, links, mod_values, 2);
                    let detune = modulated(&node.params, links, mod_values, 3);
                    let drift = modulated(&node.params, links, mod_values, 4);
                    let frequency = base_freq * semitone_ratio(semitones) * cent_ratio(detune);
                    state.vector(frequency, sample_rate, x, y, drift)
                }
                NodeParams::Mix => (
                    first_input_left + second_input_left,
                    first_input_right + second_input_right,
                ),
                NodeParams::Filter { .. } => {
                    let cutoff = modulated(&node.params, links, mod_values, 0);
                    let resonance = modulated(&node.params, links, mod_values, 1);
                    let transition = modulated(&node.params, links, mod_values, 2) * 2.0 - 1.0;
                    let vowel_left = modulated(&node.params, links, mod_values, 3);
                    let vowel_right = modulated(&node.params, links, mod_values, 4);
                    let saturation = modulated(&node.params, links, mod_values, 5);
                    if let NodeParams::Filter { kind, .. } = &node.params {
                        state.filter(
                            *kind,
                            first_input_left,
                            first_input_right,
                            cutoff,
                            resonance,
                            transition,
                            vowel_left as usize,
                            vowel_right as usize,
                            saturation,
                            sample_rate,
                        )
                    } else {
                        (0.0, 0.0)
                    }
                }
                NodeParams::Env { .. } => {
                    let attack = modulated(&node.params, links, mod_values, 0);
                    let decay = modulated(&node.params, links, mod_values, 1);
                    let sustain = modulated(&node.params, links, mod_values, 2);
                    let release = modulated(&node.params, links, mod_values, 3);
                    let envelope =
                        state.env(sample_rate, attack, decay, sustain, release, self.releasing);
                    env_present = true;
                    if state.env_stage != EnvStage::Done {
                        env_active = true;
                    }
                    (first_input_left * envelope, first_input_right * envelope)
                }
                NodeParams::Gain { .. } => {
                    let gain = modulated(&node.params, links, mod_values, 0);
                    (first_input_left * gain, first_input_right * gain)
                }
                NodeParams::Lfo { shape, .. } => {
                    let rate = modulated(&node.params, links, mod_values, 0);
                    let phase = modulated(&node.params, links, mod_values, 1);
                    let value = state.lfo(*shape, rate, phase, sample_rate);
                    (value, value)
                }
                NodeParams::Delay { .. } => {
                    let delay_time = modulated(&node.params, links, mod_values, 0);
                    let feedback = modulated(&node.params, links, mod_values, 1);
                    let dry = modulated(&node.params, links, mod_values, 2);
                    let wet = modulated(&node.params, links, mod_values, 3);
                    let ducking = modulated(&node.params, links, mod_values, 4);
                    if let NodeParams::Delay { ping_pong, .. } = &node.params {
                        state.delay(
                            first_input_left,
                            first_input_right,
                            delay_time,
                            feedback,
                            dry,
                            wet,
                            ducking,
                            *ping_pong,
                            sample_rate,
                        )
                    } else {
                        (0.0, 0.0)
                    }
                }
                NodeParams::Chorus { .. } => {
                    let rate = modulated(&node.params, links, mod_values, 0);
                    let amount = modulated(&node.params, links, mod_values, 1);
                    let feedback = modulated(&node.params, links, mod_values, 2);
                    let dry_wet = modulated(&node.params, links, mod_values, 3);
                    (
                        state.chorus(
                            0,
                            first_input_left,
                            rate,
                            amount,
                            feedback,
                            dry_wet,
                            sample_rate,
                        ),
                        state.chorus(
                            1,
                            first_input_right,
                            rate,
                            amount,
                            feedback,
                            dry_wet,
                            sample_rate,
                        ),
                    )
                }
                NodeParams::Flanger { .. } => {
                    let rate = modulated(&node.params, links, mod_values, 0);
                    let amount = modulated(&node.params, links, mod_values, 1);
                    let feedback = modulated(&node.params, links, mod_values, 2);
                    let dry_wet = modulated(&node.params, links, mod_values, 3);
                    (
                        state.flanger(
                            0,
                            first_input_left,
                            rate,
                            amount,
                            feedback,
                            dry_wet,
                            sample_rate,
                        ),
                        state.flanger(
                            1,
                            first_input_right,
                            rate,
                            amount,
                            feedback,
                            dry_wet,
                            sample_rate,
                        ),
                    )
                }
                NodeParams::Phaser { .. } => {
                    let rate = modulated(&node.params, links, mod_values, 0);
                    let base_frequency = modulated(&node.params, links, mod_values, 1);
                    let amount = modulated(&node.params, links, mod_values, 2);
                    let feedback = modulated(&node.params, links, mod_values, 3);
                    let dry_wet = modulated(&node.params, links, mod_values, 4);
                    state.phaser(
                        first_input_left,
                        first_input_right,
                        rate,
                        base_frequency,
                        amount,
                        feedback,
                        dry_wet,
                        sample_rate,
                    )
                }
                NodeParams::Bitcrusher { .. } => {
                    let bits = modulated(&node.params, links, mod_values, 0);
                    let edge = modulated(&node.params, links, mod_values, 1);
                    let dry_wet = modulated(&node.params, links, mod_values, 2);
                    state.bitcrusher(first_input_left, first_input_right, bits, edge, dry_wet)
                }
                NodeParams::RingMod { .. } => {
                    let frequency = modulated(&node.params, links, mod_values, 0);
                    let amount = modulated(&node.params, links, mod_values, 1);
                    state.ring_mod(
                        first_input_left,
                        first_input_right,
                        frequency,
                        amount,
                        sample_rate,
                    )
                }
                NodeParams::Distortion { .. } => {
                    let threshold = modulated(&node.params, links, mod_values, 0);
                    let dry_wet = modulated(&node.params, links, mod_values, 1);
                    if let NodeParams::Distortion { algorithm, .. } = &node.params {
                        (
                            state.distortion(0, first_input_left, threshold, *algorithm, dry_wet),
                            state.distortion(1, first_input_right, threshold, *algorithm, dry_wet),
                        )
                    } else {
                        (0.0, 0.0)
                    }
                }
                NodeParams::Equalizer { .. } => {
                    let frequency = modulated(&node.params, links, mod_values, 0);
                    let q = modulated(&node.params, links, mod_values, 1);
                    let gain_db = modulated(&node.params, links, mod_values, 2);
                    (
                        state.equalizer(0, first_input_left, frequency, q, gain_db, sample_rate),
                        state.equalizer(1, first_input_right, frequency, q, gain_db, sample_rate),
                    )
                }
                NodeParams::Reverb { .. } => {
                    let pre_delay = modulated(&node.params, links, mod_values, 0);
                    let xover = modulated(&node.params, links, mod_values, 1);
                    let low_decay = modulated(&node.params, links, mod_values, 2);
                    let mid_decay = modulated(&node.params, links, mod_values, 3);
                    let damping = modulated(&node.params, links, mod_values, 4);
                    let mix = modulated(&node.params, links, mod_values, 5);
                    state.reverb(
                        first_input_left,
                        first_input_right,
                        pre_delay,
                        xover,
                        low_decay,
                        mid_decay,
                        damping,
                        mix,
                        sample_rate,
                    )
                }
                NodeParams::Out => (first_input_left, first_input_right),
            };

            outputs_l[slot] = left;
            outputs_r[slot] = right;

            mod_values[slot] = match &node.params {
                NodeParams::Env { .. } => state.env_value,
                NodeParams::Lfo { .. } => left,
                _ => (left + right) * 0.5,
            };
        }

        self.env_active = env_present && env_active;

        let mut output_left = 0.0;
        let mut output_right = 0.0;
        for (slot, node) in compiled.nodes.iter().enumerate() {
            if matches!(node.params, NodeParams::Out) {
                output_left += outputs_l[slot];
                output_right += outputs_r[slot];
            }
        }

        let steal_curve = self.steal_gain * self.steal_gain;
        let gain = self.velocity_sqrt * steal_curve;
        (output_left * gain, output_right * gain)
    }

    pub fn retrigger_envs(&mut self) {
        for state in self.states.iter_mut() {
            state.env_stage = crate::engine::dsp::EnvStage::Attack;
            state.env_value = 0.0;
            state.retrigger_unison();
        }
    }
}
