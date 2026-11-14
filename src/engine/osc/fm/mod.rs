use super::operator::{OperatorVoice, OperatorWave};

/// Odin `EXP_FM_SEMITONES`.
pub const EXP_FM_SEMITONES: f32 = 24.0;

#[inline]
pub fn pitch_shift_multiplier(semitones: f32) -> f32 {
    (0.057_762_265 * semitones).exp()
}

#[derive(Debug, Clone)]
pub struct FmState {
    pub carrier: OperatorVoice,
    pub modulator: OperatorVoice,
}

impl Default for FmState {
    fn default() -> Self {
        Self {
            carrier: OperatorVoice::default(),
            modulator: OperatorVoice::default(),
        }
    }
}

impl FmState {
    #[allow(clippy::too_many_arguments)]
    pub fn next(
        &mut self,
        carrier_wave: OperatorWave,
        modulator_wave: OperatorWave,
        freq: f32,
        sample_rate: f32,
        amount: f32,
        carrier_ratio: f32,
        modulator_ratio: f32,
        exponential: bool,
        drift_amount: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        let modulator_freq = freq * modulator_ratio / carrier_ratio;
        let mod_value =
            self.modulator
                .next(modulator_wave, modulator_freq, sample_rate, drift_amount);

        let amount = amount.max(0.0);

        let carrier_freq = if exponential {
            freq * pitch_shift_multiplier(mod_value * amount * EXP_FM_SEMITONES)
        } else {
            freq * (1.0 + mod_value * 15.0 * amount)
        };

        self.carrier
            .next(carrier_wave, carrier_freq, sample_rate, drift_amount)
    }

    /// Re-seed the per-voice operator voices (phase + drift).
    pub fn seed(&mut self, seed_base: u32, node_index: u32) {
        self.carrier.seed(
            seed_base
                .wrapping_mul(0x85EB_CA6B)
                .wrapping_add(node_index.wrapping_mul(0x27D4_EB2F)),
        );
        self.modulator.seed(
            seed_base
                .wrapping_mul(0xC2B2_AE35)
                .wrapping_add(node_index.wrapping_mul(0x1656_67B1)),
        );
    }
}
