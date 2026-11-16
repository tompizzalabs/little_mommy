pub mod analog_tables;

pub use super::drift::DRIFT_RANGE_SEMITONES;
pub use super::drift::DriftGenerator;

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
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            AnalogShape::Saw => "Saw",
            AnalogShape::Square => "Square",
            AnalogShape::Triangle => "Triangle",
            AnalogShape::Sine => "Sine",
        };
        write!(formatter, "{}", name)
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
        let drifted_frequency = (freq * 2.0f32.powf(drift_semitones / 12.0)).max(0.0);

        let duty = pulse_width.clamp(0.02, 0.98);
        self.duty_smooth = self.duty_smooth * PWM_SMOOTH_FACTOR + (1.0 - PWM_SMOOTH_FACTOR) * duty;

        let mip_index = analog_tables::table_index(drifted_frequency);
        let saw_table = &tables.saw[mip_index];

        let position = self.phase * TABLE_LEN as f32;
        let first_index = position as usize & (TABLE_LEN - 1);
        let next_index = (first_index + 1) & (TABLE_LEN - 1);
        let fraction = position - position.floor();
        let saw = lerp(saw_table[first_index], saw_table[next_index], fraction);

        let output = match shape {
            AnalogShape::Saw => saw,
            AnalogShape::Square => {
                let smoothed_duty = self.duty_smooth;
                let offset_position = position + smoothed_duty * TABLE_LEN as f32;
                let offset_first_index = offset_position as usize & (TABLE_LEN - 1);
                let offset_next_index = (offset_first_index + 1) & (TABLE_LEN - 1);
                let offset_fraction = offset_position - offset_position.floor();
                let offset_saw = lerp(
                    saw_table[offset_first_index],
                    saw_table[offset_next_index],
                    offset_fraction,
                );

                let normalized_duty = smoothed_duty.rem_euclid(1.0);
                let square_scale = 2.0 * (1.0 - normalized_duty).max(normalized_duty);
                (saw - offset_saw) * square_scale * SQUARE_VOLUME_SCALAR
            }
            AnalogShape::Triangle => {
                let triangle_table = &tables.triangle[mip_index];
                lerp(
                    triangle_table[first_index],
                    triangle_table[next_index],
                    fraction,
                )
            }
            AnalogShape::Sine => {
                let sine_table = &tables.sine;
                lerp(sine_table[first_index], sine_table[next_index], fraction)
            }
        };

        self.phase += drifted_frequency / sample_rate;
        self.phase -= self.phase.floor();

        output
    }
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}
