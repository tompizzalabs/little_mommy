use crate::engine::filter::{
    CombFilter, DiodeFilter, FormantFilter, Korg35Filter, LadderFilter, SemFilter,
    StateVariableFilter,
};
use crate::engine::{
    lfo::{LfoShape, LfoState},
    osc::unison::{MAX_DETUNE_CENTS, UnisonConfig},
    osc::{
        AnalogShape, AnalogVoiceState, FmState, MAX_UNISON_VOICES, NoiseColor, OperatorWave,
        PmState, VectorState, noise, unison,
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
    pub unison: unison::UnisonEngine,
    pub analog: [AnalogVoiceState; MAX_UNISON_VOICES],
    pub noise: noise::NoiseState,
    pub env_stage: EnvStage,
    pub env_value: f32,
    pub lfo: LfoState,
    pub fm: FmState,
    pub pm: PmState,
    pub vector: VectorState,
    pub ladder: LadderFilter,
    pub sem: SemFilter,
    pub korg35: Korg35Filter,
    pub diode: DiodeFilter,
    pub formant: FormantFilter,
    pub comb: CombFilter,
    pub state_variable: StateVariableFilter,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            unison: unison::UnisonEngine::default(),
            analog: std::array::from_fn(|_| AnalogVoiceState::default()),
            noise: noise::NoiseState::default(),
            env_stage: EnvStage::Done,
            env_value: 0.0,
            lfo: LfoState::default(),
            fm: FmState::default(),
            pm: PmState::default(),
            vector: VectorState::default(),
            ladder: LadderFilter::default(),
            sem: SemFilter::default(),
            korg35: Korg35Filter::default(),
            diode: DiodeFilter::default(),
            formant: FormantFilter::default(),
            comb: CombFilter::default(),
            state_variable: StateVariableFilter::default(),
        }
    }
}

impl NodeState {
    #[allow(clippy::too_many_arguments)]
    pub fn analog(
        &mut self,
        shape: AnalogShape,
        freq: f32,
        sample_rate: f32,
        pulse_width: f32,
        drift: f32,
        unison: &UnisonConfig,
    ) -> (f32, f32) {
        self.unison.sync(*unison, sample_rate);
        let voices = self.unison.voice_count();

        let mut left = 0.0;
        let mut right = 0.0;
        for (voice_index, voice_state) in self.analog.iter_mut().enumerate().take(voices) {
            let frame = self
                .unison
                .frame(voice_index, unison.detune, MAX_DETUNE_CENTS);
            let voice_freq = freq * frame.pitch_multiplier;
            let sample = voice_state.next(shape, voice_freq, sample_rate, pulse_width, drift);
            left += sample * frame.gain_left;
            right += sample * frame.gain_right;
        }
        self.unison.tick();
        (left, right)
    }
    pub fn retrigger_unison(&mut self) {
        if self.unison.phase_randomization() <= 0.0 {
            return;
        }

        let offsets = self.unison.reroll_phase_offsets();
        for (voice_index, offset) in offsets.iter().enumerate().take(self.unison.voice_count()) {
            self.analog[voice_index].phase = *offset;
        }
    }

    pub fn noise(&mut self, color: NoiseColor) -> (f32, f32) {
        let mono = self.noise.sample(color);
        (mono, mono)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn filter(
        &mut self,
        kind: crate::engine::filter::FilterKind,
        input_left: f32,
        input_right: f32,
        cutoff: f32,
        resonance: f32,
        transition: f32,
        vowel_left: usize,
        vowel_right: usize,
        saturation: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        use crate::engine::filter::{FilterKind, LadderMode};

        match kind {
            FilterKind::Ladder4LowPass => {
                self.ladder.set_mode(LadderMode::LowPass4);
                self.ladder.set_saturation(saturation);
                let mono = self.ladder.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::Ladder2LowPass => {
                self.ladder.set_mode(LadderMode::LowPass2);
                self.ladder.set_saturation(saturation);
                let mono = self.ladder.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::Ladder4BandPass => {
                self.ladder.set_mode(LadderMode::BandPass4);
                self.ladder.set_saturation(saturation);
                let mono = self.ladder.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::Ladder2BandPass => {
                self.ladder.set_mode(LadderMode::BandPass2);
                self.ladder.set_saturation(saturation);
                let mono = self.ladder.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::Ladder4HighPass => {
                self.ladder.set_mode(LadderMode::HighPass4);
                self.ladder.set_saturation(saturation);
                let mono = self.ladder.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::Ladder2HighPass => {
                self.ladder.set_mode(LadderMode::HighPass2);
                self.ladder.set_saturation(saturation);
                let mono = self.ladder.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::Sem => {
                let mono = self.sem.process(input_left, cutoff, resonance, transition, sample_rate);
                (mono, mono)
            }
            FilterKind::Korg35LowPass => {
                self.korg35.set_lowpass();
                self.korg35.set_saturation(saturation);
                let mono = self.korg35.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::Korg35HighPass => {
                self.korg35.set_highpass();
                self.korg35.set_saturation(saturation);
                let mono = self.korg35.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::Diode => {
                self.diode.set_saturation(saturation);
                let mono = self.diode.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::Formant => {
                let mono = self.formant.process(
                    input_left,
                    transition.clamp(0.0, 1.0),
                    vowel_left,
                    vowel_right,
                    sample_rate,
                );
                (mono, mono)
            }
            FilterKind::Comb => {
                let mono = self.comb.process(input_left, cutoff, resonance, sample_rate);
                (mono, mono)
            }
            FilterKind::StateVariable => self.state_variable.process_stereo(
                input_left,
                input_right,
                cutoff,
                resonance,
                sample_rate,
            ),
        }
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
                let attack_coefficient = env_coeff(attack_ms, sample_rate);
                self.env_value += attack_coefficient * (1.0 - self.env_value);
                if self.env_value >= 0.999 {
                    self.env_value = 1.0;
                    self.env_stage = EnvStage::Decay;
                }
            }
            EnvStage::Decay => {
                let decay_coefficient = env_coeff(decay_ms, sample_rate);
                self.env_value += decay_coefficient * (sustain - self.env_value);
                if (self.env_value - sustain).abs() <= 0.001 {
                    self.env_value = sustain;
                    self.env_stage = EnvStage::Sustain;
                }
            }
            EnvStage::Sustain => {
                self.env_value = sustain;
            }
            EnvStage::Release => {
                let release_coefficient = env_coeff(release_ms, sample_rate);
                self.env_value -= release_coefficient * self.env_value;
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

    #[allow(clippy::too_many_arguments)]
    pub fn fm(
        &mut self,
        carrier_wave: OperatorWave,
        modulator_wave: OperatorWave,
        freq: f32,
        sample_rate: f32,
        amount: f32,
        carrier_ratio: f32,
        modulator_ratio: f32,
        exponential: bool,
        drift: f32,
    ) -> (f32, f32) {
        let mono = self.fm.next(
            carrier_wave,
            modulator_wave,
            freq,
            sample_rate,
            amount,
            carrier_ratio,
            modulator_ratio,
            exponential,
            drift,
        );
        (mono, mono)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn pm(
        &mut self,
        carrier_wave: OperatorWave,
        modulator_wave: OperatorWave,
        freq: f32,
        sample_rate: f32,
        amount: f32,
        carrier_ratio: f32,
        modulator_ratio: f32,
        drift: f32,
    ) -> (f32, f32) {
        let mono = self.pm.next(
            carrier_wave,
            modulator_wave,
            freq,
            sample_rate,
            amount,
            carrier_ratio,
            modulator_ratio,
            drift,
        );
        (mono, mono)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn vector(
        &mut self,
        freq: f32,
        sample_rate: f32,
        x: f32,
        y: f32,
        drift: f32,
    ) -> (f32, f32) {
        let mono = self.vector.next(freq, sample_rate, x, y, drift);
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
