use super::knob::section_label;
use super::theme as th;
use crate::engine::lfo::LfoShape;
use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, pos2, vec2};

const CELL_MIN: f32 = 38.0;
const CELL_H: f32 = 30.0;
const GAP: f32 = 4.0;
const SAMPLES: usize = 33;
const EDGE_FRAC: f32 = 0.42;

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
    let n = all.len();
    let per_row = (((width + GAP) / (CELL_MIN + GAP)).floor() as usize).clamp(1, n);
    let rows = n.div_ceil(per_row);
    let cell_w = (width - GAP * (per_row - 1) as f32) / per_row as f32;
    let height = rows as f32 * CELL_H + (rows - 1) as f32 * GAP;

    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let base = ui.scope_id().with(id);
    let painter = ui.painter().clone();
    let mut changed = false;

    for (i, &shape) in all.iter().enumerate() {
        let col = i % per_row;
        let row = i / per_row;
        let cell = Rect::from_min_size(
            pos2(
                rect.left() + col as f32 * (cell_w + GAP),
                rect.top() + row as f32 * (CELL_H + GAP),
            ),
            vec2(cell_w, CELL_H),
        );

        let resp = ui.interact(cell, base.with(i), Sense::click());
        if resp.clicked() && *value != shape {
            *value = shape;
            changed = true;
        }

        let active = *value == shape;
        let hot = resp.hovered();
        let bg = if active {
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

        painter.rect_filled(cell, 4.0, bg);
        painter.rect_stroke(
            cell,
            4.0,
            Stroke::new(if active { 1.3 } else { 1.0 }, edge),
            egui::StrokeKind::Inside,
        );

        let plot = cell.shrink2(vec2(5.0, 6.0));
        let mid = plot.center().y;
        let half = plot.height() * 0.5;
        painter.line_segment(
            [pos2(plot.left(), mid), pos2(plot.right(), mid)],
            Stroke::new(
                1.0,
                th::alpha(th::GRID_MAJOR, if active { 210 } else { 130 }),
            ),
        );

        let mut pts = [Pos2::ZERO; SAMPLES * 2];
        let mut len = 0usize;
        let mut prev_y = mid;
        for k in 0..SAMPLES {
            let t = k as f32 / (SAMPLES - 1) as f32;
            let x = plot.left() + plot.width() * t;
            let y = mid - shape.sample_at(t).clamp(-1.0, 1.0) * half;
            if k > 0 && (y - prev_y).abs() > plot.height() * EDGE_FRAC {
                pts[len] = pos2(x, prev_y);
                len += 1;
            }
            pts[len] = pos2(x, y);
            len += 1;
            prev_y = y;
        }

        let line = &pts[..len];
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
                    vec2(cell_w - 12.0, 2.0),
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
