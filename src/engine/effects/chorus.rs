use crate::engine::filter::Allpass;

const BUFFER_LENGTH: usize = 44_100;
const MIN_DISTANCE_1: f32 = 0.015;
const MIN_DISTANCE_2: f32 = 0.011;
const AMOUNT_RANGE: f32 = 0.02;
const AMOUNT_SMOOTHING: f32 = 0.995;
const OUTPUT_GAIN: f32 = 0.4;
const ALLPASS_1_RADIUS: f32 = 1.25;
const ALLPASS_1_FREQUENCY: f32 = 1000.0;
const ALLPASS_2_RADIUS: f32 = 1.3;
const ALLPASS_2_FREQUENCY: f32 = 5000.0;

#[derive(Debug, Clone)]
pub struct ChorusState {
    buffer: Vec<f32>,
    write_index: usize,
    allpass_1: Allpass,
    allpass_2: Allpass,
    lfo_position: f32,
    lfo_frequency: f32,
    amount: f32,
    feedback: f32,
}

impl Default for ChorusState {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            write_index: 0,
            allpass_1: Allpass::default(),
            allpass_2: Allpass::default(),
            lfo_position: 0.0,
            lfo_frequency: 0.15,
            amount: 0.16,
            feedback: 0.0,
        }
    }
}

impl ChorusState {
    pub fn process(
        &mut self,
        input: f32,
        rate: f32,
        amount: f32,
        feedback: f32,
        dry_wet: f32,
        sample_rate: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }
        if self.buffer.len() != BUFFER_LENGTH {
            self.buffer = vec![0.0; BUFFER_LENGTH];
        }

        self.lfo_frequency = rate.max(0.001);
        self.amount = (self.amount - amount * amount) * AMOUNT_SMOOTHING + amount * amount;
        self.feedback = feedback.clamp(-1.0, 1.0);
        let dry_wet_modded = dry_wet.clamp(0.0, 1.0);

        self.lfo_position += 2.0 * self.lfo_frequency / sample_rate;
        while self.lfo_position > 2.0 {
            self.lfo_position -= 2.0;
        }
        let lfo_sine = parabola_lfo(self.lfo_position) * 0.5 + 0.5;

        self.allpass_1.set_radius(ALLPASS_1_RADIUS);
        self.allpass_1
            .set_frequency(ALLPASS_1_FREQUENCY, sample_rate);
        self.allpass_2.set_radius(ALLPASS_2_RADIUS);
        self.allpass_2
            .set_frequency(ALLPASS_2_FREQUENCY, sample_rate);

        let line_1 = self.read_delayed(MIN_DISTANCE_1 + lfo_sine * AMOUNT_RANGE * self.amount);
        let line_2 =
            self.read_delayed(MIN_DISTANCE_2 + (1.0 - lfo_sine) * AMOUNT_RANGE * self.amount);

        let mut output = self.allpass_1.process(self.allpass_2.process(line_1));
        output -= line_2;
        output += input * 0.5;
        output *= OUTPUT_GAIN;

        let write_index = self.write_index;
        self.buffer[write_index] = input + output * self.feedback;
        self.write_index = (self.write_index + 1) % BUFFER_LENGTH;

        output * dry_wet_modded + (1.0 - dry_wet_modded) * input
    }

    fn read_delayed(&self, delay_time: f32) -> f32 {
        let sample_rate = BUFFER_LENGTH as f32;
        let read_position = self.write_index as f32 - delay_time * sample_rate;
        let read_index = read_position.floor();
        let fraction = read_position - read_index;
        let first = wrap_index(read_index as i64);
        let next = wrap_index(read_index as i64 + 1);
        (1.0 - fraction) * self.buffer[first] + fraction * self.buffer[next]
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_index = 0;
        self.allpass_1.reset();
        self.allpass_2.reset();
        self.lfo_position = 0.0;
    }
}

fn parabola_lfo(position: f32) -> f32 {
    let sign = if position < 1.0 { 1.0 } else { -1.0 };
    let calc_position = if position > 1.0 {
        position - 1.0
    } else {
        position
    };
    4.0 * (calc_position * (1.0 - calc_position)) * sign
}

fn wrap_index(index: i64) -> usize {
    index.rem_euclid(BUFFER_LENGTH as i64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn output_stays_finite_over_lfo_cycle() {
        let mut state = ChorusState::default();
        for step in 0..SAMPLE_RATE as usize {
            let input = ((step as f32 / SAMPLE_RATE * 220.0 * std::f32::consts::TAU).sin()) * 0.5;
            let output = state.process(input, 0.15, 0.4, 0.2, 1.0, SAMPLE_RATE);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn dry_wet_zero_is_transparent() {
        let mut state = ChorusState::default();
        let output = state.process(0.37, 0.15, 0.4, 0.0, 0.0, SAMPLE_RATE);
        assert!((output - 0.37).abs() < 1e-6);
    }

    #[test]
    fn reset_clears_buffer() {
        let mut state = ChorusState::default();
        for _ in 0..1_000 {
            state.process(1.0, 0.15, 0.4, 0.5, 1.0, SAMPLE_RATE);
        }
        state.reset();
        assert!(state.buffer.iter().all(|sample| *sample == 0.0));
        assert_eq!(state.write_index, 0);
    }
}
