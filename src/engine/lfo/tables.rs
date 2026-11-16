use std::f32::consts::PI;
use std::sync::LazyLock;

pub const NUMBER_OF_LFO_TABLES: usize = 13;
pub const LFO_TABLE_LENGTH: usize = 512;

pub const HARMONICS: usize = 64;

const TAU: f32 = std::f32::consts::TAU;

pub static LFO_TABLES: LazyLock<[[f32; LFO_TABLE_LENGTH]; NUMBER_OF_LFO_TABLES]> =
    LazyLock::new(|| {
        [
            square(0.5),
            square(0.25),
            square(0.125),
            spike(),
            pyramid(4),
            pyramid(6),
            pyramid(8),
            pyramid(12),
            stair(3),
            stair(4),
            stair(6),
            stair(8),
            stair(12),
        ]
    });

struct Spectrum {
    dc: f32,
    cos: [f32; HARMONICS],
    sin: [f32; HARMONICS],
}

impl Spectrum {
    fn new() -> Self {
        Self {
            dc: 0.0,
            cos: [0.0; HARMONICS],
            sin: [0.0; HARMONICS],
        }
    }

    fn pulse(&mut self, duty: f32, gain: f32) {
        self.dc += gain * (2.0 * duty - 1.0);
        for harmonic in 1..=HARMONICS {
            let harmonic_index = harmonic as f32;
            let (sine_component, cosine_component) = (TAU * harmonic_index * duty).sin_cos();
            let amplitude = 2.0 * gain / (PI * harmonic_index);
            self.cos[harmonic - 1] += amplitude * sine_component;
            self.sin[harmonic - 1] += amplitude * (1.0 - cosine_component);
        }
    }

    fn render(self) -> [f32; LFO_TABLE_LENGTH] {
        let mut output = [0.0; LFO_TABLE_LENGTH];
        for (sample_index, slot) in output.iter_mut().enumerate() {
            let time = sample_index as f32 / LFO_TABLE_LENGTH as f32;
            let mut value = self.dc;
            for harmonic in 1..=HARMONICS {
                let (sine_component, cosine_component) = (TAU * harmonic as f32 * time).sin_cos();
                value += self.cos[harmonic - 1] * cosine_component
                    + self.sin[harmonic - 1] * sine_component;
            }
            *slot = value;
        }
        output
    }
}

fn square(duty: f32) -> [f32; LFO_TABLE_LENGTH] {
    let mut spectrum = Spectrum::new();
    spectrum.pulse(duty, 1.0);
    spectrum.render()
}

fn stair(steps: usize) -> [f32; LFO_TABLE_LENGTH] {
    let mut spectrum = Spectrum::new();
    let gain = -1.0 / (steps as f32 - 1.0);
    for step in 1..steps {
        spectrum.pulse(step as f32 / steps as f32, gain);
    }
    spectrum.render()
}

fn pyramid(steps: usize) -> [f32; LFO_TABLE_LENGTH] {
    let mut spectrum = Spectrum::new();
    let gain = 2.0 / steps as f32;
    spectrum.dc += gain - 1.0;
    for step in 1..steps {
        spectrum.pulse(
            step as f32 / steps as f32,
            if step <= steps / 2 { -gain } else { gain },
        );
    }
    spectrum.render()
}

fn spike() -> [f32; LFO_TABLE_LENGTH] {
    let mut spectrum = Spectrum::new();
    spectrum.dc = -1.0 / 3.0;
    for harmonic in 1..=HARMONICS {
        let harmonic_index = harmonic as f32;
        spectrum.cos[harmonic - 1] = 8.0 / (PI * PI * harmonic_index * harmonic_index)
            * if harmonic % 2 == 0 { 1.0 } else { -1.0 };
    }
    spectrum.render()
}
