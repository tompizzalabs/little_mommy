use crate::engine::dsp::{NodeState, cent_ratio, semitone_ratio};
use crate::engine::graph::{CompiledGraph, MAX_NODES, ModLink};
use crate::engine::node::NodeParams;
use crate::engine::osc::analog::DriftGenerator;
use nice_plug::util;

const STEAL_FADE_TIME_MS: f32 = 50.0;

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
}

fn mod_offset(links: &[ModLink], param: usize, mods: &[f32; MAX_NODES]) -> f32 {
    let mut sum = 0.0;
    for l in links {
        if l.param as usize == param {
            sum += mods[l.source as usize] * l.depth;
        }
    }
    sum
}

fn modulated(params: &NodeParams, links: &[ModLink], mods: &[f32; MAX_NODES], i: usize) -> f32 {
    let d = &params.param_defs()[i];
    let base = params.param_get(i);
    let off = mod_offset(links, i, mods);
    (base + off * (d.max - d.min)).clamp(d.min, d.max)
}

impl Voice {
    pub fn new(internal_voice_id: u64, note_number: u8, velocity: f32) -> Self {
        let seed_base = internal_voice_id as u32;
        let mut states: Box<[NodeState]> = vec![NodeState::default(); MAX_NODES].into_boxed_slice();

        for (index, state) in states.iter_mut().enumerate() {
            state.noise.seed(
                seed_base
                    .wrapping_mul(0x9E3779B9)
                    .wrapping_add(index as u32 * 0x85EBCA6B),
            );
            state.lfo.rng = fastrand::Rng::with_seed(
                seed_base
                    .wrapping_mul(0x2545_F491)
                    .wrapping_add(index as u32 * 7 + 1) as u64,
            );
            state.lfo.read_index =
                ((seed_base.wrapping_add(index as u32) % 991) as f32) / 991.0 * 512.0;
            state.lfo.sh_value =
                ((seed_base.wrapping_add(index as u32 * 3) % 401) as f32) / 401.0 * 2.0 - 1.0;
            state.chiptune.seed(fastrand::Rng::with_seed(
                seed_base
                    .wrapping_mul(0x51_7C_C1_B7)
                    .wrapping_add(index as u32 * 0x1B873593) as u64,
            ));
            state.multi.voices[0].phase =
                ((seed_base.wrapping_add(index as u32) % 977) as f32) / 977.0;
            state.multi.voices[1].phase =
                ((seed_base.wrapping_add(index as u32 * 3) % 977) as f32) / 977.0;
            state.multi.voices[2].phase =
                ((seed_base.wrapping_add(index as u32 * 5) % 977) as f32) / 977.0;
            state.multi.voices[3].phase =
                ((seed_base.wrapping_add(index as u32 * 7) % 977) as f32) / 977.0;
            state.vector.voice.phase =
                ((seed_base.wrapping_add(index as u32) % 983) as f32) / 983.0;
            state.vector.snap_xy(0.5, 0.5);
            state.fm.carrier.phase = ((seed_base.wrapping_add(index as u32) % 971) as f32) / 971.0;
            state.fm.modulator.phase =
                ((seed_base.wrapping_add(index as u32 * 11) % 971) as f32) / 971.0;
            state.pm.carrier.phase = ((seed_base.wrapping_add(index as u32) % 967) as f32) / 967.0;
            state.pm.modulator.phase =
                ((seed_base.wrapping_add(index as u32 * 13) % 967) as f32) / 967.0;
            for (v_idx, analog_voice) in state.analog.iter_mut().enumerate() {
                analog_voice.phase = ((seed_base
                    .wrapping_add(index as u32)
                    .wrapping_add(v_idx as u32 * 13)
                    % 997) as f32)
                    / 997.0;
                analog_voice.duty_smooth = 0.5;
                analog_voice.drift = DriftGenerator::new(
                    seed_base
                        .wrapping_mul(0xC2B2AE35)
                        .wrapping_add(index as u32 * 0x27D4EB2F)
                        .wrapping_add(v_idx as u32 * 0x165667B1),
                );
            }
            for (v_idx, wave_phase) in state.wave_phases.iter_mut().enumerate() {
                *wave_phase = ((seed_base
                    .wrapping_add(index as u32)
                    .wrapping_add(v_idx as u32 * 17)
                    % 991) as f32)
                    / 991.0;
            }
            state.env_stage = crate::engine::dsp::EnvStage::Attack;
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
        let mut outputs_l = [0.0f32; MAX_NODES];
        let mut outputs_r = [0.0f32; MAX_NODES];
        let mut mod_values = [0.0f32; MAX_NODES];
        let mut env_present = false;
        let mut env_active = false;

        for &slot in compiled.order.iter() {
            let node = &compiled.nodes[slot];
            let in0_l = node.inputs[0].map(|s| outputs_l[s as usize]).unwrap_or(0.0);
            let in0_r = node.inputs[0].map(|s| outputs_r[s as usize]).unwrap_or(0.0);
            let in1_l = node.inputs[1].map(|s| outputs_l[s as usize]).unwrap_or(0.0);
            let in1_r = node.inputs[1].map(|s| outputs_r[s as usize]).unwrap_or(0.0);
            let state = &mut self.states[slot];
            let links = &node.mod_links;

            let (l, r) = match &node.params {
                NodeParams::Analog {
                    shape,
                    unison_voices,
                    ..
                } => {
                    let semis = modulated(&node.params, links, &mod_values, 0);
                    let det = modulated(&node.params, links, &mod_values, 1);
                    let pw = modulated(&node.params, links, &mod_values, 2);
                    let dr = modulated(&node.params, links, &mod_values, 3);
                    let ud = modulated(&node.params, links, &mod_values, 4);
                    let udp = modulated(&node.params, links, &mod_values, 5);
                    let freq = base_freq * semitone_ratio(semis) * cent_ratio(det);
                    state.analog(*shape, freq, sample_rate, pw, dr, *unison_voices, ud, udp)
                }
                NodeParams::Noise { color, .. } => {
                    let level = modulated(&node.params, links, &mod_values, 0);
                    let (nl, nr) = state.noise(*color);
                    (nl * level, nr * level)
                }
                NodeParams::Wave {
                    table: _,
                    unison_voices,
                    ..
                } => {
                    let pos = modulated(&node.params, links, &mod_values, 0);
                    let semis = modulated(&node.params, links, &mod_values, 1);
                    let det = modulated(&node.params, links, &mod_values, 2);
                    let form = modulated(&node.params, links, &mod_values, 3);
                    let skh = modulated(&node.params, links, &mod_values, 4);
                    let skv = modulated(&node.params, links, &mod_values, 5);
                    let sat = modulated(&node.params, links, &mod_values, 6);
                    let ud = modulated(&node.params, links, &mod_values, 7);
                    let udp = modulated(&node.params, links, &mod_values, 8);
                    let freq = base_freq * semitone_ratio(semis) * cent_ratio(det);
                    match &node.table_source {
                        Some(source) => state.wave(
                            source,
                            freq,
                            sample_rate,
                            pos,
                            form,
                            skh,
                            skv,
                            sat,
                            *unison_voices,
                            ud,
                            udp,
                        ),
                        None => (0.0, 0.0),
                    }
                }
                NodeParams::Chiptune { .. } => {
                    let pos = modulated(&node.params, links, &mod_values, 0);
                    let semis = modulated(&node.params, links, &mod_values, 1);
                    let det = modulated(&node.params, links, &mod_values, 2);
                    let arp_speed = modulated(&node.params, links, &mod_values, 3);
                    let freq = base_freq * semitone_ratio(semis) * cent_ratio(det);
                    if let NodeParams::Chiptune {
                        arp_enabled,
                        arp_three,
                        arp_steps,
                        noise,
                        ..
                    } = &node.params
                    {
                        state.chiptune.arp.frequency = arp_speed;
                        state.chiptune.arp.arp_active = *arp_enabled;
                        state.chiptune.arp.step_three_active = *arp_three;
                        state.chiptune.arp.step_semitones = *arp_steps;
                        state.chiptune.set_noise_mode(*noise);
                    }
                    match &node.table_source {
                        Some(source) => state.chiptune(source, freq, sample_rate, pos),
                        None => (0.0, 0.0),
                    }
                }
                NodeParams::Multi { .. } => {
                    let pos = modulated(&node.params, links, &mod_values, 0);
                    let semis = modulated(&node.params, links, &mod_values, 1);
                    let det = modulated(&node.params, links, &mod_values, 2);
                    let spread = modulated(&node.params, links, &mod_values, 3);
                    let freq = base_freq * semitone_ratio(semis);
                    match &node.table_source {
                        Some(source) => state.multi(source, freq, sample_rate, pos, det, spread),
                        None => (0.0, 0.0),
                    }
                }
                NodeParams::Vector { .. } => {
                    let x = modulated(&node.params, links, &mod_values, 0);
                    let y = modulated(&node.params, links, &mod_values, 1);
                    let p0 = modulated(&node.params, links, &mod_values, 2);
                    let p1 = modulated(&node.params, links, &mod_values, 3);
                    let p2 = modulated(&node.params, links, &mod_values, 4);
                    let p3 = modulated(&node.params, links, &mod_values, 5);
                    let semis = modulated(&node.params, links, &mod_values, 6);
                    let det = modulated(&node.params, links, &mod_values, 7);
                    let freq = base_freq * semitone_ratio(semis) * cent_ratio(det);
                    let s = &node.vector_sources;
                    match (&s[0], &s[1], &s[2], &s[3]) {
                        (Some(s0), Some(s1), Some(s2), Some(s3)) => {
                            state.vector(s0, s1, s2, s3, p0, p1, p2, p3, freq, sample_rate, x, y)
                        }
                        _ => (0.0, 0.0),
                    }
                }
                NodeParams::Fm { .. } => {
                    let cp = modulated(&node.params, links, &mod_values, 0);
                    let mp = modulated(&node.params, links, &mod_values, 1);
                    let semis = modulated(&node.params, links, &mod_values, 2);
                    let det = modulated(&node.params, links, &mod_values, 3);
                    let amt = modulated(&node.params, links, &mod_values, 4);
                    let freq = base_freq * semitone_ratio(semis) * cent_ratio(det);
                    if let NodeParams::Fm {
                        carrier_ratio,
                        modulator_ratio,
                        exponential,
                        ..
                    } = &node.params
                    {
                        let s = &node.pair_sources;
                        match (&s[0], &s[1]) {
                            (Some(carrier), Some(modulator)) => state.fm(
                                carrier,
                                modulator,
                                freq,
                                sample_rate,
                                cp,
                                mp,
                                amt,
                                *carrier_ratio as f32,
                                *modulator_ratio as f32,
                                *exponential,
                            ),
                            _ => (0.0, 0.0),
                        }
                    } else {
                        (0.0, 0.0)
                    }
                }
                NodeParams::Pm { .. } => {
                    let cp = modulated(&node.params, links, &mod_values, 0);
                    let mp = modulated(&node.params, links, &mod_values, 1);
                    let semis = modulated(&node.params, links, &mod_values, 2);
                    let det = modulated(&node.params, links, &mod_values, 3);
                    let amt = modulated(&node.params, links, &mod_values, 4);
                    let freq = base_freq * semitone_ratio(semis) * cent_ratio(det);
                    if let NodeParams::Pm {
                        carrier_ratio,
                        modulator_ratio,
                        ..
                    } = &node.params
                    {
                        let s = &node.pair_sources;
                        match (&s[0], &s[1]) {
                            (Some(carrier), Some(modulator)) => state.pm(
                                carrier,
                                modulator,
                                freq,
                                sample_rate,
                                cp,
                                mp,
                                amt,
                                *carrier_ratio as f32,
                                *modulator_ratio as f32,
                            ),
                            _ => (0.0, 0.0),
                        }
                    } else {
                        (0.0, 0.0)
                    }
                }
                NodeParams::Mix => (in0_l + in1_l, in0_r + in1_r),
                NodeParams::Filter { .. } => {
                    let cutoff = modulated(&node.params, links, &mod_values, 0);
                    let res = modulated(&node.params, links, &mod_values, 1);
                    state.filter_stereo(in0_l, in0_r, cutoff, res, sample_rate)
                }
                NodeParams::Env { .. } => {
                    let a = modulated(&node.params, links, &mod_values, 0);
                    let d = modulated(&node.params, links, &mod_values, 1);
                    let s = modulated(&node.params, links, &mod_values, 2);
                    let r = modulated(&node.params, links, &mod_values, 3);
                    let env = state.env(sample_rate, a, d, s, r, self.releasing);
                    env_present = true;
                    if state.env_stage != crate::engine::dsp::EnvStage::Done {
                        env_active = true;
                    }
                    (in0_l * env, in0_r * env)
                }
                NodeParams::Gain { .. } => {
                    let gain = modulated(&node.params, links, &mod_values, 0);
                    (in0_l * gain, in0_r * gain)
                }
                NodeParams::Lfo { shape, .. } => {
                    let rate = modulated(&node.params, links, &mod_values, 0);
                    let phase = modulated(&node.params, links, &mod_values, 1);
                    let v = state.lfo(*shape, rate, phase, sample_rate);
                    (v, v)
                }
                NodeParams::Out => (in0_l, in0_r),
            };

            outputs_l[slot] = l;
            outputs_r[slot] = r;

            mod_values[slot] = match &node.params {
                NodeParams::Env { .. } => state.env_value,
                NodeParams::Lfo { .. } => l,
                _ => (l + r) * 0.5,
            };
        }

        self.env_active = env_present && env_active;

        let mut sum_l = 0.0;
        let mut sum_r = 0.0;
        for (slot, node) in compiled.nodes.iter().enumerate() {
            if matches!(node.params, NodeParams::Out) {
                sum_l += outputs_l[slot];
                sum_r += outputs_r[slot];
            }
        }

        let steal_curve = self.steal_gain * self.steal_gain;
        let gain = self.velocity_sqrt * steal_curve;
        (sum_l * gain, sum_r * gain)
    }

    pub fn retrigger_envs(&mut self) {
        for state in self.states.iter_mut() {
            state.env_stage = crate::engine::dsp::EnvStage::Attack;
            state.env_value = 0.0;
        }
    }
}
