use crate::engine::filter::{DcBlocker, VaOnePole};

const MOVING_AVERAGE_LENGTH: f32 = 441.0;
const DUCKING_SCALE_FACTOR: f32 = 5.0;
const DELAY_TIME_SMOOTHING: f32 = 0.99994;
const HIGHPASS_FREQUENCY: f32 = 80.0;

#[derive(Debug, Clone)]
pub struct DelayState {
    buffer_left: Vec<f32>,
    buffer_right: Vec<f32>,
    write_index: usize,
    delay_time_control: f32,
    delay_time_smooth: f32,
    feedback: f32,
    dry: f32,
    wet: f32,
    ducking: f32,
    ping_pong: bool,
    highpass_left: VaOnePole,
    highpass_right: VaOnePole,
    dc_blocker_left: DcBlocker,
    dc_blocker_right: DcBlocker,
    moving_average_sum: f32,
    moving_average_mean: f32,
}

impl Default for DelayState {
    fn default() -> Self {
        let mut highpass_left = VaOnePole::new();
        let mut highpass_right = VaOnePole::new();
        highpass_left.set_highpass();
        highpass_right.set_highpass();
        Self {
            buffer_left: Vec::new(),
            buffer_right: Vec::new(),
            write_index: 0,
            delay_time_control: 1.0,
            delay_time_smooth: 1.0,
            feedback: 0.45,
            dry: 1.0,
            wet: 0.7,
            ducking: 0.0,
            ping_pong: false,
            highpass_left,
            highpass_right,
            dc_blocker_left: DcBlocker::default(),
            dc_blocker_right: DcBlocker::default(),
            moving_average_sum: 0.0,
            moving_average_mean: 0.0,
        }
    }
}

impl DelayState {
    pub fn process_stereo(
        &mut self,
        input_left: f32,
        input_right: f32,
        delay_time: f32,
        feedback: f32,
        dry: f32,
        wet: f32,
        ducking: f32,
        ping_pong: bool,
        sample_rate: f32,
    ) -> (f32, f32) {
        if sample_rate <= 0.0 {
            return (0.0, 0.0);
        }
        if !self.ensure_buffers(sample_rate) {
            return (input_left, input_right);
        }

        self.delay_time_control =
            delay_time.clamp(0.0, crate::engine::effects::MAX_DELAY_TIME_SECONDS);
        self.delay_time_smooth = (self.delay_time_smooth - self.delay_time_control)
            * DELAY_TIME_SMOOTHING
            + self.delay_time_control;

        self.feedback = feedback.clamp(0.0, 1.0);
        self.dry = dry.clamp(0.0, 1.0);
        self.wet = wet.clamp(0.0, 1.0);
        self.ducking = ducking.clamp(0.0, 1.0);
        self.ping_pong = ping_pong;

        self.highpass_left
            .set_frequency(HIGHPASS_FREQUENCY, sample_rate);
        self.highpass_right
            .set_frequency(HIGHPASS_FREQUENCY, sample_rate);
        self.dc_blocker_left.set_sample_rate(sample_rate);
        self.dc_blocker_right.set_sample_rate(sample_rate);

        let delayed_left = self.read_interpolated(&self.buffer_left);
        let write_index = self.write_index;
        if self.ping_pong {
            self.buffer_left[write_index] = input_left * 0.5;
            self.buffer_right[write_index] = delayed_left * self.feedback;
        } else {
            self.buffer_left[write_index] = input_left + delayed_left * self.feedback;
        }
        let highpassed_left = self.highpass_left.process(delayed_left);
        let ducked_left = highpassed_left * (1.0 - self.ducking_gain(input_left) * self.ducking);
        let output_left = self.dc_blocker_left.process(ducked_left);

        let delayed_right = self.read_interpolated(&self.buffer_right);
        let write_index = self.write_index;
        if self.ping_pong {
            self.buffer_left[write_index] += input_right * 0.5 + delayed_right * self.feedback;
        } else {
            self.buffer_right[write_index] = input_right + delayed_right * self.feedback;
        }
        self.write_index = (self.write_index + 1) % self.buffer_left.len();

        let highpassed_right = self.highpass_right.process(delayed_right);
        let ducked_right = highpassed_right * (1.0 - self.ducking_gain(input_right) * self.ducking);
        let output_right = self.dc_blocker_right.process(ducked_right);

        let left = output_left * self.wet + input_left * self.dry;
        let right = output_right * self.wet + input_right * self.dry;
        (left, right)
    }

    fn ensure_buffers(&mut self, sample_rate: f32) -> bool {
        let active_length = (sample_rate * crate::engine::effects::MAX_DELAY_TIME_SECONDS) as usize;
        if active_length == 0 {
            return false;
        }
        if self.buffer_left.len() != active_length {
            self.buffer_left = vec![0.0; active_length];
            self.buffer_right = vec![0.0; active_length];
            self.write_index = 0;
            self.delay_time_smooth = self.delay_time_control;
        }
        true
    }

    fn read_interpolated(&self, buffer: &[f32]) -> f32 {
        let read_position =
            self.write_index as f32 - self.delay_time_smooth * sample_rate_of(buffer);
        let read_index = read_position.floor();
        let fraction = read_position - read_index;
        let length = buffer.len() as i64;
        let first = (read_index as i64).rem_euclid(length) as usize;
        let next = (read_index as i64 + 1).rem_euclid(length) as usize;
        (1.0 - fraction) * buffer[first] + fraction * buffer[next]
    }

    fn ducking_gain(&mut self, input: f32) -> f32 {
        let absolute = input.abs();
        self.moving_average_sum += absolute - self.moving_average_mean;
        self.moving_average_mean = self.moving_average_sum / MOVING_AVERAGE_LENGTH;
        (self.moving_average_mean * DUCKING_SCALE_FACTOR).min(1.0)
    }

    pub fn reset(&mut self) {
        self.buffer_left.fill(0.0);
        self.buffer_right.fill(0.0);
        self.write_index = 0;
        self.delay_time_smooth = self.delay_time_control;
        self.highpass_left.reset();
        self.highpass_right.reset();
        self.dc_blocker_left.reset();
        self.dc_blocker_right.reset();
        self.moving_average_sum = 0.0;
        self.moving_average_mean = 0.0;
    }
}

fn sample_rate_of(buffer: &[f32]) -> f32 {
    buffer.len() as f32 / crate::engine::effects::MAX_DELAY_TIME_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn passes_dry_signal_with_zero_wet() {
        let mut state = DelayState::default();
        let (left, right) =
            state.process_stereo(0.4, -0.4, 0.1, 0.3, 1.0, 0.0, 0.0, false, SAMPLE_RATE);
        assert!((left - 0.4).abs() < 1e-6);
        assert!((right - -0.4).abs() < 1e-6);
    }

    #[test]
    fn wet_path_stays_finite_with_full_feedback() {
        let mut state = DelayState::default();
        for step in 0..SAMPLE_RATE as usize {
            let input = ((step as f32 / SAMPLE_RATE * 220.0 * std::f32::consts::TAU).sin()) * 0.5;
            let (left, right) =
                state.process_stereo(input, input, 0.25, 1.0, 0.5, 0.5, 0.5, true, SAMPLE_RATE);
            assert!(left.is_finite() && right.is_finite());
        }
    }

    #[test]
    fn reset_clears_buffers() {
        let mut state = DelayState::default();
        for _ in 0..1_000 {
            state.process_stereo(1.0, 1.0, 0.1, 0.5, 0.5, 0.5, 0.0, false, SAMPLE_RATE);
        }
        state.reset();
        assert!(state.buffer_left.iter().all(|sample| *sample == 0.0));
        assert!(state.buffer_right.iter().all(|sample| *sample == 0.0));
    }
}
