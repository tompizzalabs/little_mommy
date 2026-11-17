use crate::engine::effects::{
    BitcrusherState, ChorusState, DelayState, DistortionAlgorithm, DistortionState, FlangerState,
    PhaserState, RingModState, ZitaReverbState,
};
use crate::engine::filter::{
    CombFilter, DiodeFilter, FilterKind, FormantFilter, Korg35Filter, LadderFilter, LadderMode,
    PeakingEq, SemFilter, StateVariableFilter,
};
use crate::engine::{
    lfo::{LfoShape, LfoState},
    osc::unison::{MAX_DETUNE_CENTS, UnisonConfig},
    osc::{
        AnalogShape, AnalogVoiceState, FmState, MAX_UNISON_VOICES, NoiseColor, OperatorWave,
        PmState, SyncIn, SyncOut, VectorState, noise, unison,
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
    pub sync_out: SyncOut,
    pub env_stage: EnvStage,
    pub env_value: f32,
    pub lfo: LfoState,
    pub fm: FmState,
    pub pm: PmState,
    pub vector: VectorState,
    pub ladder: [LadderFilter; 2],
    pub sem: [SemFilter; 2],
    pub korg35: [Korg35Filter; 2],
    pub diode: [DiodeFilter; 2],
    pub formant: [FormantFilter; 2],
    pub comb: [CombFilter; 2],
    pub state_variable: StateVariableFilter,
    pub delay: DelayState,
    pub chorus: [ChorusState; 2],
    pub flanger: [FlangerState; 2],
    pub phaser: PhaserState,
    pub bitcrusher: BitcrusherState,
    pub ring_mod: RingModState,
    pub distortion: [DistortionState; 2],
    pub equalizer: [PeakingEq; 2],
    pub reverb: ZitaReverbState,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            unison: unison::UnisonEngine::default(),
            analog: std::array::from_fn(|_| AnalogVoiceState::default()),
            noise: noise::NoiseState::default(),
            sync_out: SyncOut::default(),
            env_stage: EnvStage::Done,
            env_value: 0.0,
            lfo: LfoState::default(),
            fm: FmState::default(),
            pm: PmState::default(),
            vector: VectorState::default(),
            ladder: std::array::from_fn(|_| LadderFilter::default()),
            sem: std::array::from_fn(|_| SemFilter::default()),
            korg35: std::array::from_fn(|_| Korg35Filter::default()),
            diode: std::array::from_fn(|_| DiodeFilter::default()),
            formant: std::array::from_fn(|_| FormantFilter::default()),
            comb: std::array::from_fn(|_| CombFilter::default()),
            state_variable: StateVariableFilter::default(),
            delay: DelayState::default(),
            chorus: std::array::from_fn(|_| ChorusState::default()),
            flanger: std::array::from_fn(|_| FlangerState::default()),
            phaser: PhaserState::default(),
            bitcrusher: BitcrusherState::default(),
            ring_mod: RingModState::default(),
            distortion: std::array::from_fn(|_| DistortionState::default()),
            equalizer: std::array::from_fn(|_| PeakingEq::default()),
            reverb: ZitaReverbState::default(),
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
        morph: f32,
        unison: &UnisonConfig,
        sync: Option<SyncIn>,
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
            let sample = voice_state.next(
                shape,
                voice_freq,
                sample_rate,
                pulse_width,
                drift,
                morph,
                sync,
            );
            left += sample * frame.gain_left;
            right += sample * frame.gain_right;
        }
        self.sync_out = self.analog[0].sync_out();
        self.unison.tick();
        (left, right)
    }
    pub fn retrigger_unison(&mut self) {
        if self.unison.phase_randomization() <= 0.0 {
            return;
        }

        let offsets = self.unison.reroll_phase_offsets();
        for (voice_index, offset) in offsets.iter().enumerate().take(self.unison.voice_count()) {
            self.analog[voice_index].set_phase(*offset);
        }
    }

    pub fn noise(&mut self, color: NoiseColor, frequency: f32, sample_rate: f32) -> (f32, f32) {
        let mono = self.noise.sample(color, frequency, sample_rate);
        (mono, mono)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn filter(
        &mut self,
        kind: FilterKind,
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
        match kind {
            FilterKind::Ladder4LowPass => self.process_ladder(
                LadderMode::LowPass4,
                input_left,
                input_right,
                cutoff,
                resonance,
                saturation,
                sample_rate,
            ),
            FilterKind::Ladder2LowPass => self.process_ladder(
                LadderMode::LowPass2,
                input_left,
                input_right,
                cutoff,
                resonance,
                saturation,
                sample_rate,
            ),
            FilterKind::Ladder4BandPass => self.process_ladder(
                LadderMode::BandPass4,
                input_left,
                input_right,
                cutoff,
                resonance,
                saturation,
                sample_rate,
            ),
            FilterKind::Ladder2BandPass => self.process_ladder(
                LadderMode::BandPass2,
                input_left,
                input_right,
                cutoff,
                resonance,
                saturation,
                sample_rate,
            ),
            FilterKind::Ladder4HighPass => self.process_ladder(
                LadderMode::HighPass4,
                input_left,
                input_right,
                cutoff,
                resonance,
                saturation,
                sample_rate,
            ),
            FilterKind::Ladder2HighPass => self.process_ladder(
                LadderMode::HighPass2,
                input_left,
                input_right,
                cutoff,
                resonance,
                saturation,
                sample_rate,
            ),
            FilterKind::Sem => {
                let left =
                    self.sem[0].process(input_left, cutoff, resonance, transition, sample_rate);
                let right =
                    self.sem[1].process(input_right, cutoff, resonance, transition, sample_rate);
                (left, right)
            }
            FilterKind::Korg35LowPass => self.process_korg35(
                true,
                input_left,
                input_right,
                cutoff,
                resonance,
                saturation,
                sample_rate,
            ),
            FilterKind::Korg35HighPass => self.process_korg35(
                false,
                input_left,
                input_right,
                cutoff,
                resonance,
                saturation,
                sample_rate,
            ),
            FilterKind::Diode => {
                for filter in self.diode.iter_mut() {
                    filter.set_saturation(saturation);
                }
                let left = self.diode[0].process(input_left, cutoff, resonance, sample_rate);
                let right = self.diode[1].process(input_right, cutoff, resonance, sample_rate);
                (left, right)
            }
            FilterKind::Formant => {
                let transition = transition.clamp(0.0, 1.0);
                let left = self.formant[0].process(
                    input_left,
                    transition,
                    vowel_left,
                    vowel_right,
                    sample_rate,
                );
                let right = self.formant[1].process(
                    input_right,
                    transition,
                    vowel_left,
                    vowel_right,
                    sample_rate,
                );
                (left, right)
            }
            FilterKind::Comb => {
                let left = self.comb[0].process(input_left, cutoff, resonance, sample_rate);
                let right = self.comb[1].process(input_right, cutoff, resonance, sample_rate);
                (left, right)
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

    #[allow(clippy::too_many_arguments)]
    fn process_ladder(
        &mut self,
        mode: LadderMode,
        input_left: f32,
        input_right: f32,
        cutoff: f32,
        resonance: f32,
        saturation: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        for filter in self.ladder.iter_mut() {
            filter.set_mode(mode);
            filter.set_saturation(saturation);
        }
        let left = self.ladder[0].process(input_left, cutoff, resonance, sample_rate);
        let right = self.ladder[1].process(input_right, cutoff, resonance, sample_rate);
        (left, right)
    }

    #[allow(clippy::too_many_arguments)]
    fn process_korg35(
        &mut self,
        lowpass: bool,
        input_left: f32,
        input_right: f32,
        cutoff: f32,
        resonance: f32,
        saturation: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        for filter in self.korg35.iter_mut() {
            if lowpass {
                filter.set_lowpass();
            } else {
                filter.set_highpass();
            }
            filter.set_saturation(saturation);
        }
        let left = self.korg35[0].process(input_left, cutoff, resonance, sample_rate);
        let right = self.korg35[1].process(input_right, cutoff, resonance, sample_rate);
        (left, right)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn delay(
        &mut self,
        input_left: f32,
        input_right: f32,
        delay_time: f32,
        feedback: f32,
        dry: f32,
        wet: f32,
        ducking: f32,
        ping_pong: bool,
        sample_rate: f32,
    ) -> (f32, f32) {
        self.delay.process_stereo(
            input_left,
            input_right,
            delay_time,
            feedback,
            dry,
            wet,
            ducking,
            ping_pong,
            sample_rate,
        )
    }

    pub fn chorus(
        &mut self,
        channel: usize,
        input: f32,
        rate: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
        sample_rate: f32,
    ) -> f32 {
        self.chorus[channel].process(input, rate, amount, feedback, dry_wet, sample_rate)
    }

    pub fn flanger(
        &mut self,
        channel: usize,
        input: f32,
        rate: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
        sample_rate: f32,
    ) -> f32 {
        self.flanger[channel].process(input, rate, amount, feedback, dry_wet, sample_rate)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn phaser(
        &mut self,
        input_left: f32,
        input_right: f32,
        rate: f32,
        base_frequency: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        self.phaser.process_stereo(
            input_left,
            input_right,
            rate,
            base_frequency,
            amount,
            feedback,
            dry_wet,
            sample_rate,
        )
    }

    pub fn bitcrusher(
        &mut self,
        input_left: f32,
        input_right: f32,
        bits: f32,
        edge: f32,
        dry_wet: f32,
    ) -> (f32, f32) {
        self.bitcrusher
            .process_stereo(input_left, input_right, bits, edge, dry_wet)
    }

    pub fn ring_mod(
        &mut self,
        input_left: f32,
        input_right: f32,
        frequency: f32,
        amount: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        self.ring_mod
            .process_stereo(input_left, input_right, frequency, amount, sample_rate)
    }

    pub fn distortion(
        &mut self,
        channel: usize,
        input: f32,
        threshold: f32,
        algorithm: DistortionAlgorithm,
        dry_wet: f32,
    ) -> f32 {
        self.distortion[channel].process(input, threshold, algorithm, dry_wet)
    }

    pub fn equalizer(
        &mut self,
        channel: usize,
        input: f32,
        frequency: f32,
        q: f32,
        gain_db: f32,
        sample_rate: f32,
    ) -> f32 {
        let equalizer = &mut self.equalizer[channel];
        equalizer.set_frequency(frequency);
        equalizer.set_q(q);
        equalizer.set_gain_db(gain_db);
        equalizer.process(input, sample_rate)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reverb(
        &mut self,
        input_left: f32,
        input_right: f32,
        pre_delay: f32,
        xover: f32,
        low_decay: f32,
        mid_decay: f32,
        damping: f32,
        mix: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        self.reverb.process_stereo(
            input_left,
            input_right,
            pre_delay,
            xover,
            low_decay,
            mid_decay,
            damping,
            mix,
            sample_rate,
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn mono_filters_preserve_channel_differences() {
        for kind in crate::engine::filter::FilterKind::ALL {
            let mut state = NodeState::default();
            let mut output_difference: f32 = 0.0;
            for step in 0..1_000 {
                let input_left = (step as f32 * 0.01).sin() * 0.5;
                let input_right = (step as f32 * 0.01 + 1.5).sin() * 0.5;
                let (left, right) = state.filter(
                    kind,
                    input_left,
                    input_right,
                    4_000.0,
                    0.2,
                    0.5,
                    0,
                    1,
                    0.0,
                    SAMPLE_RATE,
                );
                assert!(
                    left.is_finite() && right.is_finite(),
                    "{kind} must stay finite"
                );
                output_difference = output_difference.max((left - right).abs());
            }
            assert!(
                output_difference > 1e-4,
                "{kind} collapsed stereo to dual mono (max channel difference {output_difference})"
            );
        }
    }
}
