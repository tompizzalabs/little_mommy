use super::theme as th;
use crate::engine::lfo::LfoShape;
use crate::engine::osc::OperatorWave;
use crate::engine::osc::analog::SQUARE_VOLUME_SCALAR;
use crate::engine::osc::analog::analog_tables::{self, TABLE_LEN, table_index};
use crate::engine::osc::fm::pitch_shift_multiplier;
use crate::engine::osc::{AnalogShape, NoiseColor, NoiseState};
use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, pos2, vec2};

pub const PREVIEW_H: f32 = 54.0;
const EDGE_JUMP_FRACTION: f32 = 0.35;
const PREVIEW_SAMPLE_COUNT: usize = 128;
const SAMPLE_SPACING: f32 = 1.0 / (PREVIEW_SAMPLE_COUNT - 1) as f32;
const TABLE_MASK: usize = TABLE_LEN - 1;
const TABLE_LENGTH: f32 = TABLE_LEN as f32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Baseline {
    Center,
    Bottom,
}

#[inline(always)]
fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

#[inline(always)]
fn read(table: &[f32], position: f32) -> f32 {
    let first_index = position as usize & TABLE_MASK;
    let next_index = (first_index + 1) & TABLE_MASK;
    lerp(
        table[first_index],
        table[next_index],
        position - position.floor(),
    )
}

fn strip(ui: &mut egui::Ui, width: f32, height: f32, baseline: Baseline) -> (Rect, egui::Painter) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, th::INSET_BG);
    for division in 1..4 {
        let x = rect.left() + rect.width() * division as f32 / 4.0;
        painter.line_segment(
            [pos2(x, rect.top() + 3.0), pos2(x, rect.bottom() - 3.0)],
            Stroke::new(1.0, th::GRID_LINE),
        );
    }
    let baseline_y = match baseline {
        Baseline::Center => rect.center().y,
        Baseline::Bottom => rect.bottom() - 4.0,
    };
    painter.line_segment(
        [
            pos2(rect.left() + 3.0, baseline_y),
            pos2(rect.right() - 3.0, baseline_y),
        ],
        Stroke::new(1.0, th::GRID_MAJOR),
    );
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, th::INSET_EDGE),
        egui::StrokeKind::Inside,
    );
    (rect, painter)
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
    let (baseline_y, amplitude, minimum) = match baseline {
        Baseline::Center => (rect.center().y, rect.height() * 0.5 - 5.0, -1.0),
        Baseline::Bottom => (rect.bottom() - 4.0, rect.height() - 9.0, 0.0),
    };
    let line_start_x = rect.left() + 3.0;
    let sample_spacing_x = (rect.width() - 6.0) * SAMPLE_SPACING;
    let edge = amplitude * EDGE_JUMP_FRACTION;
    let mut points = [Pos2::ZERO; PREVIEW_SAMPLE_COUNT * 2];
    let mut point_count = 0usize;
    let mut previous_y = baseline_y;

    for sample_index in 0..PREVIEW_SAMPLE_COUNT {
        let x = line_start_x + sample_index as f32 * sample_spacing_x;
        let y = baseline_y
            - sample(sample_index as f32 * SAMPLE_SPACING).clamp(minimum, 1.0) * amplitude;
        if sample_index > 0 && (y - previous_y).abs() > edge {
            points[point_count] = pos2(x, previous_y);
            point_count += 1;
        }
        points[point_count] = pos2(x, y);
        point_count += 1;
        previous_y = y;
    }

    let line = &points[..point_count];
    if baseline == Baseline::Bottom {
        painter.add(th::fill_under(line, baseline_y, th::alpha(color, 42)));
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
    let tables = analog_tables::get();
    let mip_index = table_index(440.0);
    let saw = &tables.saw[mip_index];

    if let AnalogShape::Square = shape {
        let duty = pulse_width.clamp(0.02, 0.98);
        let offset = duty * TABLE_LENGTH;
        let scale = 2.0 * (1.0 - duty).max(duty) * SQUARE_VOLUME_SCALAR;
        wave_preview(ui, width, color, move |t| {
            let position = t * TABLE_LENGTH;
            (read(saw, position) - read(saw, position + offset)) * scale
        });
        return;
    }

    match shape {
        AnalogShape::ReverseSaw => {
            wave_preview(ui, width, color, move |t| -read(saw, t * TABLE_LENGTH));
        }
        AnalogShape::Sharktooth => {
            wave_preview(ui, width, color, |t| {
                let split = 0.78;
                if t < split {
                    let normalized = t / split;
                    -1.0 + 2.0 * normalized * normalized
                } else {
                    1.0 - 2.0 * (t - split) / (1.0 - split)
                }
            });
        }
        AnalogShape::Morph => {
            let triangle = &tables.triangle[mip_index];
            wave_preview(ui, width, color, move |t| {
                let position = t * TABLE_LENGTH;
                (read(saw, position) + read(triangle, position)) * 0.5
            });
        }
        _ => {
            let table: &[f32] = match shape {
                AnalogShape::Triangle => &tables.triangle[mip_index],
                AnalogShape::Sine => &tables.sine,
                _ => saw,
            };
            wave_preview(ui, width, color, move |t| read(table, t * TABLE_LENGTH));
        }
    }
}

#[inline(always)]
fn operator_sample(wave: OperatorWave, phase: f32) -> f32 {
    let tables = analog_tables::get();
    let mip_index = table_index(440.0);
    let position = phase.rem_euclid(1.0) * TABLE_LENGTH;
    match wave {
        OperatorWave::Saw => read(&tables.saw[mip_index], position),
        OperatorWave::Triangle => read(&tables.triangle[mip_index], position),
        OperatorWave::Sine => read(&tables.sine, position),
    }
}

pub fn preview_vector(ui: &mut egui::Ui, width: f32, color: Color32, x: f32, y: f32) {
    let x = x.clamp(0.0, 1.0);
    let y = y.clamp(0.0, 1.0);
    let tables = analog_tables::get();
    let mip_index = table_index(440.0);
    let saw = &tables.saw[mip_index];
    let triangle = &tables.triangle[mip_index];
    let sine = &tables.sine;
    wave_preview(ui, width, color, move |t| {
        let position = t * TABLE_LENGTH;
        let bottom_left_wave = read(saw, position);
        let top_left_wave = read(triangle, position);
        let top_right_wave = read(sine, position);
        let bottom_right_wave = -bottom_left_wave;
        let bottom_row = (1.0 - x) * bottom_left_wave + x * bottom_right_wave;
        let top_row = (1.0 - x) * top_left_wave + x * top_right_wave;
        (1.0 - y) * bottom_row + y * top_row
    });
}

#[allow(clippy::too_many_arguments)]
pub fn preview_fm(
    ui: &mut egui::Ui,
    width: f32,
    color: Color32,
    carrier: OperatorWave,
    modulator: OperatorWave,
    amount: f32,
    carrier_ratio: f32,
    modulator_ratio: f32,
    exponential: bool,
) {
    let modulator_frequency = modulator_ratio / carrier_ratio.max(0.0001);
    let amount = amount.max(0.0);
    let linear_depth = 15.0 * amount;
    let semitones = amount * 24.0;
    let mut phase = 0.0f32;
    let mut last_time = 0.0f32;
    wave_preview(ui, width, color, move |t| {
        let delta_time = (t - last_time).max(0.0);
        last_time = t;
        let modulator_value = operator_sample(modulator, t * modulator_frequency);
        let multiplier = if exponential {
            pitch_shift_multiplier(modulator_value * semitones)
        } else {
            1.0 + modulator_value * linear_depth
        };
        phase += multiplier * delta_time;
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
    carrier_ratio: f32,
    modulator_ratio: f32,
) {
    let modulator_frequency = modulator_ratio / carrier_ratio.max(0.0001);
    wave_preview(ui, width, color, move |t| {
        let modulator_value = operator_sample(modulator, t * modulator_frequency);
        operator_sample(carrier, (t + amount * modulator_value).rem_euclid(1.0))
    });
}

const NOISE_PREVIEW_SEED_MIX: u32 = 0x9E37_79B9;

pub fn preview_noise(
    ui: &mut egui::Ui,
    width: f32,
    color: Color32,
    noise_color: NoiseColor,
    seed: u32,
) {
    let mut state = NoiseState::default();
    state.seed(seed.wrapping_mul(NOISE_PREVIEW_SEED_MIX));
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
    let sustain_level = sustain.clamp(0.0, 1.0);
    let one_minus_sustain = 1.0 - sustain_level;
    let attack_weight = attack_ms.max(0.0).sqrt();
    let decay_weight = decay_ms.max(0.0).sqrt();
    let release_weight = release_ms.max(0.0).sqrt();
    let reciprocal_total = 1.0 / (attack_weight + decay_weight + 0.35 + release_weight).max(1e-4);
    let attack_end = attack_weight * reciprocal_total;
    let decay_end = attack_end + decay_weight * reciprocal_total;
    let sustain_end = decay_end + 0.35 * reciprocal_total;
    let reciprocal_attack = 1.0 / attack_end.max(1e-4);
    let reciprocal_decay = 1.0 / (decay_end - attack_end).max(1e-4);
    let reciprocal_release = 1.0 / (1.0 - sustain_end).max(1e-4);

    let rect = wave_preview_baseline(ui, width, color, Baseline::Bottom, move |t| {
        if t < attack_end {
            1.0 - (-5.0 * t * reciprocal_attack).exp()
        } else if t < decay_end {
            sustain_level + one_minus_sustain * (-5.0 * (t - attack_end) * reciprocal_decay).exp()
        } else if t < sustain_end {
            sustain_level
        } else {
            sustain_level * (-5.0 * (t - sustain_end) * reciprocal_release).exp()
        }
    });

    let painter = ui.painter_at(rect);
    let baseline_y = rect.bottom() - 4.0;
    let amplitude = rect.height() - 9.0;
    let line_start_x = rect.left() + 3.0;
    let line_width = rect.width() - 6.0;
    let point_at = |t: f32, value: f32| {
        pos2(
            line_start_x + line_width * t,
            baseline_y - amplitude * value,
        )
    };

    let sustain_start = point_at(decay_end, sustain_level);
    let sustain_finish = point_at(sustain_end, sustain_level);
    th::dashed(
        &painter,
        sustain_start,
        sustain_finish,
        4.0,
        3.0,
        Stroke::new(1.2, th::alpha(color, 150)),
    );

    for (t, v) in [
        (attack_end, 1.0),
        (decay_end, sustain_level),
        (sustain_end, sustain_level),
        (1.0, 0.0),
    ] {
        let position = point_at(t, v);
        painter.circle_filled(position, 4.6, th::alpha(color, 40));
        painter.circle_filled(position, 2.6, color);
        painter.circle_filled(position, 1.0, th::POINTER);
    }
}
