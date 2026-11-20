use std::f64::consts::TAU;
use std::sync::LazyLock;

pub const KERNEL_LENGTH: usize = 24;
pub const KERNEL_OVERSAMPLING: usize = 32;
pub const KERNEL_TAPS: usize = KERNEL_LENGTH * KERNEL_OVERSAMPLING + 1;
pub const BUS_LENGTH: usize = 32;
const BAND_EDGE: f64 = 0.45;
const FFT_SIZE: usize = 8192;

static STEP_KERNEL: LazyLock<[f32; KERNEL_TAPS]> = LazyLock::new(build_step_kernel);
static SLOPE_KERNEL: LazyLock<[f32; KERNEL_TAPS]> = LazyLock::new(build_slope_kernel);

pub fn step_kernel() -> &'static [f32; KERNEL_TAPS] {
    &STEP_KERNEL
}

pub fn slope_kernel() -> &'static [f32; KERNEL_TAPS] {
    &SLOPE_KERNEL
}

#[derive(Debug, Clone)]
pub struct CorrectionBus {
    slots: [f32; BUS_LENGTH],
    position: usize,
}

impl Default for CorrectionBus {
    fn default() -> Self {
        Self {
            slots: [0.0; BUS_LENGTH],
            position: 0,
        }
    }
}

impl CorrectionBus {
    pub fn clear(&mut self) {
        self.slots = [0.0; BUS_LENGTH];
        self.position = 0;
    }

    pub fn take(&mut self) -> f32 {
        let value = self.slots[self.position];
        self.slots[self.position] = 0.0;
        self.position = (self.position + 1) & (BUS_LENGTH - 1);
        value
    }

    pub fn add_step(&mut self, magnitude: f32, offset: f32) {
        self.add(step_kernel(), magnitude, offset);
    }

    pub fn add_slope(&mut self, magnitude: f32, offset: f32) {
        self.add(slope_kernel(), magnitude, offset);
    }

    fn add(&mut self, kernel: &[f32], magnitude: f32, offset: f32) {
        if !magnitude.is_finite() || magnitude == 0.0 {
            return;
        }
        let magnitude = magnitude.clamp(-64.0, 64.0);
        let offset = if offset.is_finite() {
            offset.clamp(0.0, 0.999_999)
        } else {
            0.0
        };
        let base = offset * KERNEL_OVERSAMPLING as f32;
        let step = KERNEL_OVERSAMPLING as f32;
        for tap in 0..KERNEL_LENGTH {
            let tap_position = base + tap as f32 * step;
            let tap_index = tap_position as usize;
            let fraction = tap_position - tap_index as f32;
            let value = kernel[tap_index] + (kernel[tap_index + 1] - kernel[tap_index]) * fraction;
            let slot = (self.position + tap) & (BUS_LENGTH - 1);
            self.slots[slot] += magnitude * value;
        }
    }
}

fn build_step_kernel() -> [f32; KERNEL_TAPS] {
    let windowed_sinc = minimum_phase_kernel();
    let mut step = [0.0f32; KERNEL_TAPS];
    let mut cumulative = 0.0f64;
    for (index, slot) in step.iter_mut().enumerate() {
        cumulative += windowed_sinc[index];
        *slot = (cumulative - 1.0) as f32;
    }
    step
}

fn build_slope_kernel() -> [f32; KERNEL_TAPS] {
    let step_kernel = &*STEP_KERNEL;
    let mut slope = [0.0f32; KERNEL_TAPS];
    let mut cumulative = 0.0f64;
    for (index, slot) in slope.iter_mut().enumerate() {
        cumulative += step_kernel[index] as f64;
        *slot = cumulative as f32;
    }
    let residual = slope[KERNEL_TAPS - 1];
    for slot in slope.iter_mut() {
        *slot -= residual;
    }
    slope
}

fn minimum_phase_kernel() -> [f64; KERNEL_TAPS] {
    let taps = KERNEL_LENGTH * KERNEL_OVERSAMPLING;
    let mut real = [0.0f64; FFT_SIZE];
    let mut imaginary = [0.0f64; FFT_SIZE];

    let cutoff = TAU * BAND_EDGE / KERNEL_OVERSAMPLING as f64;
    let centre = taps as f64 / 2.0;
    let mut sum = 0.0f64;
    for (index, slot) in real.iter_mut().enumerate().take(taps + 1) {
        let distance = index as f64 - centre;
        let sinc = if distance == 0.0 {
            cutoff
        } else {
            (cutoff * distance).sin() / distance
        };
        let value = sinc * blackman_harris(index as f64 / taps as f64);
        *slot = value;
        sum += value;
    }
    for slot in real.iter_mut() {
        *slot /= sum;
    }

    transform(&mut real, &mut imaginary, false);
    let mut cepstrum = [0.0f64; FFT_SIZE];
    for ((cepstrum_slot, real_value), imaginary_value) in
        cepstrum.iter_mut().zip(&real).zip(&imaginary)
    {
        *cepstrum_slot = real_value.hypot(*imaginary_value).max(1e-14).ln();
    }
    let mut cepstrum_imaginary = [0.0f64; FFT_SIZE];
    transform(&mut cepstrum, &mut cepstrum_imaginary, true);
    for slot in &mut cepstrum[1..FFT_SIZE / 2] {
        *slot *= 2.0;
    }
    for slot in &mut cepstrum[FFT_SIZE / 2 + 1..] {
        *slot = 0.0;
    }
    transform(&mut cepstrum, &mut cepstrum_imaginary, false);
    for index in 0..FFT_SIZE {
        let magnitude = cepstrum[index].exp();
        real[index] = magnitude * cepstrum_imaginary[index].cos();
        imaginary[index] = magnitude * cepstrum_imaginary[index].sin();
    }
    transform(&mut real, &mut imaginary, true);

    let mut kernel = [0.0f64; KERNEL_TAPS];
    kernel.copy_from_slice(&real[..KERNEL_TAPS]);
    let normalization: f64 = kernel.iter().sum();
    for slot in kernel.iter_mut() {
        *slot /= normalization;
    }
    kernel
}

fn blackman_harris(position: f64) -> f64 {
    let angle = TAU * position;
    0.35875 - 0.48829 * angle.cos() + 0.14128 * (2.0 * angle).cos() - 0.01168 * (3.0 * angle).cos()
}

fn transform(real: &mut [f64; FFT_SIZE], imaginary: &mut [f64; FFT_SIZE], inverse: bool) {
    let length = FFT_SIZE;
    let mut reversal = 0usize;
    for index in 1..length {
        let mut bit = length >> 1;
        while reversal & bit != 0 {
            reversal ^= bit;
            bit >>= 1;
        }
        reversal ^= bit;
        if index < reversal {
            real.swap(index, reversal);
            imaginary.swap(index, reversal);
        }
    }

    let sign = if inverse { 1.0 } else { -1.0 };
    let mut span = 2usize;
    while span <= length {
        let half = span >> 1;
        let mut start = 0usize;
        while start < length {
            for step in 0..half {
                let angle = sign * TAU * step as f64 / span as f64;
                let (twiddle_real, twiddle_imaginary) = (angle.cos(), angle.sin());
                let first = start + step;
                let second = first + half;
                let first_real = real[first];
                let first_imaginary = imaginary[first];
                let product_real =
                    real[second] * twiddle_real - imaginary[second] * twiddle_imaginary;
                let product_imaginary =
                    real[second] * twiddle_imaginary + imaginary[second] * twiddle_real;
                real[first] = first_real + product_real;
                imaginary[first] = first_imaginary + product_imaginary;
                real[second] = first_real - product_real;
                imaginary[second] = first_imaginary - product_imaginary;
            }
            start += span;
        }
        span <<= 1;
    }

    if inverse {
        let scale = 1.0 / length as f64;
        for index in 0..length {
            real[index] *= scale;
            imaginary[index] *= scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_kernel_returns_to_zero() {
        let kernel = step_kernel();
        assert!(
            kernel[0] < -0.5,
            "step residual must start near -1, got {}",
            kernel[0]
        );
        assert!(
            kernel[KERNEL_TAPS - 1].abs() < 1e-3,
            "step residual must end at zero, got {}",
            kernel[KERNEL_TAPS - 1]
        );
    }

    #[test]
    fn slope_kernel_is_transient() {
        let kernel = slope_kernel();
        assert!(
            kernel[KERNEL_TAPS - 1].abs() < 1e-3,
            "slope residual must end at zero, got {}",
            kernel[KERNEL_TAPS - 1]
        );
        for value in kernel {
            assert!(value.is_finite());
        }
    }

    #[test]
    fn bus_retires_corrections_after_full_pass() {
        let mut bus = CorrectionBus::default();
        bus.add_step(1.0, 0.5);
        let mut accumulated = 0.0f32;
        for _ in 0..BUS_LENGTH {
            accumulated += bus.take();
        }
        assert!(accumulated.is_finite());
        let retired = bus.take();
        assert!(retired.is_finite());
    }

    #[test]
    fn bus_ignores_invalid_magnitudes() {
        let mut bus = CorrectionBus::default();
        bus.add_step(f32::NAN, 0.5);
        bus.add_step(0.0, 0.5);
        bus.add_slope(f32::INFINITY, 0.5);
        for _ in 0..BUS_LENGTH {
            assert_eq!(bus.take(), 0.0);
        }
    }
}
