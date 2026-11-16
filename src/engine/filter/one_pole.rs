#[derive(Debug, Clone)]
pub struct VaOnePole {
    pub alpha: f32,
    pub beta: f32,
    pub gamma: f32,
    pub delta: f32,
    pub epsilon: f32,
    pub input_gain: f32,
    pub feedback: f32,
    is_lowpass: bool,
    z1: f32,
}

impl Default for VaOnePole {
    fn default() -> Self {
        Self::new()
    }
}

impl VaOnePole {
    pub fn new() -> Self {
        Self {
            alpha: 1.0,
            beta: 0.0,
            gamma: 1.0,
            delta: 0.0,
            epsilon: 0.0,
            input_gain: 1.0,
            feedback: 0.0,
            is_lowpass: true,
            z1: 0.0,
        }
    }

    pub fn set_lowpass(&mut self) {
        self.is_lowpass = true;
    }

    pub fn set_highpass(&mut self) {
        self.is_lowpass = false;
    }

    pub fn set_frequency(&mut self, frequency: f32, sample_rate: f32) {
        let warped = (2.0 * sample_rate) * (frequency * std::f32::consts::TAU / sample_rate / 2.0).tan();
        let g = warped / sample_rate * 0.5;
        self.alpha = g / (1.0 + g);
    }

    pub fn feedback_output(&self) -> f32 {
        self.beta * (self.z1 + self.feedback * self.delta)
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.feedback = 0.0;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let input = input * self.gamma + self.feedback + self.epsilon * self.feedback_output();
        let filtered = (self.input_gain * input - self.z1) * self.alpha;
        let low_pass = filtered + self.z1;
        self.z1 = filtered + low_pass;
        if self.is_lowpass {
            return low_pass;
        }
        input - low_pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_pass_attenuates_high_frequencies() {
        let cutoff = 500.0;
        let sample_rate = 48_000.0;
        let mut filter = VaOnePole::new();
        filter.set_frequency(cutoff, sample_rate);

        let mut input_energy = 0.0;
        let mut output_energy = 0.0;
        for step in 0..sample_rate as usize {
            let input = if (step / 5) % 2 == 0 { 1.0 } else { -1.0 };
            let output = filter.process(input);
            input_energy += input * input;
            output_energy += output * output;
        }

        let input_rms = (input_energy / sample_rate).sqrt();
        let output_rms = (output_energy / sample_rate).sqrt();
        assert!(
            output_rms < input_rms * 0.1,
            "4.8 kHz square should collapse into a 500 Hz low pass: {output_rms} vs {input_rms}"
        );
    }

    #[test]
    fn high_pass_blocks_direct_current() {
        let mut filter = VaOnePole::new();
        filter.set_highpass();
        filter.set_frequency(200.0, 48_000.0);

        let mut tail = 0.0;
        for _ in 0..48_000 {
            tail = filter.process(0.5);
        }
        assert!(tail.abs() < 0.01, "constant input should vanish, got {tail}");
    }

    #[test]
    fn reset_clears_state() {
        let mut filter = VaOnePole::new();
        filter.set_frequency(500.0, 48_000.0);
        for _ in 0..1_000 {
            filter.process(1.0);
        }
        filter.reset();
        let fresh = VaOnePole::new();
        assert_eq!(filter.z1, fresh.z1);
        assert_eq!(filter.feedback, fresh.feedback);
    }
}
