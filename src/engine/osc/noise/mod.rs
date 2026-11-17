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
    held: HeldRandom,
}

#[derive(Debug, Clone)]
struct HeldRandom {
    value: f32,
    target: f32,
    counter: u32,
    interval: u32,
}

impl Default for NoiseState {
    fn default() -> Self {
        Self {
            rng: Rng::with_seed(0),
            pink: [0.0; 7],
            brown: 0.0,
            held: HeldRandom {
                value: 0.0,
                target: 0.0,
                counter: 0,
                interval: 1,
            },
        }
    }
}

impl NoiseState {
    pub fn seed(&mut self, seed: u32) {
        self.rng = Rng::with_seed(seed as u64);
    }

    pub fn sample(&mut self, color: NoiseColor, frequency: f32, sample_rate: f32) -> f32 {
        let generated = match color {
            NoiseColor::White => self.white(),
            NoiseColor::Pink => self.pink(),
            NoiseColor::Brown => self.brown(),
        };

        let interval = if frequency > 0.0 && sample_rate > 0.0 {
            ((sample_rate / frequency) as u32).max(1)
        } else {
            1
        };
        if interval != self.held.interval {
            self.held.interval = interval;
            self.held.counter = 0;
        }
        self.held.counter += 1;
        if self.held.counter >= interval {
            self.held.counter = 0;
            self.held.target = generated;
        }

        let glide = 1.0 - (-12.0 / self.held.interval as f32).exp();
        self.held.value += (self.held.target - self.held.value) * glide;
        self.held.value
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_rate_stays_noise_like() {
        let mut state = NoiseState::default();
        state.seed(9);
        let mut movement = 0.0f32;
        let mut previous = 0.0f32;
        for _ in 0..4_800 {
            let sample = state.sample(NoiseColor::White, 20_000.0, 48_000.0);
            assert!(sample.is_finite() && sample.abs() <= 1.5);
            movement += (sample - previous).abs();
            previous = sample;
        }
        assert!(
            movement > 800.0,
            "audio-rate noise should still jump around, movement {movement}"
        );
    }

    #[test]
    fn slow_rate_glides_smoothly() {
        let mut state = NoiseState::default();
        state.seed(11);
        let mut movement = 0.0f32;
        let mut previous = 0.0f32;
        for _ in 0..4_800 {
            let sample = state.sample(NoiseColor::White, 5.0, 48_000.0);
            assert!(sample.is_finite() && sample.abs() <= 1.5);
            movement += (sample - previous).abs();
            previous = sample;
        }
        assert!(
            movement < 400.0,
            "slow-rate noise should glide gently, movement {movement}"
        );
    }

    #[test]
    fn zero_frequency_is_safe() {
        let mut state = NoiseState::default();
        let sample = state.sample(NoiseColor::Pink, 0.0, 0.0);
        assert!(sample.is_finite());
    }
}
