use super::wavetable::{TableSource, WtVoice};

#[derive(Debug, Clone)]
pub struct PmState {
    pub carrier: WtVoice,
    pub modulator: WtVoice,
}

impl Default for PmState {
    fn default() -> Self {
        Self {
            carrier: WtVoice::default(),
            modulator: WtVoice::default(),
        }
    }
}

impl PmState {
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

        let phase_mod = amount * mod_value;

        self.carrier.next_mod(
            carrier_source,
            carrier_position,
            freq,
            sample_rate,
            phase_mod,
        )
    }
}
