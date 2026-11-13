use fastrand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NoiseColor {
    White,
    Pink,
    Brown,
}

impl NoiseColor {
    pub const ALL: [NoiseColor; 3] = [NoiseColor::White, NoiseColor::Pink, NoiseColor::Brown];
}

impl std::fmt::Display for NoiseColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            NoiseColor::White => "White",
            NoiseColor::Pink => "Pink",
            NoiseColor::Brown => "Brown",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, Clone)]
pub struct NoiseState {
    rng: Rng,
    pub pink: [f32; 7],
    pub brown: f32,
}

impl Default for NoiseState {
    fn default() -> Self {
        Self {
            rng: Rng::with_seed(0),
            pink: [0.0; 7],
            brown: 0.0,
        }
    }
}

impl NoiseState {
    /// Deterministically re-seed the generator (per-voice, per-node).
    pub fn seed(&mut self, seed: u32) {
        self.rng = Rng::with_seed(seed as u64);
    }

    pub fn sample(&mut self, color: NoiseColor) -> f32 {
        match color {
            NoiseColor::White => self.white(),
            NoiseColor::Pink => self.pink(),
            NoiseColor::Brown => self.brown(),
        }
    }

    fn white(&mut self) -> f32 {
        self.rng.f32() * 2.0 - 1.0
    }

    fn pink(&mut self) -> f32 {
        let w = self.white();
        let b = &mut self.pink;
        b[0] = 0.99886 * b[0] + w * 0.0555179;
        b[1] = 0.99332 * b[1] + w * 0.0750759;
        b[2] = 0.96900 * b[2] + w * 0.153_852;
        b[3] = 0.86715 * b[3] + w * 0.3104856;
        b[4] = 0.55000 * b[4] + w * 0.5329522;
        b[5] = -0.7616 * b[5] - w * 0.0168980;
        let out = (b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + w * 0.5362) * 0.11;
        b[6] = w * 0.115926;
        out
    }

    fn brown(&mut self) -> f32 {
        let w = self.white();
        self.brown = (self.brown + 0.02 * w) / 1.02;
        self.brown * 3.5
    }
}
