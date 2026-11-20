mod tables;

pub use tables::{LFO_TABLE_LENGTH, LFO_TABLES};

use fastrand::Rng;

const TAU: f32 = std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LfoShape {
    Sine,
    Saw,
    Triangle,
    Square,
    Square25,
    Square12,
    Spike,
    Pyramid4,
    Pyramid6,
    Pyramid8,
    Pyramid12,
    Stair3,
    Stair4,
    Stair6,
    Stair8,
    Stair12,
    SampleHold,
}

impl LfoShape {
    pub const ALL: [LfoShape; 17] = [
        LfoShape::Sine,
        LfoShape::Saw,
        LfoShape::Triangle,
        LfoShape::Square,
        LfoShape::Square25,
        LfoShape::Square12,
        LfoShape::Spike,
        LfoShape::Pyramid4,
        LfoShape::Pyramid6,
        LfoShape::Pyramid8,
        LfoShape::Pyramid12,
        LfoShape::Stair3,
        LfoShape::Stair4,
        LfoShape::Stair6,
        LfoShape::Stair8,
        LfoShape::Stair12,
        LfoShape::SampleHold,
    ];

    pub fn table_index(&self) -> Option<usize> {
        match self {
            LfoShape::Square => Some(0),
            LfoShape::Square25 => Some(1),
            LfoShape::Square12 => Some(2),
            LfoShape::Spike => Some(3),
            LfoShape::Pyramid4 => Some(4),
            LfoShape::Pyramid6 => Some(5),
            LfoShape::Pyramid8 => Some(6),
            LfoShape::Pyramid12 => Some(7),
            LfoShape::Stair3 => Some(8),
            LfoShape::Stair4 => Some(9),
            LfoShape::Stair6 => Some(10),
            LfoShape::Stair8 => Some(11),
            LfoShape::Stair12 => Some(12),
            _ => None,
        }
    }

    pub fn sample(&self, phase: f32) -> f32 {
        let length = LFO_TABLE_LENGTH as f32;
        let table_position = phase.rem_euclid(1.0) * length;
        if *self == LfoShape::SampleHold {
            let step = (phase.rem_euclid(1.0) * 8.0) as u32;
            return (step.wrapping_mul(0x9E37_79B9) as f32 / u32::MAX as f32) * 2.0 - 1.0;
        }
        sample_table(*self, table_position)
    }
}

impl std::fmt::Display for LfoShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            LfoShape::Sine => "Sine",
            LfoShape::Saw => "Saw",
            LfoShape::Triangle => "Triangle",
            LfoShape::Square => "Square",
            LfoShape::Square25 => "Square 25%",
            LfoShape::Square12 => "Square 12%",
            LfoShape::Spike => "Spike",
            LfoShape::Pyramid4 => "Pyramid 4",
            LfoShape::Pyramid6 => "Pyramid 6",
            LfoShape::Pyramid8 => "Pyramid 8",
            LfoShape::Pyramid12 => "Pyramid 12",
            LfoShape::Stair3 => "Stair 3",
            LfoShape::Stair4 => "Stair 4",
            LfoShape::Stair6 => "Stair 6",
            LfoShape::Stair8 => "Stair 8",
            LfoShape::Stair12 => "Stair 12",
            LfoShape::SampleHold => "S+H",
        };
        write!(f, "{}", name)
    }
}

pub const DEFAULT_TEMPO_BPM: f64 = 120.0;

const BEATS_PER_WHOLE_NOTE: f32 = 4.0;

pub fn synced_rate_hz(bpm: f64, note_division: u8) -> f32 {
    let beats_per_cycle = BEATS_PER_WHOLE_NOTE / note_division.max(1) as f32;
    (bpm.max(0.0) / 60.0) as f32 / beats_per_cycle
}

#[derive(Debug, Clone)]
pub struct LfoState {
    pub read_index: f32,
    pub sample_hold_value: f32,
    pub rng: Rng,
}

impl Default for LfoState {
    fn default() -> Self {
        Self {
            read_index: 0.0,
            sample_hold_value: 0.0,
            rng: Rng::with_seed(0x9E37_79B9),
        }
    }
}

impl LfoState {
    pub fn next(&mut self, shape: LfoShape, rate: f32, phase_offset: f32, sample_rate: f32) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        let length = LFO_TABLE_LENGTH as f32;
        let increment = length * rate.max(0.0) / sample_rate;

        if shape == LfoShape::SampleHold {
            self.read_index += increment;
            while self.read_index < 0.0 {
                self.read_index += length;
                self.sample_hold_value = self.rng.f32() * 2.0 - 1.0;
            }
            while self.read_index >= length {
                self.read_index -= length;
                self.sample_hold_value = self.rng.f32() * 2.0 - 1.0;
            }
            return self.sample_hold_value;
        }

        let position = (self.read_index + phase_offset.clamp(0.0, 1.0) * length).rem_euclid(length);
        let output = sample_table(shape, position);

        self.read_index += increment;
        while self.read_index < 0.0 {
            self.read_index += length;
        }
        while self.read_index >= length {
            self.read_index -= length;
        }

        output
    }
}

fn sample_table(shape: LfoShape, position: f32) -> f32 {
    let length = LFO_TABLE_LENGTH as f32;
    let first_index = position as usize;

    let next_index = if first_index + 1 >= LFO_TABLE_LENGTH {
        0
    } else {
        first_index + 1
    };
    let fraction = position - first_index as f32;

    match shape {
        LfoShape::Sine => (position / length * TAU).sin(),
        LfoShape::Saw => position / length * 2.0 - 1.0,
        LfoShape::Triangle => {
            if position < length * 0.5 {
                4.0 * (position / length) - 1.0
            } else {
                3.0 - 4.0 * (position / length)
            }
        }
        _ => {
            let table = &LFO_TABLES[shape.table_index().unwrap_or(0)];
            table[first_index] + (table[next_index] - table[first_index]) * fraction
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synced_rate_tracks_tempo_and_division() {
        assert!(
            (synced_rate_hz(120.0, 4) - 2.0).abs() < 1e-5,
            "1/4 at 120 BPM is two cycles per second, got {}",
            synced_rate_hz(120.0, 4)
        );
        assert!(
            (synced_rate_hz(120.0, 1) - 0.5).abs() < 1e-5,
            "1/1 at 120 BPM is one cycle per bar, got {}",
            synced_rate_hz(120.0, 1)
        );
        assert!(
            (synced_rate_hz(120.0, 16) - 8.0).abs() < 1e-5,
            "1/16 at 120 BPM is eight cycles per second, got {}",
            synced_rate_hz(120.0, 16)
        );
        assert!(
            (synced_rate_hz(90.0, 4) - 1.5).abs() < 1e-5,
            "1/4 at 90 BPM is 1.5 cycles per second, got {}",
            synced_rate_hz(90.0, 4)
        );
    }

    #[test]
    fn synced_rate_stays_safe_for_odd_input() {
        assert_eq!(synced_rate_hz(0.0, 4), 0.0);
        assert_eq!(synced_rate_hz(-10.0, 4), 0.0);
        assert_eq!(synced_rate_hz(120.0, 0), synced_rate_hz(120.0, 1));
    }
}
