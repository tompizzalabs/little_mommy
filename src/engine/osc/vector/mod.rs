//! Vector oscillator: 2D morph between four fixed corner waves.
//!
//! The corners are bandlimited waves read from the analog module's
//! mip-mapped tables — saw, triangle, sine, and reverse saw — so every
//! point in the morph space is a blend of bandlimited waves and stays
//! alias-free without oversampling. X sweeps the bottom row
//! (saw -> reverse saw) against the top row (triangle -> sine); Y
//! crossfades between the rows. The pad position is smoothed so modulated
//! XY moves glide instead of stepping. Mono output, duplicated to both
//! channels.

use super::analog::analog_tables::{self, TABLE_LEN};
use super::drift::{DRIFT_RANGE_SEMITONES, DriftGenerator};

/// XY position smoothing per sample. Slow enough to turn stepped
/// modulation into a glide, fast enough to track the pad.
const VECTOR_SMOOTH: f32 = 0.001;

#[derive(Debug, Clone)]
pub struct VectorState {
    pub phase: f32,
    pub drift: DriftGenerator,
    pub x_smooth: f32,
    pub y_smooth: f32,
}

impl Default for VectorState {
    fn default() -> Self {
        Self {
            phase: 0.0,
            drift: DriftGenerator::default(),
            x_smooth: 0.5,
            y_smooth: 0.5,
        }
    }
}

#[inline(always)]
fn read(table: &[f32], phase: f32) -> f32 {
    let pos = phase * TABLE_LEN as f32;
    let i0 = pos as usize & (TABLE_LEN - 1);
    let i1 = (i0 + 1) & (TABLE_LEN - 1);
    let frac = pos - pos.floor();
    table[i0] + (table[i1] - table[i0]) * frac
}

impl VectorState {
    /// Jump the smoothed pad position (used on voice creation).
    pub fn snap_xy(&mut self, x: f32, y: f32) {
        self.x_smooth = x;
        self.y_smooth = y;
    }

    /// Sample one output step: X/Y morph between the corner waves, phase
    /// advanced by `freq / sample_rate` with drift applied.
    pub fn next(&mut self, freq: f32, sample_rate: f32, x: f32, y: f32, drift_amount: f32) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        self.x_smooth += (x - self.x_smooth) * VECTOR_SMOOTH;
        self.y_smooth += (y - self.y_smooth) * VECTOR_SMOOTH;
        let xm = self.x_smooth.clamp(0.0, 1.0);
        let ym = self.y_smooth.clamp(0.0, 1.0);

        let drift_semitones = self.drift.next(sample_rate) * drift_amount * DRIFT_RANGE_SEMITONES;
        let drifted = freq * 2.0f32.powf(drift_semitones / 12.0);
        self.phase += drifted / sample_rate;
        self.phase -= self.phase.floor();

        let tables = analog_tables::get();
        let idx = analog_tables::table_index(drifted.max(1.0));
        let saw = &tables.saw[idx];

        // Corners: bottom-left saw, top-left triangle, top-right sine,
        // bottom-right reverse saw (the saw's mirror).
        let e0 = read(saw, self.phase);
        let e1 = read(&tables.triangle[idx], self.phase);
        let e2 = read(&tables.sine, self.phase);
        let e3 = -e0;

        let lower = (1.0 - xm) * e0 + xm * e3;
        let upper = (1.0 - xm) * e1 + xm * e2;
        (1.0 - ym) * lower + ym * upper
    }

    /// Re-seed phase and drift (deterministic, per voice/node).
    pub fn seed(&mut self, seed: u32) {
        self.phase = (seed.wrapping_mul(0x9E37_79B9) % 983) as f32 / 983.0;
        self.drift = DriftGenerator::new(seed);
        self.snap_xy(0.5, 0.5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    #[test]
    fn bottom_left_corner_is_pure_saw() {
        let mut state = VectorState::default();
        state.phase = 0.25;
        state.snap_xy(0.0, 0.0);

        let tables = analog_tables::get();
        let idx = analog_tables::table_index(440.0);
        let saw = &tables.saw[idx];

        // The read happens after the phase advance.
        let phase_after = 0.25 + 440.0 / SR;
        let out = state.next(440.0, SR, 0.0, 0.0, 0.0);
        let expect = read(saw, phase_after);
        assert!((out - expect).abs() < 1e-5, "corner (0,0) should be pure saw");
    }

    #[test]
    fn bottom_right_corner_is_inverted_saw() {
        let mut a = VectorState::default();
        let mut b = VectorState::default();
        a.phase = 0.375;
        b.phase = 0.375;
        a.snap_xy(0.0, 0.0);
        b.snap_xy(1.0, 0.0);

        let low = a.next(440.0, SR, 0.0, 0.0, 0.0);
        let high = b.next(440.0, SR, 1.0, 0.0, 0.0);
        assert!(
            (low + high).abs() < 1e-4,
            "x=1 on the bottom row should be the inverted saw"
        );
    }

    #[test]
    fn top_row_blends_triangle_toward_sine() {
        let mut state = VectorState::default();
        state.phase = 0.25;
        state.snap_xy(1.0, 1.0);

        let tables = analog_tables::get();
        let sine = &tables.sine;

        let phase_after = 0.25 + 440.0 / SR;
        let out = state.next(440.0, SR, 1.0, 1.0, 0.0);
        let expect = read(sine, phase_after);
        assert!((out - expect).abs() < 1e-5, "corner (1,1) should be pure sine");
    }

    #[test]
    fn xy_blend_stays_in_range() {
        let mut state = VectorState::default();
        let mut peak = 0.0f32;
        for step in 0..SR as usize {
            let x = (step % 1000) as f32 / 1000.0;
            let y = ((step / 700) % 1000) as f32 / 1000.0;
            let v = state.next(220.0, SR, x, y, 0.0);
            assert!(v.is_finite(), "output must stay finite");
            peak = peak.max(v.abs());
        }
        assert!(peak > 0.0, "oscillator should produce sound");
        assert!(peak < 4.0, "crossfade must not blow up, got {peak}");
    }

    #[test]
    fn drift_zero_is_pitch_stable() {
        let mut state = VectorState::default();
        // Zero drift must leave the phase increment exactly at freq/sr.
        let mut crossings = 0usize;
        let mut last = state.next(100.0, SR, 0.5, 0.5, 0.0);
        for _ in 1..(SR as usize) {
            let v = state.next(100.0, SR, 0.5, 0.5, 0.0);
            if last < 0.0 && v >= 0.0 {
                crossings += 1;
            }
            last = v;
        }
        // 1 second at 100 Hz = ~100 cycles.
        assert!(
            (crossings as i32 - 100).abs() <= 1,
            "expected ~100 cycles, got {crossings}"
        );
    }

    #[test]
    fn seed_is_deterministic() {
        let mut a = VectorState::default();
        let mut b = VectorState::default();
        a.seed(1234);
        b.seed(1234);
        let mut c = VectorState::default();
        c.seed(4321);

        for _ in 0..256 {
            let va = a.next(440.0, SR, 0.2, 0.8, 1.0);
            let vb = b.next(440.0, SR, 0.2, 0.8, 1.0);
            let vc = c.next(440.0, SR, 0.2, 0.8, 1.0);
            assert_eq!(va, vb, "same seed must produce identical output");
            assert_ne!(va, vc, "different seeds should diverge");
        }
    }
}
