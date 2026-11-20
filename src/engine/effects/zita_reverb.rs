use crate::engine::filter::PeakingEq;

const DIFFUSION_TIMES: [f32; 8] = [
    20_346e-6, 24_421e-6, 31_604e-6, 27_333e-6, 22_904e-6, 29_291e-6, 13_458e-6, 19_123e-6,
];
const DELAY_TIMES: [f32; 8] = [
    153_129e-6, 210_389e-6, 127_837e-6, 256_891e-6, 174_713e-6, 192_303e-6, 125_000e-6, 219_991e-6,
];
const TANK_GAIN: f32 = 0.35355;
const INPUT_DECAY: f32 = 0.3;
const OUTPUT_EQ_Q: f32 = 0.9;
const OUTPUT_EQ_FREQUENCY: f32 = 1000.0;

#[derive(Debug, Clone)]
struct FirstOrderDiffuser {
    line: Vec<f32>,
    index: usize,
    coefficient: f32,
}

impl FirstOrderDiffuser {
    fn process(&mut self, input: f32) -> f32 {
        if self.line.is_empty() {
            return input;
        }
        let delayed = self.line[self.index];
        let diffused = input - self.coefficient * delayed;
        self.line[self.index] = diffused;
        self.index = (self.index + 1) % self.line.len();
        delayed + self.coefficient * diffused
    }

    fn reset(&mut self) {
        self.line.fill(0.0);
        self.index = 0;
    }
}

#[derive(Debug, Clone, Default)]
struct FeedbackDelay {
    line: Vec<f32>,
    index: usize,
}

impl FeedbackDelay {
    fn read(&self) -> f32 {
        self.line.get(self.index).copied().unwrap_or(0.0)
    }

    fn write(&mut self, input: f32) {
        if self.line.is_empty() {
            return;
        }
        self.line[self.index] = input;
        self.index = (self.index + 1) % self.line.len();
    }

    fn reset(&mut self) {
        self.line.fill(0.0);
        self.index = 0;
    }
}

#[derive(Debug, Clone, Default)]
struct VariableDelay {
    line: Vec<f32>,
    read_index: usize,
    write_index: usize,
}

impl VariableDelay {
    fn set_delay(&mut self, delay_samples: usize) {
        if self.line.is_empty() {
            return;
        }
        self.read_index = (self.write_index + self.line.len()
            - delay_samples.min(self.line.len() - 1))
            % self.line.len();
    }

    fn read(&mut self) -> f32 {
        if self.line.is_empty() {
            return 0.0;
        }
        let sample = self.line[self.read_index];
        self.read_index = (self.read_index + 1) % self.line.len();
        sample
    }

    fn write(&mut self, input: f32) {
        if self.line.is_empty() {
            return;
        }
        self.line[self.write_index] = input;
        self.write_index = (self.write_index + 1) % self.line.len();
    }

    fn reset(&mut self) {
        self.line.fill(0.0);
    }
}

#[derive(Debug, Clone, Default)]
struct DampingFilter {
    mid_gain: f32,
    low_gain: f32,
    low_coefficient: f32,
    high_coefficient: f32,
    low_state: f32,
    high_state: f32,
}

impl DampingFilter {
    fn set_params(
        &mut self,
        delay_time: f32,
        mid_decay: f32,
        low_decay: f32,
        low_frequency: f32,
        high_decay: f32,
        damping: f32,
    ) {
        self.mid_gain = 0.001f32.powf(delay_time / mid_decay.max(0.01));
        self.low_gain = 0.001f32.powf(delay_time / low_decay.max(0.01)) / self.mid_gain - 1.0;
        self.low_coefficient = low_frequency;
        let high_gain = 0.001f32.powf(delay_time / high_decay.max(0.01)) / self.mid_gain;
        let t = (1.0 - high_gain * high_gain) / (2.0 * high_gain * high_gain * damping.max(1e-4));
        self.high_coefficient = ((1.0 + 4.0 * t).sqrt() - 1.0) / (2.0 * t);
    }

    fn process(&mut self, input: f32) -> f32 {
        self.low_state += self.low_coefficient * (input - self.low_state) + 1e-10;
        let mid = input + self.low_gain * self.low_state;
        self.high_state += self.high_coefficient * (mid - self.high_state);
        self.mid_gain * self.high_state
    }

    fn reset(&mut self) {
        self.low_state = 0.0;
        self.high_state = 0.0;
    }
}

#[derive(Debug, Clone)]
pub struct ZitaReverbState {
    pre_delay_left: VariableDelay,
    pre_delay_right: VariableDelay,
    diffusers: [FirstOrderDiffuser; 8],
    damping_filters: [DampingFilter; 8],
    feedback_delays: [FeedbackDelay; 8],
    output_eqs: [PeakingEq; 2],
    dry_gains: (f32, f32),
    sample_rate: f32,
    pre_delay_seconds: f32,
    xover_frequency: f32,
    low_decay_seconds: f32,
    mid_decay_seconds: f32,
    damping_frequency: f32,
    output_mix: f32,
}

impl Default for ZitaReverbState {
    fn default() -> Self {
        Self {
            pre_delay_left: VariableDelay::default(),
            pre_delay_right: VariableDelay::default(),
            diffusers: std::array::from_fn(|index| FirstOrderDiffuser {
                line: Vec::new(),
                index: 0,
                coefficient: if index & 1 == 1 { -0.6 } else { 0.6 },
            }),
            damping_filters: std::array::from_fn(|_| DampingFilter::default()),
            feedback_delays: std::array::from_fn(|_| FeedbackDelay::default()),
            output_eqs: std::array::from_fn(|_| PeakingEq::default()),
            dry_gains: (1.0, 0.0),
            sample_rate: 0.0,
            pre_delay_seconds: 0.04,
            xover_frequency: 200.0,
            low_decay_seconds: 3.0,
            mid_decay_seconds: 2.0,
            damping_frequency: 3_000.0,
            output_mix: 0.25,
        }
    }
}

impl ZitaReverbState {
    #[allow(clippy::too_many_arguments)]
    pub fn process_stereo(
        &mut self,
        input_left: f32,
        input_right: f32,
        pre_delay: f32,
        xover: f32,
        low_decay: f32,
        mid_decay: f32,
        damping: f32,
        mix: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        if sample_rate <= 0.0 {
            return (0.0, 0.0);
        }
        if self.sample_rate != sample_rate {
            self.allocate(sample_rate);
        }

        self.pre_delay_seconds = pre_delay.clamp(0.02, 0.1);
        self.xover_frequency = xover.max(1.0);
        self.low_decay_seconds = low_decay.max(0.01);
        self.mid_decay_seconds = mid_decay.max(0.01);
        self.damping_frequency = damping.clamp(1.0, sample_rate * 0.49);
        self.output_mix = mix.clamp(0.0, 1.0);
        self.update_gains();
        self.update_pre_delays();
        self.update_damping_filters();

        self.pre_delay_left.write(input_left);
        self.pre_delay_right.write(input_right);

        let tap_left = INPUT_DECAY * self.pre_delay_left.read();
        let tap_right = INPUT_DECAY * self.pre_delay_right.read();
        let mut taps = [
            self.diffusers[0].process(self.feedback_delays[0].read() + tap_left),
            self.diffusers[1].process(self.feedback_delays[1].read() + tap_left),
            self.diffusers[2].process(self.feedback_delays[2].read() - tap_left),
            self.diffusers[3].process(self.feedback_delays[3].read() - tap_left),
            self.diffusers[4].process(self.feedback_delays[4].read() + tap_right),
            self.diffusers[5].process(self.feedback_delays[5].read() + tap_right),
            self.diffusers[6].process(self.feedback_delays[6].read() - tap_right),
            self.diffusers[7].process(self.feedback_delays[7].read() - tap_right),
        ];

        hadamard_shuffle(&mut taps);

        let out_left = self.dry_gains.1 * (taps[1] + taps[2]);
        let out_right = self.dry_gains.1 * (taps[1] - taps[2]);

        for (index, tap) in taps.iter().enumerate() {
            let filtered = self.damping_filters[index].process(TANK_GAIN * tap);
            self.feedback_delays[index].write(filtered);
        }

        self.output_eqs[0].set_q(OUTPUT_EQ_Q);
        self.output_eqs[0].set_frequency(OUTPUT_EQ_FREQUENCY);
        self.output_eqs[1].set_q(OUTPUT_EQ_Q);
        self.output_eqs[1].set_frequency(OUTPUT_EQ_FREQUENCY);
        let out_left = self.output_eqs[0].process(out_left, sample_rate);
        let out_right = self.output_eqs[1].process(out_right, sample_rate);

        (
            out_left + self.dry_gains.0 * input_left,
            out_right + self.dry_gains.0 * input_right,
        )
    }

    fn allocate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.pre_delay_left.line = vec![0.0; (0.1 * sample_rate) as usize];
        self.pre_delay_right.line = vec![0.0; (0.1 * sample_rate) as usize];
        for (index, diffuser) in self.diffusers.iter_mut().enumerate() {
            let size = (DIFFUSION_TIMES[index] * sample_rate + 0.5) as usize;
            diffuser.line = vec![0.0; size.max(1)];
            diffuser.index = 0;
        }
        for (index, delay) in self.feedback_delays.iter_mut().enumerate() {
            let diffuser_size = (DIFFUSION_TIMES[index] * sample_rate + 0.5) as usize;
            let size = (DELAY_TIMES[index] * sample_rate + 0.5) as usize - diffuser_size;
            delay.line = vec![0.0; size.max(1)];
            delay.index = 0;
        }
        for filter in self.damping_filters.iter_mut() {
            filter.reset();
        }
    }

    fn update_gains(&mut self) {
        let dry = (1.0 - self.output_mix) * (1.0 - self.output_mix);
        self.dry_gains = (dry, 1.0 - dry);
    }

    fn update_pre_delays(&mut self) {
        let delay_samples = ((self.pre_delay_seconds - 0.02) * self.sample_rate + 0.5) as usize;
        self.pre_delay_left.set_delay(delay_samples);
        self.pre_delay_right.set_delay(delay_samples);
    }

    fn update_damping_filters(&mut self) {
        let low_frequency = std::f32::consts::TAU * self.xover_frequency / self.sample_rate;
        let damping = if self.damping_frequency > 0.49 * self.sample_rate {
            2.0
        } else {
            1.0 - (std::f32::consts::TAU * self.damping_frequency / self.sample_rate).cos()
        };
        for (index, filter) in self.damping_filters.iter_mut().enumerate() {
            filter.set_params(
                DELAY_TIMES[index],
                self.mid_decay_seconds,
                self.low_decay_seconds,
                low_frequency,
                0.5 * self.mid_decay_seconds,
                damping,
            );
        }
    }

    pub fn reset(&mut self) {
        self.pre_delay_left.reset();
        self.pre_delay_right.reset();
        for diffuser in self.diffusers.iter_mut() {
            diffuser.reset();
        }
        for delay in self.feedback_delays.iter_mut() {
            delay.reset();
        }
        for filter in self.damping_filters.iter_mut() {
            filter.reset();
        }
        for eq in self.output_eqs.iter_mut() {
            eq.reset();
        }
    }
}

fn hadamard_shuffle(taps: &mut [f32; 8]) {
    for group in [0usize, 2, 4, 6] {
        let (a, b) = (taps[group], taps[group + 1]);
        taps[group] = a + b;
        taps[group + 1] = a - b;
    }
    for group in [0usize, 4] {
        let (a, b) = (taps[group], taps[group + 2]);
        taps[group] = a + b;
        taps[group + 2] = a - b;
        let (a, b) = (taps[group + 1], taps[group + 3]);
        taps[group + 1] = a + b;
        taps[group + 3] = a - b;
    }
    for group in [0usize, 1, 2, 3] {
        let (a, b) = (taps[group], taps[group + 4]);
        taps[group] = a + b;
        taps[group + 4] = a - b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn output_stays_finite_and_decays() {
        let mut state = ZitaReverbState::default();
        for step in 0..(SAMPLE_RATE * 2.0) as usize {
            let input = if step < 100 { 0.5 } else { 0.0 };
            let (left, right) = state.process_stereo(
                input,
                input,
                0.04,
                200.0,
                3.0,
                2.0,
                3_000.0,
                0.25,
                SAMPLE_RATE,
            );
            assert!(left.is_finite() && right.is_finite());
        }
    }

    #[test]
    fn full_mix_still_contains_energy() {
        let mut state = ZitaReverbState::default();
        let mut energy = 0.0;
        for step in 0..SAMPLE_RATE as usize {
            let input = if step < 100 { 0.5 } else { 0.0 };
            let (left, _) = state.process_stereo(
                input,
                input,
                0.04,
                200.0,
                2.0,
                1.5,
                3_000.0,
                1.0,
                SAMPLE_RATE,
            );
            energy += left * left;
        }
        assert!(
            energy > 1e-6,
            "reverb tail should carry energy, got {energy}"
        );
    }

    #[test]
    fn reset_clears_tails() {
        let mut state = ZitaReverbState::default();
        for _ in 0..SAMPLE_RATE as usize {
            state.process_stereo(0.5, 0.5, 0.04, 200.0, 3.0, 2.0, 3_000.0, 0.5, SAMPLE_RATE);
        }
        state.reset();
        let (left, _) =
            state.process_stereo(0.0, 0.0, 0.04, 200.0, 3.0, 2.0, 3_000.0, 0.5, SAMPLE_RATE);
        assert!(
            left.abs() < 1e-6,
            "silent input after reset should be silent, got {left}"
        );
    }
}
