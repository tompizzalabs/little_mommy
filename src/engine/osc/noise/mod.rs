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
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            NoiseColor::White => "White",
            NoiseColor::Pink => "Pink",
            NoiseColor::Brown => "Brown",
        };
        write!(formatter, "{}", name)
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
        let white_sample = self.white();
        let bands = &mut self.pink;
        bands[0] = 0.99886 * bands[0] + white_sample * 0.0555179;
        bands[1] = 0.99332 * bands[1] + white_sample * 0.0750759;
        bands[2] = 0.96900 * bands[2] + white_sample * 0.153_852;
        bands[3] = 0.86715 * bands[3] + white_sample * 0.3104856;
        bands[4] = 0.55000 * bands[4] + white_sample * 0.5329522;
        bands[5] = -0.7616 * bands[5] - white_sample * 0.0168980;
        let output = (bands[0]
            + bands[1]
            + bands[2]
            + bands[3]
            + bands[4]
            + bands[5]
            + bands[6]
            + white_sample * 0.5362)
            * 0.11;
        bands[6] = white_sample * 0.115926;
        output
    }

    fn brown(&mut self) -> f32 {
        let white_sample = self.white();
        self.brown = (self.brown + 0.02 * white_sample) / 1.02;
        self.brown * 3.5
    }
}
