use super::theme::{self as th, Glyph};
use egui::{Color32, Pos2, Rect, Sense, Stroke, pos2, vec2};

pub struct KnobOut {
    pub response: egui::Response,
    pub mod_pos: Pos2,
}

pub struct Knob<'a> {
    value: &'a mut f32,
    min: f32,
    max: f32,
    accent: Color32,
    size: f32,
    label: &'a str,
    text: Option<String>,
    default: Option<f32>,
    bipolar: bool,
    depth: Option<&'a mut f32>,
    integer: bool,
}

impl<'a> Knob<'a> {
    pub fn new(value: &'a mut f32, min: f32, max: f32) -> Self {
        Self {
            value,
            min,
            max,
            accent: th::AUDIO,
            size: 34.0,
            label: "",
            text: None,
            default: None,
            bipolar: min < 0.0 && max > 0.0,
            depth: None,
            integer: false,
        }
    }

    pub fn accent(mut self, c: Color32) -> Self {
        self.accent = c;
        self
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    pub fn label(mut self, l: &'a str) -> Self {
        self.label = l;
        self
    }
    pub fn text(mut self, t: String) -> Self {
        self.text = Some(t);
        self
    }
    pub fn default(mut self, d: f32) -> Self {
        self.default = Some(d);
        self
    }
    pub fn bipolar(mut self, b: bool) -> Self {
        self.bipolar = b;
        self
    }
    pub fn integer(mut self, i: bool) -> Self {
        self.integer = i;
        self
    }
    pub fn depth(mut self, d: Option<&'a mut f32>) -> Self {
        self.depth = Some(d).flatten();
        self
    }

    pub fn ui(self, ui: &mut egui::Ui) -> KnobOut {
        let has_depth = self.depth.is_some();
        let w = self.size + 14.0;
        let h = self.size + 24.0 + if has_depth { 9.0 } else { 0.0 };
        let (rect, mut resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
        let span = (self.max - self.min).max(1e-9);
        let mut value = *self.value;

        if resp.dragged() {
            let (delta, fine) = ui.input(|i| (i.pointer.delta(), i.modifiers.shift));
            let speed = if fine { 0.16 } else { 1.0 };
            let d = (-delta.y + delta.x) * speed / 190.0;
            value = (value + d * span).clamp(self.min, self.max);
            if self.integer {
                value = value.round();
            }
        }
        if resp.double_clicked() {
            if let Some(d) = self.default {
                value = d.clamp(self.min, self.max);
            }
        }
        let changed = (value - *self.value).abs() > f32::EPSILON;
        if changed {
            *self.value = value;
            resp.mark_changed();
        }

        let t = ((value - self.min) / span).clamp(0.0, 1.0);
        let r = self.size * 0.5;
        let center = pos2(rect.left() + 12.0 + r, rect.top() + r);
        let linked = has_depth;
        let arc_color = if linked { th::MOD } else { self.accent };
        let painter = ui.painter();

        if resp.hovered() || resp.dragged() {
            painter.circle_filled(center, r + 3.0, th::alpha(arc_color, 26));
        }
        painter.circle_filled(center, r, th::KNOB_RING);
        painter.add(th::arc(
            center,
            r - 1.6,
            0.0,
            1.0,
            Stroke::new(2.6, th::KNOB_TRACK),
        ));
        if self.bipolar {
            painter.add(th::arc(
                center,
                r - 1.6,
                0.5,
                t,
                Stroke::new(2.6, arc_color),
            ));
        } else {
            painter.add(th::arc(
                center,
                r - 1.6,
                0.0,
                t,
                Stroke::new(2.6, arc_color),
            ));
        }
        painter.add(th::disc_gradient(
            center,
            r * 0.74,
            th::KNOB_TOP,
            th::KNOB_BOTTOM,
        ));
        painter.circle_stroke(
            center,
            r * 0.74,
            Stroke::new(1.0, Color32::from_black_alpha(110)),
        );
        let dir = th::knob_dir(t);
        painter.line_segment(
            [center + dir * (r * 0.20), center + dir * (r * 0.66)],
            Stroke::new(2.0, th::POINTER),
        );

        let text = self
            .text
            .unwrap_or_else(|| fmt_param(self.label, value, self.min, self.max, self.integer));
        painter.text(
            pos2(center.x, rect.top() + self.size + 7.0),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(10.0),
            arc_color,
        );
        painter.text(
            pos2(center.x, rect.top() + self.size + 18.0),
            egui::Align2::CENTER_CENTER,
            self.label,
            egui::FontId::proportional(9.0),
            th::TEXT_DIM,
        );

        let mod_pos = pos2(rect.left() + 4.0, center.y);
        th::port(
            painter,
            mod_pos,
            if linked { th::MOD } else { th::TEXT_FAINT },
            false,
            false,
        );

        if let Some(depth) = self.depth {
            let bar = Rect::from_min_size(
                pos2(center.x - r, rect.top() + self.size + 25.0),
                vec2(self.size, 4.0),
            );
            let dr = ui.interact(bar, resp.id.with("depth"), Sense::click_and_drag());
            let mut d = *depth;
            if dr.dragged() {
                d = (d + ui.input(|i| i.pointer.delta().x) / 90.0).clamp(-1.0, 1.0);
            }
            if dr.double_clicked() {
                d = 1.0;
            }
            if (d - *depth).abs() > f32::EPSILON {
                *depth = d;
                resp.mark_changed();
            }
            let p = ui.painter();
            p.rect_filled(bar, 2.0, th::KNOB_TRACK);
            let mid = bar.center().x;
            let end = mid + bar.width() * 0.5 * *depth;
            let fill = Rect::from_min_max(
                pos2(mid.min(end), bar.top()),
                pos2(mid.max(end), bar.bottom()),
            );
            p.rect_filled(fill, 2.0, th::MOD);
            p.rect_filled(
                Rect::from_center_size(pos2(mid, bar.center().y), vec2(1.0, bar.height())),
                0.0,
                th::TEXT_FAINT,
            );
            if dr.hovered() {
                p.rect_stroke(
                    bar.expand(1.5),
                    3.0,
                    Stroke::new(1.0, th::alpha(th::MOD, 120)),
                    egui::StrokeKind::Outside,
                );
            }
        }

        KnobOut {
            response: resp,
            mod_pos,
        }
    }
}

pub fn fmt_param(name: &str, v: f32, min: f32, max: f32, integer: bool) -> String {
    let n = name.to_ascii_lowercase();
    let span = max - min;
    if integer || span >= 999.0 {
        if v.abs() >= 1000.0 {
            return format!("{:.2}k", v / 1000.0);
        }
        return format!("{:.0}", v.round());
    }
    if n.contains("attack") || n.contains("decay") || n.contains("release") || n.contains("time") {
        return format!("{:.0} ms", v);
    }
    if n.contains("freq") || n.contains("cutoff") || n.contains("hz") {
        return if v >= 1000.0 {
            format!("{:.2} kHz", v / 1000.0)
        } else {
            format!("{:.0} Hz", v)
        };
    }
    if n.contains("detune") || n.contains("spread") || n.contains("semitone") {
        return format!("{:+.2} st", v);
    }
    if n.contains("ratio") {
        return format!("{:.0}:1", v);
    }
    if span <= 1.001 && min >= 0.0 {
        return format!("{:.0}%", v * 100.0);
    }
    if min < 0.0 {
        return format!("{:+.2}", v);
    }
    format!("{:.2}", v)
}

pub fn chip(ui: &mut egui::Ui, _id: &str, on: &mut bool, label: &str, accent: Color32) -> bool {
    let font = egui::FontId::proportional(10.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, th::TEXT_BRIGHT);
    let size = galley.size() + vec2(16.0, 8.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let active = *on;
    let bg = if active {
        th::alpha(accent, 46)
    } else if resp.hovered() {
        th::WIDGET_HOT
    } else {
        th::WIDGET
    };
    let p = ui.painter();
    p.rect_filled(rect, 4.0, bg);
    p.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, if active { accent } else { th::INSET_EDGE }),
        egui::StrokeKind::Inside,
    );
    p.galley(
        rect.center() - galley.size() * 0.5,
        galley,
        if active { accent } else { th::TEXT_DIM },
    );
    resp.changed()
}

pub fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut T,
    options: &[(T, &str)],
    accent: Color32,
) -> bool {
    let width = ui.available_width();
    let h = 20.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let base = ui.scope_id().with(id);
    let cell = width / options.len().max(1) as f32;
    let mut changed = false;
    let p = ui.painter().clone();
    p.rect_filled(rect, 4.0, th::INSET_BG);
    p.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, th::INSET_EDGE),
        egui::StrokeKind::Inside,
    );
    for (i, (v, label)) in options.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(rect.left() + i as f32 * cell, rect.top()),
            vec2(cell, h),
        );
        let resp = ui.interact(r, base.with(i), Sense::click());
        if resp.clicked() && *value != *v {
            *value = *v;
            changed = true;
        }
        let active = *value == *v;
        let color = if active {
            accent
        } else if resp.hovered() {
            th::TEXT_BRIGHT
        } else {
            th::TEXT_FAINT
        };
        if resp.hovered() && !active {
            p.rect_filled(r.shrink(1.0), 3.0, th::alpha(Color32::WHITE, 8));
        }
        p.text(
            r.center() - vec2(0.0, 1.0),
            egui::Align2::CENTER_CENTER,
            *label,
            egui::FontId::proportional(10.0),
            color,
        );
        if active {
            p.rect_filled(
                Rect::from_min_size(
                    pos2(r.left() + 5.0, r.bottom() - 3.0),
                    vec2(cell - 10.0, 2.0),
                ),
                1.0,
                accent,
            );
        }
    }
    changed
}

pub fn glyph_row<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut T,
    options: &[(T, Glyph)],
    accent: Color32,
) -> bool {
    let width = ui.available_width();
    let h = 24.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let base = ui.scope_id().with(id);
    let cell = width / options.len().max(1) as f32;
    let mut changed = false;
    let p = ui.painter().clone();
    for (i, (v, g)) in options.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(rect.left() + i as f32 * cell, rect.top()),
            vec2(cell, h),
        );
        let resp = ui.interact(r, base.with(i), Sense::click());
        if resp.clicked() && *value != *v {
            *value = *v;
            changed = true;
        }
        let active = *value == *v;
        if active {
            p.rect_filled(r.shrink2(vec2(3.0, 1.0)), 4.0, th::alpha(accent, 34));
        } else if resp.hovered() {
            p.rect_filled(
                r.shrink2(vec2(3.0, 1.0)),
                4.0,
                th::alpha(Color32::WHITE, 10),
            );
        }
        th::draw_glyph(
            &p,
            r.shrink2(vec2(cell * 0.24, 5.0)),
            *g,
            if active { accent } else { th::TEXT_FAINT },
            1.4,
        );
    }
    changed
}

pub fn stepper(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut u8,
    min: u8,
    max: u8,
    label: &str,
    accent: Color32,
) -> bool {
    let h = 20.0;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let base = ui.scope_id().with(id);
    let p = ui.painter().clone();
    p.text(
        pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(10.0),
        th::TEXT_DIM,
    );
    let box_w = 62.0;
    let bx = rect.right() - box_w;
    let panel = Rect::from_min_size(pos2(bx, rect.top()), vec2(box_w, h));
    p.rect_filled(panel, 4.0, th::INSET_BG);
    p.rect_stroke(
        panel,
        4.0,
        Stroke::new(1.0, th::INSET_EDGE),
        egui::StrokeKind::Inside,
    );
    let mut changed = false;
    for (i, dir) in [(0usize, -1i32), (1usize, 1i32)] {
        let r = Rect::from_min_size(
            pos2(if dir < 0 { bx } else { panel.right() - 18.0 }, rect.top()),
            vec2(18.0, h),
        );
        let resp = ui.interact(r, base.with(i), Sense::click());
        if resp.clicked() {
            let next = (*value as i32 + dir).clamp(min as i32, max as i32) as u8;
            if next != *value {
                *value = next;
                changed = true;
            }
        }
        if resp.hovered() {
            p.rect_filled(r.shrink(1.0), 3.0, th::alpha(Color32::WHITE, 12));
        }
        let c = r.center();
        let s = 3.2;
        let tri = if dir < 0 {
            vec![
                pos2(c.x + s * 0.6, c.y - s),
                pos2(c.x + s * 0.6, c.y + s),
                pos2(c.x - s * 0.7, c.y),
            ]
        } else {
            vec![
                pos2(c.x - s * 0.6, c.y - s),
                pos2(c.x - s * 0.6, c.y + s),
                pos2(c.x + s * 0.7, c.y),
            ]
        };
        p.add(egui::Shape::convex_polygon(tri, th::TEXT_DIM, Stroke::NONE));
    }
    p.text(
        panel.center(),
        egui::Align2::CENTER_CENTER,
        format!("{}", *value),
        egui::FontId::proportional(11.0),
        accent,
    );
    changed
}

pub fn xy_pad(
    ui: &mut egui::Ui,
    _id: &str,
    x: &mut f32,
    y: &mut f32,
    size: f32,
    accent: Color32,
) -> bool {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(size, size), Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        let d = resp.drag_delta();
        let nx = (*x + d.x / size).clamp(0.0, 1.0);
        let ny = (*y - d.y / size).clamp(0.0, 1.0);
        if (nx - *x).abs() > f32::EPSILON || (ny - *y).abs() > f32::EPSILON {
            *x = nx;
            *y = ny;
            changed = true;
            resp.mark_changed();
        }
    }
    if resp.double_clicked() {
        *x = 0.5;
        *y = 0.5;
        changed = true;
        resp.mark_changed();
    }
    let p = ui.painter();
    p.rect_filled(rect, 4.0, th::INSET_BG);
    for i in 1..4 {
        let f = i as f32 / 4.0;
        p.line_segment(
            [
                pos2(rect.left() + rect.width() * f, rect.top()),
                pos2(rect.left() + rect.width() * f, rect.bottom()),
            ],
            Stroke::new(1.0, th::GRID_LINE),
        );
        p.line_segment(
            [
                pos2(rect.left(), rect.top() + rect.height() * f),
                pos2(rect.right(), rect.top() + rect.height() * f),
            ],
            Stroke::new(1.0, th::GRID_LINE),
        );
    }
    p.rect_stroke(
        rect,
        4.0,
        Stroke::new(
            1.0,
            if resp.hovered() {
                th::alpha(accent, 130)
            } else {
                th::INSET_EDGE
            },
        ),
        egui::StrokeKind::Inside,
    );
    let dot = pos2(
        rect.left() + rect.width() * *x,
        rect.bottom() - rect.height() * *y,
    );
    p.line_segment(
        [pos2(dot.x, rect.top()), pos2(dot.x, rect.bottom())],
        Stroke::new(1.0, th::alpha(accent, 70)),
    );
    p.line_segment(
        [pos2(rect.left(), dot.y), pos2(rect.right(), dot.y)],
        Stroke::new(1.0, th::alpha(accent, 70)),
    );
    p.circle_filled(dot, 7.0, th::alpha(accent, 40));
    p.circle_filled(dot, 3.4, accent);
    p.circle_filled(dot, 1.3, th::POINTER);
    changed
}

pub fn section_label(ui: &mut egui::Ui, text: &str) {
    let font = egui::FontId::proportional(9.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, th::TEXT_FAINT);
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), galley.size().y + 4.0),
        Sense::hover(),
    );
    ui.painter()
        .galley(pos2(rect.left(), rect.top() + 2.0), galley, th::TEXT_FAINT);
}
