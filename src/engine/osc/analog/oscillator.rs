use super::analog_tables::{self, TABLE_LEN};
use super::character::{ModelSpec, OscillatorModel, Tolerances, model_spec};
use super::kernels::CorrectionBus;
use super::{AnalogShape, SQUARE_VOLUME_SCALAR};
use crate::engine::filter::{Biquad, DcBlocker};
use crate::engine::osc::drift::{DRIFT_DECIMATION, DriftGenerator};
use fastrand::Rng;
use std::f32::consts::TAU;

const FREQUENCY_GLIDE_SECONDS: f32 = 0.0015;
const CONTROL_GLIDE_SECONDS: f32 = 0.005;
const FREQUENCY_CEILING_RATIO: f32 = 0.45;
const DUTY_MINIMUM: f32 = 0.02;
const DUTY_MAXIMUM: f32 = 0.98;

#[derive(Debug, Clone, Copy)]
pub struct SyncIn {
    pub time: f32,
    pub depth: f32,
    pub threshold: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SyncOut {
    pub phase: f32,
    pub increment: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Geometry {
    curvature: f32,
    reset_fraction: f32,
    triangle_peak: f32,
    shark_split: f32,
    shark_power: u8,
}

#[inline]
fn curved_ramp(phase: f32, curvature: f32) -> f32 {
    2.0 * (phase + curvature * phase * (1.0 - phase) * (1.0 - 2.0 * phase)) - 1.0
}

#[inline]
fn curved_ramp_slope(phase: f32, curvature: f32) -> f32 {
    2.0 * (1.0 + curvature * (1.0 - 6.0 * phase + 6.0 * phase * phase))
}

#[inline]
fn naive_triangle(phase: f32, peak: f32) -> f32 {
    if phase < peak {
        -1.0 + 2.0 * phase / peak
    } else {
        1.0 - 2.0 * (phase - peak) / (1.0 - peak)
    }
}

#[inline]
fn naive_sharktooth(phase: f32, split: f32, power: u8) -> f32 {
    if phase < split {
        let normalized = phase / split;
        let folded = if power >= 3 {
            normalized * normalized * normalized
        } else {
            normalized * normalized
        };
        -1.0 + 2.0 * folded
    } else {
        1.0 - 2.0 * (phase - split) / (1.0 - split)
    }
}

#[inline]
fn naive_pulse(phase: f32, duty: f32) -> f32 {
    if phase < duty { 1.0 } else { -1.0 }
}

#[inline]
fn crossing_time(target: f32, start: f32, span: f32, increment: f32) -> f32 {
    if increment <= 1e-12 || span <= 0.0 {
        return -1.0;
    }
    let mut distance = target - start;
    if distance <= 0.0 {
        distance += 1.0;
    }
    if distance <= span {
        distance / increment
    } else {
        -1.0
    }
}

#[inline]
fn uses_wavetable_core(shape: AnalogShape) -> bool {
    matches!(shape, AnalogShape::Sine | AnalogShape::Morph)
}

#[derive(Debug, Clone)]
pub struct Oscillator {
    model: OscillatorModel,
    spec: &'static ModelSpec,
    shape: AnalogShape,
    tolerances: Tolerances,
    drift: DriftGenerator,
    rng: Rng,
    sample_rate: f32,
    frequency_ceiling: f32,
    frequency_glide: f32,
    control_glide: f32,

    phase: f32,
    previous_phase: f32,
    increment: f32,
    period_jitter: f32,
    frequency: f32,
    target_frequency: f32,
    duty: f32,
    morph: f32,
    geometry: Geometry,
    table_band: (usize, f32),
    control_divider: u32,

    saw_bus: CorrectionBus,
    pulse_bus: CorrectionBus,
    triangle_bus: CorrectionBus,
    shark_bus: CorrectionBus,
    sine_bus: CorrectionBus,
    output_stage: Biquad,
    dc_blocker: DcBlocker,
}

impl Default for Oscillator {
    fn default() -> Self {
        Self::new(0x1234_5678)
    }
}

impl Oscillator {
    pub fn new(seed: u32) -> Self {
        let mut rng = Rng::with_seed(seed as u64);
        let drift = DriftGenerator::new(seed);
        let tolerance_scale = model_spec(OscillatorModel::Precision).tolerance_scale;
        let tolerances = Tolerances::draw(&mut rng, tolerance_scale);
        let mut oscillator = Self {
            model: OscillatorModel::Precision,
            spec: model_spec(OscillatorModel::Precision),
            shape: AnalogShape::Saw,
            tolerances,
            drift,
            rng,
            sample_rate: 48_000.0,
            frequency_ceiling: 48_000.0 * FREQUENCY_CEILING_RATIO,
            frequency_glide: 1.0 - (-1.0 / (48_000.0 * FREQUENCY_GLIDE_SECONDS)).exp(),
            control_glide: 1.0 - (-1.0 / (48_000.0 * CONTROL_GLIDE_SECONDS)).exp(),
            phase: 0.0,
            previous_phase: 0.0,
            increment: 0.0,
            period_jitter: 0.0,
            frequency: 440.0,
            target_frequency: 440.0,
            duty: 0.5,
            morph: 0.0,
            geometry: Geometry::default(),
            table_band: (0, 0.0),
            control_divider: 0,
            saw_bus: CorrectionBus::default(),
            pulse_bus: CorrectionBus::default(),
            triangle_bus: CorrectionBus::default(),
            shark_bus: CorrectionBus::default(),
            sine_bus: CorrectionBus::default(),
            output_stage: Biquad::default(),
            dc_blocker: DcBlocker::default(),
        };
        oscillator.apply_output_stage();
        oscillator
    }

    pub fn reseed(&mut self, seed: u32) {
        let mut rng = Rng::with_seed(seed as u64);
        self.drift = DriftGenerator::new(seed);
        self.tolerances = Tolerances::draw(&mut rng, self.spec.tolerance_scale);
        self.rng = rng;
    }

    pub fn phase(&self) -> f32 {
        self.phase
    }

    pub fn set_phase(&mut self, phase: f32) {
        self.phase = phase.rem_euclid(1.0);
        self.previous_phase = self.phase;
    }

    pub fn sync_out(&self) -> SyncOut {
        SyncOut {
            phase: self.previous_phase,
            increment: self.increment,
        }
    }

    pub fn soft_sync_threshold(&self) -> f32 {
        self.spec.soft_sync_threshold
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.previous_phase = 0.0;
        self.output_stage.reset();
        self.dc_blocker.reset();
        self.clear_buses();
    }

    fn clear_buses(&mut self) {
        self.saw_bus.clear();
        self.pulse_bus.clear();
        self.triangle_bus.clear();
        self.shark_bus.clear();
        self.sine_bus.clear();
    }

    fn apply_output_stage(&mut self) {
        let cutoff = self.spec.stage_cutoff.min(self.sample_rate * 0.42);
        let angular = TAU * cutoff / self.sample_rate;
        let (sine, cosine) = (angular.sin(), angular.cos());
        let alpha = sine / (2.0 * self.spec.stage_resonance.max(0.2));
        let denominator = 1.0 + alpha;
        let b0 = (1.0 - cosine) * 0.5 / denominator;
        let b1 = (1.0 - cosine) / denominator;
        let b2 = b0;
        let a1 = -2.0 * cosine / denominator;
        let a2 = (1.0 - alpha) / denominator;
        self.output_stage.set_coefficients(a1, a2, b0, b1, b2);
    }

    fn set_shape(&mut self, shape: AnalogShape) {
        self.shape = shape;
        self.model = match shape {
            AnalogShape::Sine | AnalogShape::Morph => OscillatorModel::Precision,
            _ => self.model,
        };
        self.clear_buses();
        self.apply_output_stage();
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.frequency_ceiling = sample_rate * FREQUENCY_CEILING_RATIO;
        self.frequency_glide = 1.0 - (-1.0 / (sample_rate * FREQUENCY_GLIDE_SECONDS)).exp();
        self.control_glide = 1.0 - (-1.0 / (sample_rate * CONTROL_GLIDE_SECONDS)).exp();
        self.dc_blocker.set_sample_rate(sample_rate);
        self.apply_output_stage();
    }

    fn update_control(&mut self, base_frequency: f32, character_amount: f32) {
        let (wander, thermal) = self.drift.next(self.sample_rate);
        let detune_cents = self.tolerances.drift_depth * wander * self.spec.drift_cents
            + self.tolerances.thermal_depth * thermal * self.spec.thermal_cents;
        self.target_frequency = base_frequency * (detune_cents * character_amount / 1200.0).exp2();
        if !self.target_frequency.is_finite() {
            self.target_frequency = 0.0;
        }

        self.geometry.curvature =
            (self.spec.ramp_curvature + self.tolerances.curvature) * character_amount;
        self.geometry.triangle_peak =
            (0.5 + self.tolerances.triangle_peak * character_amount).clamp(0.2, 0.8);
        self.geometry.shark_split = self.spec.shark_split;
        self.geometry.shark_power = self.spec.shark_power;
        self.geometry.reset_fraction = if self.spec.reset_fraction > 0.0 && character_amount > 0.0 {
            (self.spec.reset_fraction * character_amount * (1.0 + self.tolerances.reset))
                .max(1.5 * self.increment)
        } else {
            0.0
        };

        self.table_band = analog_tables::band_pos(self.frequency);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process(
        &mut self,
        shape: AnalogShape,
        base_frequency: f32,
        pulse_width: f32,
        character_amount: f32,
        morph: f32,
        sample_rate: f32,
        sync: Option<SyncIn>,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }
        if sample_rate != self.sample_rate {
            self.set_sample_rate(sample_rate);
        }
        if self.shape != shape {
            self.set_shape(shape);
        }

        let amount = character_amount.clamp(0.0, 1.0);
        self.control_divider += 1;
        if self.control_divider >= DRIFT_DECIMATION {
            self.control_divider = 0;
            self.update_control(base_frequency.max(0.0), amount);
        }

        self.frequency += (self.target_frequency - self.frequency) * self.frequency_glide;
        if !self.frequency.is_finite() {
            self.frequency = 0.0;
        }
        let frequency = self.frequency.clamp(0.0, self.frequency_ceiling);
        self.increment = frequency / sample_rate * (1.0 + self.period_jitter);

        let duty_target = (pulse_width.clamp(DUTY_MINIMUM, DUTY_MAXIMUM)
            + self.tolerances.duty * amount
            + self.spec.duty_pitch_coupling * amount * (frequency * 0.001).min(2.0))
        .clamp(DUTY_MINIMUM, DUTY_MAXIMUM);
        self.duty += (duty_target - self.duty) * self.control_glide;
        self.morph += (morph.clamp(0.0, 1.0) - self.morph) * self.control_glide;

        let phase_start = self.phase;
        self.previous_phase = phase_start;

        let mut sync_event = None;
        if let Some(event) = sync
            && event.depth > 0.0
            && event.time > 0.0
            && event.time <= 1.0
            && self.increment > 0.0
        {
            let reset_phase = (phase_start + event.time * self.increment).rem_euclid(1.0);
            if reset_phase >= event.threshold {
                sync_event = Some((event, reset_phase));
            }
        }

        match sync_event {
            Some((event, reset_phase)) => {
                let first_span = event.time * self.increment;
                self.emit_corrections(shape, phase_start, first_span, 0.0);
                let reset_target = reset_phase * (1.0 - event.depth);
                self.emit_sync_corrections(shape, reset_phase, reset_target, event.time);
                let second_span = (1.0 - event.time) * self.increment;
                self.emit_corrections(shape, reset_target, second_span, event.time);
                self.phase = (reset_target + second_span).rem_euclid(1.0);
            }
            None => {
                self.emit_corrections(shape, phase_start, self.increment, 0.0);
                self.phase = (phase_start + self.increment).rem_euclid(1.0);
            }
        }

        if self.phase < phase_start {
            self.period_jitter = self.spec.period_jitter
                * amount
                * self.tolerances.drift_depth
                * (self.rng.f32() * 2.0 - 1.0);
        }

        let mixed = self.evaluate(shape, self.phase);
        let shaped = self.dc_blocker.process(self.output_stage.process(mixed));
        let output = shaped * (1.0 + self.tolerances.gain * amount);
        if output.is_finite() {
            output
        } else {
            self.reset();
            0.0
        }
    }

    fn emit_corrections(&mut self, shape: AnalogShape, start: f32, span: f32, offset: f32) {
        if uses_wavetable_core(shape) {
            return;
        }
        let increment = self.increment;
        let geometry = self.geometry;

        match shape {
            AnalogShape::Saw | AnalogShape::ReverseSaw => {
                if geometry.reset_fraction > 0.0 {
                    let reset_start = 1.0 - geometry.reset_fraction;
                    let held = curved_ramp(reset_start, geometry.curvature);
                    let fall_slope = (-1.0 - held) / geometry.reset_fraction;
                    let crossing = crossing_time(reset_start, start, span, increment);
                    if crossing >= 0.0 {
                        let magnitude = (fall_slope
                            - curved_ramp_slope(reset_start, geometry.curvature))
                            * increment;
                        self.saw_bus.add_slope(magnitude, 1.0 - (offset + crossing));
                    }
                    let crossing = crossing_time(0.0, start, span, increment);
                    if crossing >= 0.0 {
                        let magnitude =
                            (curved_ramp_slope(0.0, geometry.curvature) - fall_slope) * increment;
                        self.saw_bus.add_slope(magnitude, 1.0 - (offset + crossing));
                    }
                } else {
                    let crossing = crossing_time(0.0, start, span, increment);
                    if crossing >= 0.0 {
                        self.saw_bus.add_step(-2.0, 1.0 - (offset + crossing));
                    }
                }
            }
            AnalogShape::Sharktooth => {
                let split = geometry.shark_split;
                let rising = 2.0 * geometry.shark_power as f32 / split;
                let falling = -2.0 / (1.0 - split);
                let crossing = crossing_time(split, start, span, increment);
                if crossing >= 0.0 {
                    self.shark_bus
                        .add_slope((falling - rising) * increment, 1.0 - (offset + crossing));
                }
                let crossing = crossing_time(0.0, start, span, increment);
                if crossing >= 0.0 {
                    self.shark_bus
                        .add_slope(-falling * increment, 1.0 - (offset + crossing));
                }
            }
            AnalogShape::Sine
            | AnalogShape::Morph
            | AnalogShape::Square
            | AnalogShape::Triangle => {}
        }
    }

    fn emit_sync_corrections(
        &mut self,
        shape: AnalogShape,
        from_phase: f32,
        to_phase: f32,
        time: f32,
    ) {
        let offset = 1.0 - time;
        let increment = self.increment;
        let wavetable = uses_wavetable_core(shape);
        let skipped = if wavetable {
            (1.0 - to_phase / (2.0 * increment).max(1e-9)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        match shape {
            AnalogShape::Saw | AnalogShape::ReverseSaw => {
                let before = self.saw_value(from_phase);
                let after = self.saw_value(to_phase);
                self.saw_bus.add_step(after - before, offset);
                if !wavetable {
                    let slope_change =
                        (self.saw_slope(to_phase) - self.saw_slope(from_phase)) * increment;
                    self.saw_bus.add_slope(slope_change, offset);
                }
            }
            AnalogShape::Square => {
                let before = self.table_square(from_phase);
                let after = self.table_square(to_phase);
                self.pulse_bus.add_step(after - before, offset);
            }
            AnalogShape::Triangle => {
                let before = self.table_triangle(from_phase);
                let after = self.table_triangle(to_phase);
                self.triangle_bus.add_step(after - before, offset);
            }
            AnalogShape::Sharktooth => {
                let before = naive_sharktooth(
                    from_phase,
                    self.geometry.shark_split,
                    self.geometry.shark_power,
                );
                let after = naive_sharktooth(
                    to_phase,
                    self.geometry.shark_split,
                    self.geometry.shark_power,
                );
                self.shark_bus.add_step(after - before, offset);
                let slope_change =
                    (self.shark_slope(to_phase) - self.shark_slope(from_phase)) * increment;
                self.shark_bus.add_slope(slope_change, offset);
            }
            AnalogShape::Sine => {
                let before = (from_phase * TAU).sin();
                let after = (to_phase * TAU).sin();
                self.sine_bus.add_step(after - before, offset);
            }
            AnalogShape::Morph => {
                let before_saw = 2.0 * from_phase - 1.0;
                let after_saw = 2.0 * to_phase - 1.0;
                self.saw_bus
                    .add_step(after_saw - before_saw + 2.0 * skipped, offset);
                let before_pulse = naive_pulse(from_phase, self.duty);
                let after_pulse = naive_pulse(to_phase, self.duty);
                self.pulse_bus
                    .add_step(after_pulse - before_pulse - 2.0 * skipped, offset);
                let before_triangle = naive_triangle(from_phase, self.geometry.triangle_peak);
                let after_triangle = naive_triangle(to_phase, self.geometry.triangle_peak);
                self.triangle_bus
                    .add_step(after_triangle - before_triangle, offset);
            }
        }
    }

    fn table_square(&self, phase: f32) -> f32 {
        let (band, band_mix) = self.table_band;
        let position = phase * TABLE_LEN as f32;
        let offset_position = position + self.duty * TABLE_LEN as f32;
        let saw =
            analog_tables::read_band_pair(&analog_tables::get().saw, band, band_mix, position);
        let offset_saw = analog_tables::read_band_pair(
            &analog_tables::get().saw,
            band,
            band_mix,
            offset_position,
        );
        let scale = 2.0 * (1.0 - self.duty).max(self.duty);
        (saw - offset_saw) * scale * SQUARE_VOLUME_SCALAR
    }

    fn table_triangle(&self, phase: f32) -> f32 {
        let (band, band_mix) = self.table_band;
        let position = phase * TABLE_LEN as f32;
        analog_tables::read_band_pair(&analog_tables::get().triangle, band, band_mix, position)
    }

    fn saw_value(&self, phase: f32) -> f32 {
        let reset = self.geometry.reset_fraction;
        if reset > 0.0 && phase >= 1.0 - reset {
            let reset_start = 1.0 - reset;
            let held = curved_ramp(reset_start, self.geometry.curvature);
            held + (-1.0 - held) * ((phase - reset_start) / reset)
        } else {
            curved_ramp(phase, self.geometry.curvature)
        }
    }

    fn saw_slope(&self, phase: f32) -> f32 {
        let reset = self.geometry.reset_fraction;
        if reset > 0.0 && phase >= 1.0 - reset {
            let reset_start = 1.0 - reset;
            let held = curved_ramp(reset_start, self.geometry.curvature);
            (-1.0 - held) / reset
        } else {
            curved_ramp_slope(phase, self.geometry.curvature)
        }
    }

    fn shark_slope(&self, phase: f32) -> f32 {
        let split = self.geometry.shark_split;
        let power = self.geometry.shark_power;
        if phase < split {
            let normalized = phase / split;
            let slope_multiplier = if power >= 3 {
                normalized * normalized
            } else {
                normalized
            };
            2.0 * power as f32 * slope_multiplier / split
        } else {
            -2.0 / (1.0 - split)
        }
    }

    fn evaluate(&mut self, shape: AnalogShape, phase: f32) -> f32 {
        match shape {
            AnalogShape::Saw | AnalogShape::ReverseSaw => {
                let saw = self.saw_value(phase) + self.saw_bus.take();
                if shape == AnalogShape::ReverseSaw {
                    -saw
                } else {
                    saw
                }
            }
            AnalogShape::Square => self.table_square(phase) + self.pulse_bus.take(),
            AnalogShape::Triangle => self.table_triangle(phase) + self.triangle_bus.take(),
            AnalogShape::Sharktooth => {
                naive_sharktooth(phase, self.geometry.shark_split, self.geometry.shark_power)
                    + self.shark_bus.take()
            }
            AnalogShape::Sine => {
                let position = phase * TABLE_LEN as f32;
                analog_tables::read_sine(position) + self.sine_bus.take()
            }
            AnalogShape::Morph => self.evaluate_morph(phase),
        }
    }

    fn evaluate_morph(&mut self, phase: f32) -> f32 {
        let (band, band_mix) = self.table_band;
        let position = phase * TABLE_LEN as f32;
        let saw =
            analog_tables::read_band_pair(&analog_tables::get().saw, band, band_mix, position)
                + self.saw_bus.take();
        let offset_position = (phase + 1.0 - self.duty).rem_euclid(1.0) * TABLE_LEN as f32;
        let offset_saw = analog_tables::read_band_pair(
            &analog_tables::get().saw,
            band,
            band_mix,
            offset_position,
        );
        let pulse =
            (offset_saw - saw) * (0.5 / self.duty.max(1.0 - self.duty)) + self.pulse_bus.take();
        let triangle =
            analog_tables::read_band_pair(&analog_tables::get().triangle, band, band_mix, position)
                + self.triangle_bus.take();

        self.morph_mix(saw, triangle, pulse)
    }

    fn morph_mix(&self, saw: f32, triangle: f32, pulse: f32) -> f32 {
        let position = self.morph;
        let (from, to, blend) = if position < 0.5 {
            (saw, triangle, position * 2.0)
        } else {
            (triangle, pulse, (position - 0.5) * 2.0)
        };
        let from_weight = (1.0 - blend) * (1.0 - blend);
        let to_weight = blend * blend;
        let total = (from_weight + to_weight).max(1e-6);
        from * (from_weight / total) + to * (to_weight / total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    fn render(shape: AnalogShape, frequency: f32, samples: usize, drift: f32) -> Vec<f32> {
        let mut oscillator = Oscillator::new(11);
        (0..samples)
            .map(|_| oscillator.process(shape, frequency, 0.5, drift, 0.5, SAMPLE_RATE, None))
            .collect()
    }

    #[test]
    fn pitch_tracks_requested_frequency() {
        for &frequency in &[55.0f32, 440.0, 3_000.0] {
            for shape in [
                AnalogShape::Saw,
                AnalogShape::Square,
                AnalogShape::Triangle,
                AnalogShape::Morph,
                AnalogShape::Sine,
            ] {
                let rendered = render(shape, frequency, SAMPLE_RATE as usize, 0.0);
                let mut crossings = 0usize;
                let mut previous = rendered[0];
                for &sample in &rendered[1..] {
                    if previous <= 0.0 && sample > 0.0 {
                        crossings += 1;
                    }
                    previous = sample;
                }
                let expected = frequency;
                assert!(
                    (crossings as f32 - expected).abs() <= expected * 0.05,
                    "{shape:?} at {frequency} Hz measured {crossings} cycles"
                );
            }
        }
    }

    #[test]
    fn every_shape_stays_finite_at_extremes() {
        for shape in AnalogShape::ALL {
            for &frequency in &[10.0f32, 800.0, 15_000.0] {
                for &drift in &[0.0f32, 1.0] {
                    let rendered = render(shape, frequency, 2_048, drift);
                    for (index, &sample) in rendered.iter().enumerate() {
                        assert!(
                            sample.is_finite(),
                            "{shape:?} at {frequency} Hz drift {drift} produced {sample} at sample {index}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn hard_sync_shortens_period() {
        const MASTER_HZ: f32 = 440.0;
        const SLAVE_HZ: f32 = 200.0;
        const SAMPLES: usize = 48_000;

        let mut master = Oscillator::new(21);
        let mut slave = Oscillator::new(22);
        let mut crossings = 0usize;
        let mut previous_slave = 0.0f32;
        for _ in 0..SAMPLES {
            master.process(
                AnalogShape::Saw,
                MASTER_HZ,
                0.5,
                0.0,
                0.5,
                SAMPLE_RATE,
                None,
            );
            let master_out = master.sync_out();
            let remaining = 1.0 - master_out.phase;
            let sync = if remaining <= master_out.increment {
                Some(SyncIn {
                    time: (remaining / master_out.increment).clamp(1e-6, 1.0),
                    depth: 1.0,
                    threshold: 0.0,
                })
            } else {
                None
            };
            let slave_sample =
                slave.process(AnalogShape::Saw, SLAVE_HZ, 0.5, 0.0, 0.5, SAMPLE_RATE, sync);
            if previous_slave <= 0.0 && slave_sample > 0.0 {
                crossings += 1;
            }
            previous_slave = slave_sample;
        }
        let measured_hz = crossings as f32;
        assert!(
            (measured_hz - MASTER_HZ).abs() <= MASTER_HZ * 0.05,
            "synced slave should follow the master's {MASTER_HZ} Hz, measured {measured_hz} Hz"
        );
    }

    #[test]
    fn morph_changes_timbre() {
        let saw_side = render_morph(0.0);
        let pulse_side = render_morph(1.0);
        let difference: f32 = saw_side
            .iter()
            .zip(&pulse_side)
            .map(|(a, b)| (a - b).abs())
            .sum();
        let mean_difference = difference / saw_side.len() as f32;
        assert!(
            mean_difference > 0.05,
            "morph extremes should sound different, mean difference {mean_difference}"
        );
    }

    #[test]
    fn square_and_triangle_have_no_dc_offset() {
        for shape in [AnalogShape::Square, AnalogShape::Triangle] {
            let rendered = render(shape, 220.0, SAMPLE_RATE as usize, 0.0);
            let steady = &rendered[rendered.len() / 2..];
            let mean: f32 = steady.iter().sum::<f32>() / steady.len() as f32;
            assert!(
                mean.abs() < 0.01,
                "{shape:?} should center around zero, mean {mean}"
            );
        }
    }

    fn render_morph(morph: f32) -> Vec<f32> {
        let mut oscillator = Oscillator::new(31);
        (0..SAMPLE_RATE as usize)
            .map(|_| {
                oscillator.process(
                    AnalogShape::Morph,
                    220.0,
                    0.5,
                    0.0,
                    morph,
                    SAMPLE_RATE,
                    None,
                )
            })
            .skip(SAMPLE_RATE as usize / 2)
            .collect()
    }
}
