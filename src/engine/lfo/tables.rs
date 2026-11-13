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
        for k in 1..=HARMONICS {
            let kf = k as f32;
            let (s, c) = (TAU * kf * duty).sin_cos();
            let g = 2.0 * gain / (PI * kf);
            self.cos[k - 1] += g * s;
            self.sin[k - 1] += g * (1.0 - c);
        }
    }

    fn render(self) -> [f32; LFO_TABLE_LENGTH] {
        let mut out = [0.0; LFO_TABLE_LENGTH];
        for (i, slot) in out.iter_mut().enumerate() {
            let t = i as f32 / LFO_TABLE_LENGTH as f32;
            let mut v = self.dc;
            for k in 1..=HARMONICS {
                let (s, c) = (TAU * k as f32 * t).sin_cos();
                v += self.cos[k - 1] * c + self.sin[k - 1] * s;
            }
            *slot = v;
        }
        out
    }
}

fn square(duty: f32) -> [f32; LFO_TABLE_LENGTH] {
    let mut s = Spectrum::new();
    s.pulse(duty, 1.0);
    s.render()
}

fn stair(n: usize) -> [f32; LFO_TABLE_LENGTH] {
    let mut s = Spectrum::new();
    let g = -1.0 / (n as f32 - 1.0);
    for j in 1..n {
        s.pulse(j as f32 / n as f32, g);
    }
    s.render()
}

fn pyramid(n: usize) -> [f32; LFO_TABLE_LENGTH] {
    let mut s = Spectrum::new();
    let g = 2.0 / n as f32;
    s.dc += g - 1.0;
    for j in 1..n {
        s.pulse(j as f32 / n as f32, if j <= n / 2 { -g } else { g });
    }
    s.render()
}

fn spike() -> [f32; LFO_TABLE_LENGTH] {
    let mut s = Spectrum::new();
    s.dc = -1.0 / 3.0;
    for k in 1..=HARMONICS {
        let kf = k as f32;
        s.cos[k - 1] = 8.0 / (PI * PI * kf * kf) * if k % 2 == 0 { 1.0 } else { -1.0 };
    }
    s.render()
}
