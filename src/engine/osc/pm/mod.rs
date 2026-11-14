use super::operator::{OperatorVoice, OperatorWave};

#[derive(Debug, Clone, Default)]
pub struct PmState {
    pub carrier: OperatorVoice,
    pub modulator: OperatorVoice,
}

impl PmState {
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
        drift_amount: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        let modulator_freq = freq * modulator_ratio / carrier_ratio;
        let mod_value =
            self.modulator
                .next(modulator_wave, modulator_freq, sample_rate, drift_amount);

        let phase_mod = amount * mod_value;

        self.carrier
            .next_mod(carrier_wave, freq, sample_rate, drift_amount, phase_mod)
    }

    /// Re-seed the per-voice operator voices (phase + drift).
    pub fn seed(&mut self, seed_base: u32, node_index: u32) {
        self.carrier.seed(
            seed_base
                .wrapping_mul(0x96A5_A9B3)
                .wrapping_add(node_index.wrapping_mul(0x27D4_EB2F)),
        );
        self.modulator.seed(
            seed_base
                .wrapping_mul(0xB0E7_3A3D)
                .wrapping_add(node_index.wrapping_mul(0x1656_67B1)),
        );
    }
}
