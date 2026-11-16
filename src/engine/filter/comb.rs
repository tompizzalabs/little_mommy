use super::biquad::DcBlocker;

const FREQUENCY_MIN: f32 = 40.0;
const MAX_SAMPLE_RATE: f32 = 192_000.0;
const BUFFER_LENGTH: usize = (MAX_SAMPLE_RATE / FREQUENCY_MIN * 1.1) as usize;
const SMOOTHING_FACTOR: f32 = 0.999;

#[derive(Debug, Clone)]
pub struct CombFilter {
    buffer: Vec<f32>,
    write_index: usize,
    delay_time_control: f32,
    delay_time_smooth: f32,
    dc_blocker: DcBlocker,
}

impl Default for CombFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl CombFilter {
    pub fn new() -> Self {
        Self {
            buffer: vec![0.0; BUFFER_LENGTH],
            write_index: 0,
            delay_time_control: 1.0 / 2000.0,
            delay_time_smooth: 1.0 / 2000.0,
            dc_blocker: DcBlocker::default(),
        }
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_index = 0;
        self.dc_blocker.reset();
        self.delay_time_smooth = self.delay_time_control;
    }

    pub fn process(&mut self, input: f32, frequency: f32, feedback: f32, sample_rate: f32) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }
        self.dc_blocker.set_sample_rate(sample_rate);

        self.delay_time_control = 1.0 / frequency.max(FREQUENCY_MIN);
        self.delay_time_smooth = (self.delay_time_smooth - self.delay_time_control)
            * SMOOTHING_FACTOR
            + self.delay_time_control;
        let delay_time = self.delay_time_smooth.min(1.0 / FREQUENCY_MIN);

        let read_position = self.write_index as f32 - delay_time * sample_rate;
        let read_index = read_position.floor() as i64;
        let next_index = read_index + 1;
        let fraction = read_position - read_position.floor();

        let wrapped = |index: i64| index.rem_euclid(BUFFER_LENGTH as i64) as usize;
        let delayed = (1.0 - fraction) * self.buffer[wrapped(read_index)]
            + fraction * self.buffer[wrapped(next_index)];

        let feedback_modded = feedback.clamp(0.0, 1.0);
        self.buffer[self.write_index] = input + delayed * feedback_modded;
        self.write_index = (self.write_index + 1) % BUFFER_LENGTH;

        let output = (input + delayed) * 0.5;
        self.dc_blocker.process(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn output_stays_finite_with_feedback() {
        let mut filter = CombFilter::new();
        for _ in 0..SAMPLE_RATE as usize {
            let input = ((0.0_f32).sin()) * 0.5;
            let output = filter.process(input, 200.0, 0.9, SAMPLE_RATE);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn feedback_is_clamped() {
        let mut filter = CombFilter::new();
        for _ in 0..SAMPLE_RATE as usize {
            let output = filter.process(0.5, 100.0, 5.0, SAMPLE_RATE);
            assert!(output.is_finite(), "runaway feedback must not explode");
        }
    }

    #[test]
    fn reset_clears_buffer() {
        let mut filter = CombFilter::new();
        for _ in 0..1_000 {
            filter.process(1.0, 200.0, 0.5, SAMPLE_RATE);
        }
        filter.reset();
        assert!(filter.buffer.iter().all(|sample| *sample == 0.0));
    }
}
