use super::knob::section_label;
use super::theme as th;
use crate::engine::lfo::LfoShape;
use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, pos2, vec2};

const CELL_MIN: f32 = 38.0;
const CELL_H: f32 = 30.0;
const GAP: f32 = 4.0;
const SAMPLES: usize = 33;
const EDGE_JUMP_FRACTION: f32 = 0.42;

pub trait WaveShape: Copy + PartialEq {
    fn sample_at(self, t: f32) -> f32;
    fn label(self) -> String;
}

impl WaveShape for LfoShape {
    #[inline(always)]
    fn sample_at(self, t: f32) -> f32 {
        self.sample(t)
    }
    #[inline]
    fn label(self) -> String {
        self.to_string()
    }
}

impl WaveShape for crate::engine::osc::AnalogShape {
    #[inline(always)]
    fn sample_at(self, t: f32) -> f32 {
        match self {
            crate::engine::osc::AnalogShape::Saw => 2.0 * t - 1.0,
            crate::engine::osc::AnalogShape::ReverseSaw => 1.0 - 2.0 * t,
            crate::engine::osc::AnalogShape::Square => {
                if t < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            crate::engine::osc::AnalogShape::Triangle => {
                if t < 0.5 {
                    -1.0 + 4.0 * t
                } else {
                    1.0 - 4.0 * (t - 0.5)
                }
            }
            crate::engine::osc::AnalogShape::Sharktooth => {
                let split = 0.78;
                if t < split {
                    let normalized = t / split;
                    -1.0 + 2.0 * normalized * normalized
                } else {
                    1.0 - 2.0 * (t - split) / (1.0 - split)
                }
            }
            crate::engine::osc::AnalogShape::Sine => (t * std::f32::consts::TAU).sin(),
            crate::engine::osc::AnalogShape::Morph => {
                if t < 0.5 {
                    -1.0 + 4.0 * t
                } else {
                    1.0 - 4.0 * (t - 0.5)
                }
            }
        }
    }
    #[inline]
    fn label(self) -> String {
        self.to_string()
    }
}

pub fn wave_shape_picker<S: WaveShape>(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut S,
    all: &[S],
    accent: Color32,
) -> bool {
    if all.is_empty() {
        return false;
    }

    section_label(ui, "SHAPE");

    let width = ui.available_width().max(CELL_MIN);
    let shape_count = all.len();
    let per_row = (((width + GAP) / (CELL_MIN + GAP)).floor() as usize).clamp(1, shape_count);
    let row_count = shape_count.div_ceil(per_row);
    let cell_width = (width - GAP * (per_row - 1) as f32) / per_row as f32;
    let height = row_count as f32 * CELL_H + (row_count - 1) as f32 * GAP;

    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let base = ui.scope_id().with(id);
    let painter = ui.painter().clone();
    let mut changed = false;

    for (shape_index, &shape) in all.iter().enumerate() {
        let column = shape_index % per_row;
        let row = shape_index / per_row;
        let cell = Rect::from_min_size(
            pos2(
                rect.left() + column as f32 * (cell_width + GAP),
                rect.top() + row as f32 * (CELL_H + GAP),
            ),
            vec2(cell_width, CELL_H),
        );

        let resp = ui.interact(cell, base.with(shape_index), Sense::click());
        if resp.clicked() && *value != shape {
            *value = shape;
            changed = true;
        }

        let active = *value == shape;
        let hot = resp.hovered();
        let background = if active {
            th::alpha(accent, 42)
        } else if hot {
            th::WIDGET_HOT
        } else {
            th::INSET_BG
        };
        let edge = if active {
            accent
        } else if hot {
            th::alpha(accent, 120)
        } else {
            th::INSET_EDGE
        };

        painter.rect_filled(cell, 4.0, background);
        painter.rect_stroke(
            cell,
            4.0,
            Stroke::new(if active { 1.3 } else { 1.0 }, edge),
            egui::StrokeKind::Inside,
        );

        let plot = cell.shrink2(vec2(5.0, 6.0));
        let midline = plot.center().y;
        let half_height = plot.height() * 0.5;
        painter.line_segment(
            [pos2(plot.left(), midline), pos2(plot.right(), midline)],
            Stroke::new(
                1.0,
                th::alpha(th::GRID_MAJOR, if active { 210 } else { 130 }),
            ),
        );

        let mut points = [Pos2::ZERO; SAMPLES * 2];
        let mut point_count = 0usize;
        let mut previous_y = midline;
        for sample_index in 0..SAMPLES {
            let normalized_time = sample_index as f32 / (SAMPLES - 1) as f32;
            let x = plot.left() + plot.width() * normalized_time;
            let y = midline - shape.sample_at(normalized_time).clamp(-1.0, 1.0) * half_height;
            if sample_index > 0 && (y - previous_y).abs() > plot.height() * EDGE_JUMP_FRACTION {
                points[point_count] = pos2(x, previous_y);
                point_count += 1;
            }
            points[point_count] = pos2(x, y);
            point_count += 1;
            previous_y = y;
        }

        let line = &points[..point_count];
        let color = if active {
            accent
        } else if hot {
            th::TEXT_BRIGHT
        } else {
            th::TEXT_DIM
        };
        if active {
            painter.add(Shape::line(
                line.to_vec(),
                Stroke::new(3.6, th::alpha(accent, 46)),
            ));
        }
        painter.add(Shape::line(
            line.to_vec(),
            Stroke::new(if active { 1.7 } else { 1.3 }, color),
        ));

        if active {
            painter.rect_filled(
                Rect::from_min_size(
                    pos2(cell.left() + 6.0, cell.bottom() - 3.0),
                    vec2(cell_width - 12.0, 2.0),
                ),
                1.0,
                accent,
            );
        }

        if hot {
            resp.on_hover_text(shape.label());
        }
    }

    changed
}
