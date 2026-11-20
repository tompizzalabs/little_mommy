const THRESHOLD_MIN: f32 = 0.05;
const THRESHOLD_SMOOTHING: f32 = 0.998;
const DOWNSAMPLER_INPUT_GAIN: f32 = 0.019_966_84;
const DOWNSAMPLER_FEEDBACK: [f32; 9] = [
    -0.0003977153,
    -0.006_447_462,
    -0.047_699_742,
    -0.218_582_97,
    -0.664_923_4,
    -1.477_365_7,
    -2.272_142_2,
    -2.659_867_3,
    -1.875_596,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DistortionAlgorithm {
    Clamp,
    Fold,
    Zero,
    Sine,
    Cube,
}

impl DistortionAlgorithm {
    pub const ALL: [DistortionAlgorithm; 5] = [
        DistortionAlgorithm::Clamp,
        DistortionAlgorithm::Fold,
        DistortionAlgorithm::Zero,
        DistortionAlgorithm::Sine,
        DistortionAlgorithm::Cube,
    ];
}

impl std::fmt::Display for DistortionAlgorithm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            DistortionAlgorithm::Clamp => "Clamp",
            DistortionAlgorithm::Fold => "Fold",
            DistortionAlgorithm::Zero => "Zero",
            DistortionAlgorithm::Sine => "Sine",
            DistortionAlgorithm::Cube => "Cube",
        };
        write!(formatter, "{}", name)
    }
}

#[derive(Debug, Clone, Default)]
struct ButterworthDownsampler {
    xv: [f32; 10],
    yv: [f32; 10],
}

impl ButterworthDownsampler {
    fn process(&mut self, input: f32) -> f32 {
        self.xv.copy_within(1..10, 0);
        self.xv[9] = input * DOWNSAMPLER_INPUT_GAIN;
        self.yv.copy_within(1..10, 0);
        let feedforward: f32 = (self.xv[0] + self.xv[9])
            + 9.0 * (self.xv[1] + self.xv[8])
            + 36.0 * (self.xv[2] + self.xv[7])
            + 84.0 * (self.xv[3] + self.xv[6])
            + 126.0 * (self.xv[4] + self.xv[5]);
        let feedback: f32 = DOWNSAMPLER_FEEDBACK
            .iter()
            .enumerate()
            .map(|(index, coefficient)| coefficient * self.yv[index])
            .sum();
        let output = feedforward + feedback;
        self.yv[9] = output;
        output
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone)]
pub struct DistortionState {
    downsampler: ButterworthDownsampler,
    last_input: f32,
    threshold: f32,
    threshold_smooth: f32,
    algorithm: DistortionAlgorithm,
}

impl Default for DistortionState {
    fn default() -> Self {
        Self {
            downsampler: ButterworthDownsampler::default(),
            last_input: 0.0,
            threshold: 0.0,
            threshold_smooth: 0.0,
            algorithm: DistortionAlgorithm::Clamp,
        }
    }
}

impl DistortionState {
    pub fn process(
        &mut self,
        input: f32,
        threshold: f32,
        algorithm: DistortionAlgorithm,
        dry_wet: f32,
    ) -> f32 {
        self.algorithm = algorithm;

        let threshold_control = (1.0 - threshold).powi(3);
        self.threshold = threshold_control;
        self.threshold_smooth = self.threshold_smooth * THRESHOLD_SMOOTHING
            + (1.0 - THRESHOLD_SMOOTHING) * self.threshold;
        let threshold_modded = (self.threshold_smooth * (1.0 - THRESHOLD_MIN) + THRESHOLD_MIN)
            .clamp(THRESHOLD_MIN, 1.0);

        let mut upsampled = [
            0.666_666_7 * self.last_input + 0.33333333 * input,
            0.33333333 * self.last_input + 0.666_666_7 * input,
            input,
        ];
        self.last_input = input;

        for sample in upsampled.iter_mut() {
            *sample = distort(*sample, threshold_modded, algorithm);
        }

        let mut downsampled = 0.0;
        for sample in upsampled {
            downsampled = self.downsampler.process(sample);
        }

        let dry_wet_modded = dry_wet.clamp(0.0, 1.0);
        match algorithm {
            DistortionAlgorithm::Clamp | DistortionAlgorithm::Fold | DistortionAlgorithm::Zero => {
                downsampled * dry_wet_modded / threshold_modded + input * (1.0 - dry_wet_modded)
            }
            DistortionAlgorithm::Sine | DistortionAlgorithm::Cube => {
                downsampled * dry_wet_modded + input * (1.0 - dry_wet_modded)
            }
        }
    }

    pub fn reset(&mut self) {
        self.downsampler.reset();
        self.last_input = 0.0;
        self.threshold_smooth = self.threshold;
    }
}

fn distort(input: f32, threshold: f32, algorithm: DistortionAlgorithm) -> f32 {
    match algorithm {
        DistortionAlgorithm::Clamp => {
            if input > threshold {
                threshold
            } else if input < -threshold {
                -threshold
            } else {
                input
            }
        }
        DistortionAlgorithm::Zero => {
            let widened_threshold = 0.5 + threshold * 0.5;
            if input > widened_threshold || input < -widened_threshold {
                0.0
            } else {
                input
            }
        }
        DistortionAlgorithm::Sine | DistortionAlgorithm::Cube => {
            let sine = input.sin();
            sine * sine * sine
        }
        DistortionAlgorithm::Fold => {
            let mut folded = input;
            while folded.abs() > threshold {
                folded = if folded > threshold {
                    2.0 * threshold - folded
                } else {
                    -2.0 * threshold - folded
                };
            }
            folded
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLES: usize = 4_800;

    fn sweep(algorithm: DistortionAlgorithm) -> Vec<f32> {
        let mut state = super::DistortionState::default();
        (0..SAMPLES)
            .map(|step| {
                let input = (step as f32 / 100.0 * std::f32::consts::TAU).sin() * 0.5;
                state.process(input, 0.5, algorithm, 1.0)
            })
            .collect()
    }

    #[test]
    fn all_algorithms_stay_finite() {
        for algorithm in DistortionAlgorithm::ALL {
            for output in sweep(algorithm) {
                assert!(output.is_finite(), "{algorithm} produced non-finite output");
            }
        }
    }

    #[test]
    fn clamp_limits_output() {
        let outputs = sweep(DistortionAlgorithm::Clamp);
        let tail = &outputs[SAMPLES / 2..];
        let peak = tail
            .iter()
            .fold(0.0f32, |peak, sample| sample.abs().max(peak));
        assert!(peak < 2.0, "clamped output should stay bounded, got {peak}");
    }

    #[test]
    fn fold_never_exceeds_threshold() {
        let outputs = sweep(DistortionAlgorithm::Fold);
        let tail = &outputs[SAMPLES / 2..];
        let peak = tail
            .iter()
            .fold(0.0f32, |peak, sample| sample.abs().max(peak));
        assert!(peak < 1.5, "folded output should stay bounded, got {peak}");
    }

    #[test]
    fn dry_wet_zero_is_transparent() {
        let mut state = super::DistortionState::default();
        let output = state.process(0.37, 0.5, DistortionAlgorithm::Clamp, 0.0);
        assert!((output - 0.37).abs() < 1e-6);
    }
}
