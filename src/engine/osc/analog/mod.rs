pub mod analog_tables;

pub use super::drift::DriftGenerator;
pub use super::drift::DRIFT_RANGE_SEMITONES;

use analog_tables::TABLE_LEN;

pub const SQUARE_VOLUME_SCALAR: f32 = 0.3;
pub const PWM_SMOOTH_FACTOR: f32 = 0.999;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AnalogShape {
    Saw,
    Square,
    Triangle,
    Sine,
}

impl AnalogShape {
    pub const ALL: [AnalogShape; 4] = [
        AnalogShape::Saw,
        AnalogShape::Square,
        AnalogShape::Triangle,
        AnalogShape::Sine,
    ];
}

impl std::fmt::Display for AnalogShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            AnalogShape::Saw => "Saw",
            AnalogShape::Square => "Square",
            AnalogShape::Triangle => "Triangle",
            AnalogShape::Sine => "Sine",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, Clone)]
pub struct AnalogVoiceState {
    pub phase: f32,
    pub duty_smooth: f32,
    pub drift: DriftGenerator,
}

impl Default for AnalogVoiceState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            duty_smooth: 0.5,
            drift: DriftGenerator::default(),
        }
    }
}

impl AnalogVoiceState {
    pub fn next(
        &mut self,
        shape: AnalogShape,
        freq: f32,
        sample_rate: f32,
        pulse_width: f32,
        drift: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        let tables = analog_tables::get();

        let drift_semitones = self.drift.next(sample_rate) * drift * DRIFT_RANGE_SEMITONES;
        let f = (freq * 2.0f32.powf(drift_semitones / 12.0)).max(0.0);

        let duty = pulse_width.clamp(0.02, 0.98);
        self.duty_smooth = self.duty_smooth * PWM_SMOOTH_FACTOR + (1.0 - PWM_SMOOTH_FACTOR) * duty;

        let idx = analog_tables::table_index(f);
        let saw_table = &tables.saw[idx];

        let pos = self.phase * TABLE_LEN as f32;
        let i0 = pos as usize & (TABLE_LEN - 1);
        let i1 = (i0 + 1) & (TABLE_LEN - 1);
        let frac = pos - pos.floor();
        let saw = lerp(saw_table[i0], saw_table[i1], frac);

        let out = match shape {
            AnalogShape::Saw => saw,
            AnalogShape::Square => {
                let duty_modded = self.duty_smooth;
                let pos_off = pos + duty_modded * TABLE_LEN as f32;
                let j0 = pos_off as usize & (TABLE_LEN - 1);
                let j1 = (j0 + 1) & (TABLE_LEN - 1);
                let frac_off = pos_off - pos_off.floor();
                let saw_off = lerp(saw_table[j0], saw_table[j1], frac_off);

                let d = duty_modded.rem_euclid(1.0);
                let square_scale = 2.0 * (1.0 - d).max(d);
                (saw - saw_off) * square_scale * SQUARE_VOLUME_SCALAR
            }
            AnalogShape::Triangle => {
                let tri = &tables.triangle[idx];
                lerp(tri[i0], tri[i1], frac)
            }
            AnalogShape::Sine => {
                let s = &tables.sine;
                lerp(s[i0], s[i1], frac)
            }
        };

        self.phase += f / sample_rate;
        self.phase -= self.phase.floor();

        out
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
