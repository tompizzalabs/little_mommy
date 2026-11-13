use super::wavetable::{TableSource, WtVoice};
use crate::engine::arpeggiator::pitch_shift_multiplier;

/// Odin `EXP_FM_SEMITONES`.
pub const EXP_FM_SEMITONES: f32 = 24.0;

#[derive(Debug, Clone)]
pub struct FmState {
    pub carrier: WtVoice,
    pub modulator: WtVoice,
}

impl Default for FmState {
    fn default() -> Self {
        Self {
            carrier: WtVoice::default(),
            modulator: WtVoice::default(),
        }
    }
}

impl FmState {
    #[allow(clippy::too_many_arguments)]
    pub fn next(
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
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        let modulator_freq = freq * modulator_ratio / carrier_ratio;
        let mod_value = self.modulator.next(
            modulator_source,
            modulator_position,
            modulator_freq,
            sample_rate,
        );

        let amount = amount.max(0.0);

        let carrier_freq = if exponential {
            freq * pitch_shift_multiplier(mod_value * amount * EXP_FM_SEMITONES)
        } else {
            freq * (1.0 + mod_value * 15.0 * amount)
        };

        self.carrier
            .next(carrier_source, carrier_position, carrier_freq, sample_rate)
    }
}
