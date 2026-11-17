#[derive(Debug, Clone)]
pub struct BitcrusherState {
    left_value: f32,
    right_value: f32,
    discretization_scalar: f32,
    bits: u8,
}

impl Default for BitcrusherState {
    fn default() -> Self {
        Self {
            left_value: 0.0,
            right_value: 0.0,
            discretization_scalar: 8.0,
            bits: 4,
        }
    }
}

impl BitcrusherState {
    pub fn process_stereo(
        &mut self,
        input_left: f32,
        input_right: f32,
        bits: f32,
        edge: f32,
        dry_wet: f32,
    ) -> (f32, f32) {
        let bits_whole = bits.round().clamp(1.0, 16.0) as u8;
        if bits_whole != self.bits {
            self.bits = bits_whole;
            self.discretization_scalar = 2.0f32.powi(bits_whole as i32 - 1);
        }

        let left = crush_sample(
            input_left,
            &mut self.left_value,
            self.discretization_scalar,
            edge,
            dry_wet,
        );
        let right = crush_sample(
            input_right,
            &mut self.right_value,
            self.discretization_scalar,
            edge,
            dry_wet,
        );
        (left, right)
    }

    pub fn reset(&mut self) {
        self.left_value = 0.0;
        self.right_value = 0.0;
    }
}

fn crush_sample(
    input: f32,
    current_value: &mut f32,
    discretization_scalar: f32,
    edge: f32,
    dry_wet: f32,
) -> f32 {
    let target_value = (input * discretization_scalar).floor() / discretization_scalar;
    *current_value += (target_value - *current_value) * edge.clamp(0.0, 1.0);
    let blend = dry_wet.clamp(0.0, 1.0);
    *current_value * blend + input * (1.0 - blend)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_bit_depth_quantizes_toward_steps() {
        let mut state = BitcrusherState::default();
        let mut distinct_outputs = std::collections::HashSet::new();
        for step in 0..2_000 {
            let input = (step as f32 / 2_000.0) * 2.0 - 1.0;
            let (left, _) = state.process_stereo(input, input, 2.0, 1.0, 1.0);
            distinct_outputs.insert((left * 16.0).round() as i32);
        }
        assert!(
            distinct_outputs.len() <= 8,
            "2-bit crush should give few levels, got {}",
            distinct_outputs.len()
        );
    }

    #[test]
    fn dry_wet_zero_is_transparent() {
        let mut state = BitcrusherState::default();
        let (left, _) = state.process_stereo(0.37, 0.37, 2.0, 1.0, 0.0);
        assert!((left - 0.37).abs() < 1e-6);
    }

    #[test]
    fn high_bit_depth_is_nearly_transparent() {
        let mut state = BitcrusherState::default();
        let (left, _) = state.process_stereo(0.37, 0.37, 16.0, 1.0, 1.0);
        assert!(
            (left - 0.37).abs() < 0.01,
            "16 bits should be near silent quantization, got {left}"
        );
    }
}
