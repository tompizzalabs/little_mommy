mod tables;

pub use tables::{LFO_TABLE_LENGTH, LFO_TABLES};

use fastrand::Rng;

const TAU: f32 = std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn sample(&self, p: f32) -> f32 {
        let length = LFO_TABLE_LENGTH as f32;
        let pp = p.rem_euclid(1.0) * length;
        if *self == LfoShape::SampleHold {
            let step = (p.rem_euclid(1.0) * 8.0) as u32;
            return (step.wrapping_mul(0x9E37_79B9) as f32 / u32::MAX as f32) * 2.0 - 1.0;
        }
        sample_table(*self, pp)
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

#[derive(Debug, Clone)]
pub struct LfoState {
    pub read_index: f32,
    pub sh_value: f32,
    pub rng: Rng,
}

impl Default for LfoState {
    fn default() -> Self {
        Self {
            read_index: 0.0,
            sh_value: 0.0,
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
        let inc = length * rate.max(0.0) / sample_rate;

        if shape == LfoShape::SampleHold {
            // Odin `doSampleHold()`: advance the reader, then pull a fresh S&H value
            // on *every* boundary crossed. A large increment can wrap several times.
            self.read_index += inc;
            while self.read_index < 0.0 {
                self.read_index += length;
                self.sh_value = self.rng.f32() * 2.0 - 1.0;
            }
            while self.read_index >= length {
                self.read_index -= length;
                self.sh_value = self.rng.f32() * 2.0 - 1.0;
            }
            return self.sh_value;
        }

        let p = (self.read_index + phase_offset.clamp(0.0, 1.0) * length).rem_euclid(length);
        let out = sample_table(shape, p);

        self.read_index += inc;
        while self.read_index < 0.0 {
            self.read_index += length;
        }
        while self.read_index >= length {
            self.read_index -= length;
        }

        out
    }
}

fn sample_table(shape: LfoShape, p: f32) -> f32 {
    let length = LFO_TABLE_LENGTH as f32;
    let i0 = p as usize;

    let i1 = if i0 + 1 >= LFO_TABLE_LENGTH {
        0
    } else {
        i0 + 1
    };
    let frac = p - i0 as f32;

    match shape {
        LfoShape::Sine => (p / length * TAU).sin(),
        LfoShape::Saw => p / length * 2.0 - 1.0,
        LfoShape::Triangle => {
            if p < length * 0.5 {
                4.0 * (p / length) - 1.0
            } else {
                3.0 - 4.0 * (p / length)
            }
        }
        _ => {
            let table = &LFO_TABLES[shape.table_index().unwrap_or(0)];
            table[i0] + (table[i1] - table[i0]) * frac
        }
    }
}
