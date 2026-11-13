use super::wavetable::{TableSource, WtVoice};

pub const OSCS_PER_MULTIOSC: usize = 4;

const MULTI_MULTIPLIERS: [f32; OSCS_PER_MULTIOSC] = [0.97, -0.348, 0.238, -1.0];

#[inline]
fn cheap_pitch_shift_multiplier(semitones: f32) -> f32 {
    let x = semitones * 0.057762265;
    1.0 + x * (1.0 + x * (0.5 + x * (0.1666666 + x * 0.04166666)))
}

#[derive(Debug, Clone)]
pub struct MultiState {
    pub voices: [WtVoice; OSCS_PER_MULTIOSC],
}

impl Default for MultiState {
    fn default() -> Self {
        Self {
            voices: std::array::from_fn(|_| WtVoice::default()),
        }
    }
}

impl MultiState {
    #[allow(clippy::too_many_arguments)]
    pub fn next(
        &mut self,
        source: &TableSource,
        freq: f32,
        sample_rate: f32,
        position: f32,
        detune: f32,
        spread: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        let detune_modded = (detune * detune).max(0.0);

        let mut output = 0.0;
        for osc in 0..OSCS_PER_MULTIOSC {
            let osc_freq =
                freq * cheap_pitch_shift_multiplier(MULTI_MULTIPLIERS[osc] * detune_modded);

            let osc_pos = (position + (osc as f32 - 1.5) * spread * 0.5).clamp(0.0, 1.0);

            output += self.voices[osc].next(source, osc_pos, osc_freq, sample_rate);
        }

        output * 0.25
    }
}
