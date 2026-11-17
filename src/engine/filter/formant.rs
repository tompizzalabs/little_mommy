use super::biquad::Resonator;

pub const VOWEL_COUNT: usize = 8;

const OUTPUT_SCALAR: f32 = 0.35;
const RESONATOR_RADIUS: f32 = 0.996;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vowel {
    A,
    E,
    I,
    O,
    U,
    Ae,
    Oe,
    Ue,
}

impl Vowel {
    pub const ALL: [Vowel; VOWEL_COUNT] = [
        Vowel::A,
        Vowel::E,
        Vowel::I,
        Vowel::O,
        Vowel::U,
        Vowel::Ae,
        Vowel::Oe,
        Vowel::Ue,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Vowel::A => "A",
            Vowel::E => "E",
            Vowel::I => "I",
            Vowel::O => "O",
            Vowel::U => "U",
            Vowel::Ae => "Ä",
            Vowel::Oe => "Ö",
            Vowel::Ue => "Ü",
        }
    }

    fn formants(self) -> (f32, f32) {
        match self {
            Vowel::A => (1000.0, 1400.0),
            Vowel::E => (500.0, 2300.0),
            Vowel::I => (320.0, 3200.0),
            Vowel::O => (500.0, 1000.0),
            Vowel::U => (320.0, 800.0),
            Vowel::Ae => (700.0, 1800.0),
            Vowel::Oe => (500.0, 1500.0),
            Vowel::Ue => (320.0, 1650.0),
        }
    }

    fn from_index(index: usize) -> Vowel {
        Vowel::ALL[index.min(VOWEL_COUNT - 1)]
    }
}

#[derive(Debug, Clone)]
pub struct FormantFilter {
    resonator_1: Resonator,
    resonator_2: Resonator,
    parabola_1: [f32; 3],
    parabola_2: [f32; 3],
}

impl Default for FormantFilter {
    fn default() -> Self {
        let mut filter = Self {
            resonator_1: Resonator::default(),
            resonator_2: Resonator::default(),
            parabola_1: [0.0; 3],
            parabola_2: [0.0; 3],
        };
        filter.update_parabolas(Vowel::A, Vowel::E, 48_000.0);
        filter
    }
}

impl FormantFilter {
    fn update_parabolas(&mut self, vowel_left: Vowel, vowel_right: Vowel, sample_rate: f32) {
        self.parabola_1 = parabola_coefficients(vowel_left.formants().0, vowel_right.formants().0);
        self.parabola_2 = parabola_coefficients(vowel_left.formants().1, vowel_right.formants().1);
        self.resonator_1.set_radius(RESONATOR_RADIUS, sample_rate);
        self.resonator_2.set_radius(RESONATOR_RADIUS, sample_rate);
    }

    pub fn process(
        &mut self,
        input: f32,
        transition: f32,
        vowel_left: usize,
        vowel_right: usize,
        sample_rate: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        self.update_parabolas(
            Vowel::from_index(vowel_left),
            Vowel::from_index(vowel_right),
            sample_rate,
        );

        let t = transition.clamp(0.0, 1.0);
        let [a0, b0, c0] = self.parabola_1;
        let [a1, b1, c1] = self.parabola_2;
        self.resonator_1
            .set_frequency(a0 * t * t + b0 * t + c0, sample_rate);
        self.resonator_2
            .set_frequency(a1 * t * t + b1 * t + c1, sample_rate);

        let output = self.resonator_1.process(self.resonator_2.process(input));
        output * OUTPUT_SCALAR
    }

    pub fn reset(&mut self) {
        self.resonator_1.reset();
        self.resonator_2.reset();
    }
}

fn parabola_coefficients(left: f32, right: f32) -> [f32; 3] {
    let midpoint = left * (right / left).sqrt();
    let a = 2.0 * right - 4.0 * midpoint + 2.0 * left;
    let b = 4.0 * midpoint - 3.0 * left - right;
    let c = left;
    [a, b, c]
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    #[test]
    fn output_stays_finite_across_vowel_sweep() {
        let mut filter = FormantFilter::default();
        for step in 0..SAMPLE_RATE as usize {
            let transition = (step % 1_000) as f32 / 1_000.0;
            let output = filter.process(0.5, transition, 0, 3, SAMPLE_RATE);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn every_vowel_pair_produces_sound() {
        for left in 0..VOWEL_COUNT {
            for right in 0..VOWEL_COUNT {
                let mut filter = FormantFilter::default();
                let mut energy = 0.0;
                for step in 0..2_000 {
                    let input =
                        ((step as f32 / SAMPLE_RATE * 200.0 * std::f32::consts::TAU).sin()) * 0.5;
                    energy += filter.process(input, 0.5, left, right, SAMPLE_RATE).powi(2);
                }
                assert!(energy > 0.0, "vowel pair {left}/{right} should pass signal");
            }
        }
    }
}
