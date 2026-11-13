//! Shared static waveform previews.
//!
//! Every oscillator node (and the LFO) renders its one-cycle waveform through
//! [`wave_preview`], so the painting code lives in exactly one place. Each
//! node supplies a `FnMut(f32) -> f32` callback that samples one cycle at a
//! normalized phase `t` in `0..1` (or, for noise, returns successive random
//! samples). Vertical edges are drawn as crisp jumps wherever consecutive
//! samples step the plot by more than [`EDGE_FRAC`] of the amplitude.

use crate::engine::arpeggiator::pitch_shift_multiplier;
use crate::engine::dsp;
use crate::engine::lfo::LfoShape;
use crate::engine::osc::analog::analog_tables::{TABLE_LEN, table_index};
use crate::engine::osc::analog::SQUARE_VOLUME_SCALAR;
use crate::engine::osc::wavetable::TableSource;
use crate::engine::osc::{AnalogShape, NoiseColor, NoiseState};

/// Height of the waveform strip inside a node body.
pub const PREVIEW_H: f32 = 44.0;
/// Fraction of the plot amplitude that counts as a "crisp edge" (discontinuity).
const EDGE_FRAC: f32 = 0.35;
/// Number of points sampled across the strip. Small enough to be free on the
/// UI thread, large enough to look smooth.
const N: usize = 128;

/// Vertical anchoring for a preview strip.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Baseline {
    /// Waveforms: zero maps to the vertical middle, values in [-1, 1].
    Center,
    /// Envelopes: value zero sits on the bottom edge, values in [0, 1] rise up.
    Bottom,
}

/// Shared waveform strip renderer: allocates a fixed-size rect, paints the
/// background + baseline, then samples `sample` in order and draws the
/// polyline, inserting crisp vertical edges at discontinuities. Center-anchored,
/// mirroring the classic LFO preview.
pub fn wave_preview(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    sample: impl FnMut(f32) -> f32,
) {
    wave_preview_baseline(ui, width, color, Baseline::Center, sample);
}

/// Like [`wave_preview`], but with an explicit [`Baseline`] so envelopes can be
/// drawn bottom-anchored while oscillators keep their centred zero line.
pub fn wave_preview_baseline(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    baseline: Baseline,
    mut sample: impl FnMut(f32) -> f32,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, PREVIEW_H), egui::Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(24, 24, 30));
    painter.rect_stroke(
        rect.shrink(1.0),
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 48, 58)),
        egui::StrokeKind::Inside,
    );

    let (baseline_y, amp) = match baseline {
        Baseline::Center => (rect.center().y, rect.height() * 0.5 - 3.0),
        Baseline::Bottom => (rect.bottom() - 3.0, rect.height() - 6.0),
    };
    painter.line_segment(
        [egui::pos2(rect.left(), baseline_y), egui::pos2(rect.right(), baseline_y)],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 48, 58)),
    );

    let mut prev_y = baseline_y;
    let mut pts: Vec<egui::Pos2> = Vec::with_capacity(N * 2);
    for i in 0..N {
        let t = i as f32 / (N - 1) as f32;
        let x = rect.left() + t * rect.width();
        let v = match baseline {
            Baseline::Center => sample(t).clamp(-1.0, 1.0),
            Baseline::Bottom => sample(t).clamp(0.0, 1.0),
        };
        let y = match baseline {
            Baseline::Center => baseline_y - v * amp,
            Baseline::Bottom => baseline_y - v * amp,
        };
        if i > 0 && (y - prev_y).abs() > amp * EDGE_FRAC {
            pts.push(egui::pos2(x, prev_y)); // crisp vertical edge across a discontinuity
        }
        pts.push(egui::pos2(x, y));
        prev_y = y;
    }
    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5, color)));
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// -------------------------------------------------------------------------------------------------
// LFO
// -------------------------------------------------------------------------------------------------

/// One cycle of the LFO, sampled straight from the engine's own shape
/// evaluation so the picture matches the voice exactly.
pub fn preview_lfo(ui: &mut egui::Ui, width: f32, color: egui::Color32, shape: LfoShape) {
    wave_preview(ui, width, color, |t| shape.sample(t));
}

// -------------------------------------------------------------------------------------------------
// Analog
// -------------------------------------------------------------------------------------------------

/// Band-limited analog waveform at an explicit phase. Uses a fixed mid-range
/// table index and the raw pulse width (no duty smoothing / drift) so the
/// preview is crisp and shows exactly what the user has dialled in.
fn analog_sample(shape: AnalogShape, pulse_width: f32, phase: f32) -> f32 {
    let tables = &*crate::engine::osc::analog::analog_tables::get();
    // G4-ish index: bright enough to look like the real oscillator, but still
    // band-limited so the preview stays clean.
    let idx = table_index(440.0);
    let saw_table = &tables.saw[idx];

    let pos = phase.rem_euclid(1.0) * TABLE_LEN as f32;
    let i0 = pos as usize & (TABLE_LEN - 1);
    let i1 = (i0 + 1) & (TABLE_LEN - 1);
    let frac = pos - pos.floor();
    let saw = lerp(saw_table[i0], saw_table[i1], frac);

    match shape {
        AnalogShape::Saw => saw,
        AnalogShape::Triangle => {
            let tri = &tables.triangle[idx];
            lerp(tri[i0], tri[i1], frac)
        }
        AnalogShape::Sine => {
            let s = &tables.sine;
            lerp(s[i0], s[i1], frac)
        }
        AnalogShape::Square => {
            // Saw-differencing square, mirroring AnalogVoiceState::next.
            let d = pulse_width.clamp(0.02, 0.98);
            let pos_off = pos + d * TABLE_LEN as f32;
            let j0 = pos_off as usize & (TABLE_LEN - 1);
            let j1 = (j0 + 1) & (TABLE_LEN - 1);
            let frac_off = pos_off - pos_off.floor();
            let saw_off = lerp(saw_table[j0], saw_table[j1], frac_off);
            let scale = 2.0 * (1.0 - d).max(d);
            (saw - saw_off) * scale * SQUARE_VOLUME_SCALAR
        }
    }
}

pub fn preview_analog(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    shape: AnalogShape,
    pulse_width: f32,
) {
    wave_preview(ui, width, color, |t| analog_sample(shape, pulse_width, t));
}

// -------------------------------------------------------------------------------------------------
// Wavetable-based: Wave, Chiptune, Multi, Vector, FM, PM
// -------------------------------------------------------------------------------------------------

/// Single wavetable, read at an explicit phase/position with the Wave node's
/// phase/amplitude shaping applied (identical maths to `NodeState::wave`).
fn wave_sample(
    source: &TableSource,
    position: f32,
    formant: f32,
    skew_h: f32,
    skew_v: f32,
    saturation: f32,
    phase: f32,
) -> f32 {
    let shaped = dsp::shape_phase(phase, formant, skew_h);
    let raw = source.sample(shaped, position);
    dsp::shape_amplitude(raw, skew_v, saturation)
}

pub fn preview_wave(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    source: &TableSource,
    position: f32,
    formant: f32,
    skew_h: f32,
    skew_v: f32,
    saturation: f32,
) {
    wave_preview(ui, width, color, |t| {
        wave_sample(source, position, formant, skew_h, skew_v, saturation, t)
    });
}

/// Chiptune wavetable/1-bit noise preview. In noise mode we draw the stepped
/// pseudo-random pattern the 1-bit noise produces (values are multiples of
/// 0.125, held between steps), otherwise the plain table read.
pub fn preview_chiptune(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    source: &TableSource,
    position: f32,
    noise_mode: bool,
) {
    if noise_mode {
        // Deterministic, phase-hashed sample-and-hold so it never flickers.
        let steps: f32 = 12.0;
        wave_preview(ui, width, color, |t| {
            let step = (t * steps) as u32;
            let raw = hash_noise(step) % 16;
            (raw as f32 - 8.0) * 0.125
        });
    } else {
        wave_preview(ui, width, color, |t| source.sample(t, position));
    }
}

/// Multi oscillator: four voices of the same table spread across the morph
/// axis, summed (identical to `MultiState::next`, minus per-voice pitch which
/// has no bearing on a single-cycle picture).
pub fn preview_multi(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    source: &TableSource,
    position: f32,
    spread: f32,
) {
    let offsets: [f32; 4] = [-1.5, -0.5, 0.5, 1.5];
    wave_preview(ui, width, color, |t| {
        let mut sum = 0.0;
        for &off in &offsets {
            let pos = (position + off * spread * 0.5).clamp(0.0, 1.0);
            sum += source.sample(t, pos);
        }
        sum * 0.25
    });
}

/// Vector oscillator: four tables crossfaded by the X/Y pad (identical to
/// `VectorState::next`, without the smoothing time constant).
pub fn preview_vector(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    sources: &[TableSource; 4],
    positions: [f32; 4],
    x: f32,
    y: f32,
) {
    let x = x.clamp(0.0, 1.0);
    let y = y.clamp(0.0, 1.0);
    wave_preview(ui, width, color, |t| {
        let e0 = sources[0].sample(t, positions[0]);
        let e1 = sources[1].sample(t, positions[1]);
        let e2 = sources[2].sample(t, positions[2]);
        let e3 = sources[3].sample(t, positions[3]);
        let lower = (1.0 - x) * e0 + x * e3;
        let upper = (1.0 - x) * e1 + x * e2;
        (1.0 - y) * lower + y * upper
    });
}

/// FM: the modulator sweeps the carrier's frequency, integrated over one output
/// cycle so the plot shows the true steady-state waveform (carrier gains extra
/// cycles where the FM index is high). Mirrors `FmState::next`'s pitch math.
pub fn preview_fm(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    carrier: &TableSource,
    modulator: &TableSource,
    c_pos: f32,
    m_pos: f32,
    amount: f32,
    c_ratio: f32,
    m_ratio: f32,
    exponential: bool,
) {
    let mf = m_ratio / c_ratio.max(0.0001); // modulator cycles per output cycle
    let amount = amount.max(0.0);
    let exp = exponential;
    let mut carrier_phase = 0.0f32;
    let mut last_t = 0.0f32;
    wave_preview(ui, width, color, move |t| {
        let dt = (t - last_t).max(0.0);
        last_t = t;
        let mv = modulator.sample((t * mf).rem_euclid(1.0), m_pos);
        let mult = if exp {
            pitch_shift_multiplier(mv * amount * 24.0)
        } else {
            1.0 + mv * 15.0 * amount
        };
        carrier_phase += mult * dt;
        carrier.sample(carrier_phase.rem_euclid(1.0), c_pos)
    });
}

/// PM: the modulator shifts the carrier's read phase (identical math to
/// `PmState::next`, evaluated over one carrier cycle).
pub fn preview_pm(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    carrier: &TableSource,
    modulator: &TableSource,
    c_pos: f32,
    m_pos: f32,
    amount: f32,
    c_ratio: f32,
    m_ratio: f32,
) {
    let mf = m_ratio / c_ratio.max(0.0001);
    wave_preview(ui, width, color, move |t| {
        let mv = modulator.sample((t * mf).rem_euclid(1.0), m_pos);
        carrier.sample((t + amount * mv).rem_euclid(1.0), c_pos)
    });
}

// -------------------------------------------------------------------------------------------------
// Noise
// -------------------------------------------------------------------------------------------------

/// Noise strip. Not periodic: a freshly-seeded generator is run along the
/// strip so white/pink/brown show their character (jitter / rolloff), seeded
/// per-node so different noise nodes look different but stay flicker-free.
pub fn preview_noise(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    noise_color: NoiseColor,
    seed: u32,
) {
    let mut state = NoiseState::default();
    state.seed(seed.wrapping_mul(0x9E37_79B9));
    wave_preview(ui, width, color, move |_| state.sample(noise_color));
}

/// Cheap deterministic integer hash for the chiptune noise steps.
fn hash_noise(mut x: u32) -> u32 {
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}

// -------------------------------------------------------------------------------------------------
// Envelope (ADSR)
// -------------------------------------------------------------------------------------------------

/// ADSR preview, bottom-anchored. One cycle shows attack 0→1, decay 1→sustain,
/// a sustain hold, then release sustain→0, all with the same exponential
/// curvature the engine's `EnvStage` uses. Segment widths use softened times
/// (sqrt) so even a short attack stays visible, with a fixed share reserved for
/// the (indefinite) sustain hold.
pub fn preview_env(
    ui: &mut egui::Ui,
    width: f32,
    color: egui::Color32,
    attack_ms: f32,
    decay_ms: f32,
    sustain: f32,
    release_ms: f32,
) {
    let sustain = sustain.clamp(0.0, 1.0);

    // Softened (sqrt) time weights; reserve a constant share for sustain.
    let wa = attack_ms.max(0.0).sqrt();
    let wd = decay_ms.max(0.0).sqrt();
    let wr = release_ms.max(0.0).sqrt();
    let ws = 0.35f32;
    let total = (wa + wd + ws + wr).max(1e-4);

    let a_end = wa / total;
    let d_end = a_end + wd / total;
    let s_end = d_end + ws / total;

    wave_preview_baseline(ui, width, color, Baseline::Bottom, move |t| {
        if t < a_end {
            let x = t / a_end.max(1e-4);
            1.0 - (-5.0 * x).exp()
        } else if t < d_end {
            let x = (t - a_end) / (d_end - a_end).max(1e-4);
            sustain + (1.0 - sustain) * (-5.0 * x).exp()
        } else if t < s_end {
            sustain
        } else {
            let x = (t - s_end) / (1.0 - s_end).max(1e-4);
            sustain * (-5.0 * x).exp()
        }
    });
}
