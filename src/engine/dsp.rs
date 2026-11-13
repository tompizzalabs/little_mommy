use crate::engine::{
    lfo::{LfoShape, LfoState},
    osc::{
        AnalogShape, AnalogVoiceState, ChiptuneState, FmState, MAX_UNISON_VOICES, MultiState,
        NoiseColor, PmState, TableSource, VectorState, aa, noise, unison,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvStage {
    Attack,
    Decay,
    Sustain,
    Release,
    #[default]
    Done,
}

#[derive(Debug, Clone)]
pub struct NodeState {
    pub analog: [AnalogVoiceState; MAX_UNISON_VOICES],
    pub noise: noise::NoiseState,
    pub wave_phases: [f32; MAX_UNISON_VOICES],
    pub wave_aa: [aa::AntiAlias; MAX_UNISON_VOICES],
    pub low_l: f32,
    pub band_l: f32,
    pub low_r: f32,
    pub band_r: f32,
    pub env_stage: EnvStage,
    pub env_value: f32,
    pub lfo: LfoState,
    pub chiptune: ChiptuneState,
    pub multi: MultiState,
    pub vector: VectorState,
    pub fm: FmState,
    pub pm: PmState,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            analog: std::array::from_fn(|_| AnalogVoiceState::default()),
            noise: noise::NoiseState::default(),
            wave_phases: [0.0; MAX_UNISON_VOICES],
            wave_aa: [aa::AntiAlias::default(); MAX_UNISON_VOICES],
            low_l: 0.0,
            band_l: 0.0,
            low_r: 0.0,
            band_r: 0.0,
            env_stage: EnvStage::Done,
            env_value: 0.0,
            lfo: LfoState::default(),
            chiptune: ChiptuneState::default(),
            multi: MultiState::default(),
            vector: VectorState::default(),
            fm: FmState::default(),
            pm: PmState::default(),
        }
    }
}

impl NodeState {
    // Parameter lists mirror the node's ParamDef table.
    #[allow(clippy::too_many_arguments)]
    pub fn analog(
        &mut self,
        shape: AnalogShape,
        freq: f32,
        sample_rate: f32,
        pulse_width: f32,
        drift: f32,
        unison_voices: u8,
        unison_detune: f32,
        unison_detune_power: f32,
    ) -> (f32, f32) {
        let voices = unison_voices.max(1).min(MAX_UNISON_VOICES as u8);
        let multipliers = unison::detune_multipliers(voices, unison_detune_power);
        let positions = unison::pan_positions(voices);
        let comp = unison::gain_compensation(voices);
        let max_detune_cents = 50.0;

        let mut left = 0.0;
        let mut right = 0.0;
        for i in 0..voices as usize {
            let detune_cents = multipliers[i] * unison_detune * max_detune_cents;
            let voice_freq = freq * 2.0f32.powf(detune_cents / 1200.0);
            let (gain_l, gain_r) = unison::pan_gains(positions[i]);
            let sample = self.analog[i].next(shape, voice_freq, sample_rate, pulse_width, drift);
            left += sample * gain_l;
            right += sample * gain_r;
        }
        (left * comp, right * comp)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn wave(
        &mut self,
        source: &TableSource,
        freq: f32,
        sample_rate: f32,
        position: f32,
        formant: f32,
        skew_horizontal: f32,
        skew_vertical: f32,
        saturation: f32,
        unison_voices: u8,
        unison_detune: f32,
        unison_detune_power: f32,
    ) -> (f32, f32) {
        if sample_rate <= 0.0 {
            return (0.0, 0.0);
        }
        let voices = unison_voices.max(1).min(MAX_UNISON_VOICES as u8);
        let multipliers = unison::detune_multipliers(voices, unison_detune_power);
        let positions = unison::pan_positions(voices);
        let comp = unison::gain_compensation(voices);
        let max_detune_cents = 50.0;

        let mut left = 0.0;
        let mut right = 0.0;
        for i in 0..voices as usize {
            let detune_cents = multipliers[i] * unison_detune * max_detune_cents;
            let voice_freq = freq * 2.0f32.powf(detune_cents / 1200.0);
            let (gain_l, gain_r) = unison::pan_gains(positions[i]);
            let oversampled_rate = sample_rate * 3.0;
            let mut up = [0.0f32; 3];
            for slot in up.iter_mut() {
                let shaped_phase = shape_phase(self.wave_phases[i], formant, skew_horizontal);
                let raw = source.sample(shaped_phase, position);
                *slot = shape_amplitude(raw, skew_vertical, saturation);
                self.wave_phases[i] += voice_freq / oversampled_rate;
                while self.wave_phases[i] >= 1.0 {
                    self.wave_phases[i] -= 1.0;
                }
            }
            let sample = self.wave_aa[i].process(up);
            left += sample * gain_l;
            right += sample * gain_r;
        }
        (left * comp, right * comp)
    }

    pub fn noise(&mut self, color: NoiseColor) -> (f32, f32) {
        let mono = self.noise.sample(color);
        (mono, mono)
    }

    pub fn filter_stereo(
        &mut self,
        in_l: f32,
        in_r: f32,
        cutoff: f32,
        resonance: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        if sample_rate <= 0.0 {
            return (0.0, 0.0);
        }

        let normalized = (cutoff / sample_rate).clamp(0.0, 0.45);
        let f = (2.0 * (std::f32::consts::PI * normalized).sin()).min(1.0);
        let damping = 1.0 - resonance.clamp(0.0, 0.95);

        self.low_l += f * self.band_l;
        let high_l = in_l - self.low_l - damping * self.band_l;
        self.band_l += f * high_l;

        self.low_r += f * self.band_r;
        let high_r = in_r - self.low_r - damping * self.band_r;
        self.band_r += f * high_r;

        if !self.low_l.is_finite() || !self.band_l.is_finite() {
            self.low_l = 0.0;
            self.band_l = 0.0;
        }
        if !self.low_r.is_finite() || !self.band_r.is_finite() {
            self.low_r = 0.0;
            self.band_r = 0.0;
        }

        (self.low_l, self.low_r)
    }

    pub fn env(
        &mut self,
        sample_rate: f32,
        attack_ms: f32,
        decay_ms: f32,
        sustain: f32,
        release_ms: f32,
        releasing: bool,
    ) -> f32 {
        if releasing && self.env_stage != EnvStage::Release && self.env_stage != EnvStage::Done {
            self.env_stage = EnvStage::Release;
        }

        match self.env_stage {
            EnvStage::Attack => {
                let c = env_coeff(attack_ms, sample_rate);
                self.env_value += c * (1.0 - self.env_value);
                if self.env_value >= 0.999 {
                    self.env_value = 1.0;
                    self.env_stage = EnvStage::Decay;
                }
            }
            EnvStage::Decay => {
                let c = env_coeff(decay_ms, sample_rate);
                self.env_value += c * (sustain - self.env_value);
                if (self.env_value - sustain).abs() <= 0.001 {
                    self.env_value = sustain;
                    self.env_stage = EnvStage::Sustain;
                }
            }
            EnvStage::Sustain => {
                self.env_value = sustain;
            }
            EnvStage::Release => {
                let c = env_coeff(release_ms, sample_rate);
                self.env_value -= c * self.env_value;
                if self.env_value <= 0.0001 {
                    self.env_value = 0.0;
                    self.env_stage = EnvStage::Done;
                }
            }
            EnvStage::Done => {
                self.env_value = 0.0;
            }
        }

        self.env_value
    }
    pub fn lfo(&mut self, shape: LfoShape, rate: f32, phase_offset: f32, sample_rate: f32) -> f32 {
        self.lfo.next(shape, rate, phase_offset, sample_rate)
    }

    /// Chiptune: wavetable or 1-bit noise, arpeggiated pitch. Mono (duplicated).
    pub fn chiptune(
        &mut self,
        source: &TableSource,
        freq: f32,
        sample_rate: f32,
        position: f32,
    ) -> (f32, f32) {
        let mono = self.chiptune.next(source, freq, sample_rate, position);
        (mono, mono)
    }

    /// Multi oscillator: four voices of the same table spread across the morph
    /// axis. Mono (duplicated).
    pub fn multi(
        &mut self,
        source: &TableSource,
        freq: f32,
        sample_rate: f32,
        position: f32,
        detune: f32,
        spread: f32,
    ) -> (f32, f32) {
        let mono = self
            .multi
            .next(source, freq, sample_rate, position, detune, spread);
        (mono, mono)
    }

    /// Vector oscillator: four tables on the XY pad edges. Mono (duplicated).
    #[allow(clippy::too_many_arguments)]
    pub fn vector(
        &mut self,
        s0: &TableSource,
        s1: &TableSource,
        s2: &TableSource,
        s3: &TableSource,
        p0: f32,
        p1: f32,
        p2: f32,
        p3: f32,
        freq: f32,
        sample_rate: f32,
        x: f32,
        y: f32,
    ) -> (f32, f32) {
        let mono = self
            .vector
            .next(s0, s1, s2, s3, p0, p1, p2, p3, freq, sample_rate, x, y);
        (mono, mono)
    }

    /// FM oscillator: modulator feeds the carrier frequency. Mono (duplicated).
    #[allow(clippy::too_many_arguments)]
    pub fn fm(
        &mut self,
        carrier_source: &TableSource,
        modulator_source: &TableSource,
        freq: f32,
        sample_rate: f32,
        carrier_position: f32,
        modulator_position: f32,
        amount: f32,
        carrier_ratio: f32,
        modulator_ratio: f32,
        exponential: bool,
    ) -> (f32, f32) {
        let mono = self.fm.next(
            carrier_source,
            modulator_source,
            freq,
            sample_rate,
            carrier_position,
            modulator_position,
            amount,
            carrier_ratio,
            modulator_ratio,
            exponential,
        );
        (mono, mono)
    }

    /// PM oscillator: modulator feeds the carrier phase. Mono (duplicated).
    #[allow(clippy::too_many_arguments)]
    pub fn pm(
        &mut self,
        carrier_source: &TableSource,
        modulator_source: &TableSource,
        freq: f32,
        sample_rate: f32,
        carrier_position: f32,
        modulator_position: f32,
        amount: f32,
        carrier_ratio: f32,
        modulator_ratio: f32,
    ) -> (f32, f32) {
        let mono = self.pm.next(
            carrier_source,
            modulator_source,
            freq,
            sample_rate,
            carrier_position,
            modulator_position,
            amount,
            carrier_ratio,
            modulator_ratio,
        );
        (mono, mono)
    }
}

pub fn semitone_ratio(semitones: f32) -> f32 {
    2.0f32.powf(semitones / 12.0)
}

pub fn cent_ratio(cents: f32) -> f32 {
    2.0f32.powf(cents / 1200.0)
}

pub fn env_coeff(time_ms: f32, sample_rate: f32) -> f32 {
    if time_ms <= 0.0 || sample_rate <= 0.0 {
        return 1.0;
    }

    let frames = (time_ms * 0.001 * sample_rate).max(1.0);
    1.0 - (-1.0 / frames).exp()
}

pub(crate) fn shape_phase(phase: f32, formant: f32, skew_horizontal: f32) -> f32 {
    let mut p = phase * formant;
    p -= p.floor();

    if skew_horizontal.abs() > 1e-4 {
        p += (skew_horizontal * (p * std::f32::consts::TAU).sin()) / std::f32::consts::TAU;
    }

    p
}

pub(crate) fn shape_amplitude(sample: f32, skew_vertical: f32, saturation: f32) -> f32 {
    let mut y = sample;

    if skew_vertical.abs() > 1e-4 {
        let gain = if y >= 0.0 {
            1.0 + skew_vertical
        } else {
            1.0 - skew_vertical
        };
        y *= gain / (1.0 + skew_vertical.abs());
    }

    if saturation > 1e-4 {
        let drive = 1.0 + saturation * 7.0;
        y = (y * drive).tanh() / drive.tanh();
    }

    y
}
