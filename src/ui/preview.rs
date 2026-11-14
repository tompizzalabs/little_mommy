use super::theme as th;
use crate::engine::lfo::LfoShape;
use crate::engine::osc::analog::SQUARE_VOLUME_SCALAR;
use crate::engine::osc::analog::analog_tables::{self, TABLE_LEN, table_index};
use crate::engine::osc::fm::pitch_shift_multiplier;
use crate::engine::osc::OperatorWave;
use crate::engine::osc::{AnalogShape, NoiseColor, NoiseState};
use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, pos2, vec2};

pub const PREVIEW_H: f32 = 54.0;
const EDGE_FRAC: f32 = 0.35;
const N: usize = 128;
const DT: f32 = 1.0 / (N - 1) as f32;
const MASK: usize = TABLE_LEN - 1;
const TLEN: f32 = TABLE_LEN as f32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Baseline {
    Center,
    Bottom,
}

#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline(always)]
fn read(table: &[f32], pos: f32) -> f32 {
    let i0 = pos as usize & MASK;
    let i1 = (i0 + 1) & MASK;
    lerp(table[i0], table[i1], pos - pos.floor())
}

fn strip(ui: &mut egui::Ui, width: f32, height: f32, baseline: Baseline) -> (Rect, egui::Painter) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 4.0, th::INSET_BG);
    for i in 1..4 {
        let x = rect.left() + rect.width() * i as f32 / 4.0;
        p.line_segment(
            [pos2(x, rect.top() + 3.0), pos2(x, rect.bottom() - 3.0)],
            Stroke::new(1.0, th::GRID_LINE),
        );
    }
    let base_y = match baseline {
        Baseline::Center => rect.center().y,
        Baseline::Bottom => rect.bottom() - 4.0,
    };
    p.line_segment(
        [
            pos2(rect.left() + 3.0, base_y),
            pos2(rect.right() - 3.0, base_y),
        ],
        Stroke::new(1.0, th::GRID_MAJOR),
    );
    p.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, th::INSET_EDGE),
        egui::StrokeKind::Inside,
    );
    (rect, p)
}

pub fn wave_preview(ui: &mut egui::Ui, width: f32, color: Color32, sample: impl FnMut(f32) -> f32) {
    wave_preview_baseline(ui, width, color, Baseline::Center, sample);
}

pub fn wave_preview_baseline(
    ui: &mut egui::Ui,
    width: f32,
    color: Color32,
    baseline: Baseline,
    mut sample: impl FnMut(f32) -> f32,
) -> Rect {
    let (rect, painter) = strip(ui, width, PREVIEW_H, baseline);
    let (base_y, amp, lo) = match baseline {
        Baseline::Center => (rect.center().y, rect.height() * 0.5 - 5.0, -1.0),
        Baseline::Bottom => (rect.bottom() - 4.0, rect.height() - 9.0, 0.0),
    };
    let x0 = rect.left() + 3.0;
    let dx = (rect.width() - 6.0) * DT;
    let edge = amp * EDGE_FRAC;
    let mut pts = [Pos2::ZERO; N * 2];
    let mut len = 0usize;
    let mut prev_y = base_y;

    for i in 0..N {
        let x = x0 + i as f32 * dx;
        let y = base_y - sample(i as f32 * DT).clamp(lo, 1.0) * amp;
        if i > 0 && (y - prev_y).abs() > edge {
            pts[len] = pos2(x, prev_y);
            len += 1;
        }
        pts[len] = pos2(x, y);
        len += 1;
        prev_y = y;
    }

    let line = &pts[..len];
    if baseline == Baseline::Bottom {
        painter.add(th::fill_under(line, base_y, th::alpha(color, 42)));
    }
    painter.add(Shape::line(
        line.to_vec(),
        Stroke::new(4.5, th::alpha(color, 26)),
    ));
    painter.add(Shape::line(line.to_vec(), Stroke::new(1.6, color)));
    rect
}

#[inline]
pub fn preview_lfo(ui: &mut egui::Ui, width: f32, color: Color32, shape: LfoShape) {
    wave_preview(ui, width, color, |t| shape.sample(t));
}

pub fn preview_analog(
    ui: &mut egui::Ui,
    width: f32,
    color: Color32,
    shape: AnalogShape,
    pulse_width: f32,
) {
    let tables = &*analog_tables::get();
    let idx = table_index(440.0);
    let saw = &tables.saw[idx];

    if let AnalogShape::Square = shape {
        let d = pulse_width.clamp(0.02, 0.98);
        let off = d * TLEN;
        let scale = 2.0 * (1.0 - d).max(d) * SQUARE_VOLUME_SCALAR;
        wave_preview(ui, width, color, move |t| {
            let pos = t * TLEN;
            (read(saw, pos) - read(saw, pos + off)) * scale
        });
        return;
    }

    let table: &[f32] = match shape {
        AnalogShape::Triangle => &tables.triangle[idx],
        AnalogShape::Sine => &tables.sine,
        _ => saw,
    };
    wave_preview(ui, width, color, move |t| read(table, t * TLEN));
}

/// One table read for an operator wave, at an arbitrary phase in [0, 1).
#[inline(always)]
fn operator_sample(wave: OperatorWave, phase: f32) -> f32 {
    let tables = analog_tables::get();
    let idx = table_index(440.0);
    let pos = phase.rem_euclid(1.0) * TLEN;
    match wave {
        OperatorWave::Saw => read(&tables.saw[idx], pos),
        OperatorWave::Triangle => read(&tables.triangle[idx], pos),
        OperatorWave::Sine => read(&tables.sine, pos),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn preview_fm(
    ui: &mut egui::Ui,
    width: f32,
    color: Color32,
    carrier: OperatorWave,
    modulator: OperatorWave,
    amount: f32,
    c_ratio: f32,
    m_ratio: f32,
    exponential: bool,
) {
    let mf = m_ratio / c_ratio.max(0.0001);
    let amount = amount.max(0.0);
    let lin = 15.0 * amount;
    let semis = amount * 24.0;
    let mut phase = 0.0f32;
    let mut last_t = 0.0f32;
    wave_preview(ui, width, color, move |t| {
        let dt = (t - last_t).max(0.0);
        last_t = t;
        let mv = operator_sample(modulator, t * mf);
        let mult = if exponential {
            pitch_shift_multiplier(mv * semis)
        } else {
            1.0 + mv * lin
        };
        phase += mult * dt;
        operator_sample(carrier, phase)
    });
}

#[allow(clippy::too_many_arguments)]
pub fn preview_pm(
    ui: &mut egui::Ui,
    width: f32,
    color: Color32,
    carrier: OperatorWave,
    modulator: OperatorWave,
    amount: f32,
    c_ratio: f32,
    m_ratio: f32,
) {
    let mf = m_ratio / c_ratio.max(0.0001);
    wave_preview(ui, width, color, move |t| {
        let mv = operator_sample(modulator, t * mf);
        operator_sample(carrier, (t + amount * mv).rem_euclid(1.0))
    });
}

pub fn preview_noise(
    ui: &mut egui::Ui,
    width: f32,
    color: Color32,
    noise_color: NoiseColor,
    seed: u32,
) {
    let mut state = NoiseState::default();
    state.seed(seed.wrapping_mul(0x9E37_79B9));
    wave_preview(ui, width, color, move |_| state.sample(noise_color));
}

pub fn preview_env(
    ui: &mut egui::Ui,
    width: f32,
    color: Color32,
    attack_ms: f32,
    decay_ms: f32,
    sustain: f32,
    release_ms: f32,
) {
    let s = sustain.clamp(0.0, 1.0);
    let s1 = 1.0 - s;
    let wa = attack_ms.max(0.0).sqrt();
    let wd = decay_ms.max(0.0).sqrt();
    let wr = release_ms.max(0.0).sqrt();
    let inv_total = 1.0 / (wa + wd + 0.35 + wr).max(1e-4);
    let a_end = wa * inv_total;
    let d_end = a_end + wd * inv_total;
    let s_end = d_end + 0.35 * inv_total;
    let inv_a = 1.0 / a_end.max(1e-4);
    let inv_d = 1.0 / (d_end - a_end).max(1e-4);
    let inv_r = 1.0 / (1.0 - s_end).max(1e-4);

    let rect = wave_preview_baseline(ui, width, color, Baseline::Bottom, move |t| {
        if t < a_end {
            1.0 - (-5.0 * t * inv_a).exp()
        } else if t < d_end {
            s + s1 * (-5.0 * (t - a_end) * inv_d).exp()
        } else if t < s_end {
            s
        } else {
            s * (-5.0 * (t - s_end) * inv_r).exp()
        }
    });

    let p = ui.painter_at(rect);
    let base_y = rect.bottom() - 4.0;
    let amp = rect.height() - 9.0;
    let x0 = rect.left() + 3.0;
    let w = rect.width() - 6.0;
    let at = |t: f32, v: f32| pos2(x0 + w * t, base_y - amp * v);

    let sus_a = at(d_end, s);
    let sus_b = at(s_end, s);
    th::dashed(
        &p,
        sus_a,
        sus_b,
        4.0,
        3.0,
        Stroke::new(1.2, th::alpha(color, 150)),
    );

    for (t, v) in [(a_end, 1.0), (d_end, s), (s_end, s), (1.0, 0.0)] {
        let pos = at(t, v);
        p.circle_filled(pos, 4.6, th::alpha(color, 40));
        p.circle_filled(pos, 2.6, color);
        p.circle_filled(pos, 1.0, th::POINTER);
    }
}
