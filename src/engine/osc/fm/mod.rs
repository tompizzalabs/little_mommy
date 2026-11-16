use super::operator::{OperatorVoice, OperatorWave};

pub const EXP_FM_SEMITONES: f32 = 24.0;

#[inline]
pub fn pitch_shift_multiplier(semitones: f32) -> f32 {
    (0.057_762_265 * semitones).exp()
}

#[derive(Debug, Clone, Default)]
pub struct FmState {
    pub carrier: OperatorVoice,
    pub modulator: OperatorVoice,
}

const CARRIER_SEED_MIX: u32 = 0x85EB_CA6B;
const CARRIER_NODE_INDEX_MIX: u32 = 0x27D4_EB2F;
const MODULATOR_SEED_MIX: u32 = 0xC2B2_AE35;
const MODULATOR_NODE_INDEX_MIX: u32 = 0x1656_67B1;

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

    pub fn seed(&mut self, seed_base: u32, node_index: u32) {
        self.carrier.seed(
            seed_base
                .wrapping_mul(CARRIER_SEED_MIX)
                .wrapping_add(node_index.wrapping_mul(CARRIER_NODE_INDEX_MIX)),
        );
        self.modulator.seed(
            seed_base
                .wrapping_mul(MODULATOR_SEED_MIX)
                .wrapping_add(node_index.wrapping_mul(MODULATOR_NODE_INDEX_MIX)),
        );
    }
}
