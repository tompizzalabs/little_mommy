use super::theme::{self as th};
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
    default: Option<f32>,
    bipolar: bool,
    log_scale: bool,
    depth: Option<&'a mut f32>,
    formatter: Option<fn(f32) -> String>,
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
            default: None,
            bipolar: min < 0.0 && max > 0.0,
            log_scale: false,
            depth: None,
            formatter: None,
        }
    }

    pub fn log(mut self) -> Self {
        self.log_scale = self.min > 0.0;
        self
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
    pub fn default(mut self, d: f32) -> Self {
        self.default = Some(d);
        self
    }
    pub fn bipolar(mut self, b: bool) -> Self {
        self.bipolar = b;
        self
    }
    pub fn formatter(mut self, formatter: fn(f32) -> String) -> Self {
        self.formatter = Some(formatter);
        self
    }
    pub fn depth(mut self, d: Option<&'a mut f32>) -> Self {
        self.depth = Some(d).flatten();
        self
    }

    pub fn ui(self, ui: &mut egui::Ui) -> KnobOut {
        let has_depth = self.depth.is_some();
        let width = self.size + 14.0;
        let height = self.size + 24.0 + if has_depth { 9.0 } else { 0.0 };
        let (rect, mut resp) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
        let span = (self.max - self.min).max(1e-9);
        let log_span = (self.max / self.min).max(1.000_1);
        let value_to_normalized = |value: f32| {
            if self.log_scale {
                (value.max(self.min) / self.min)
                    .log(log_span)
                    .clamp(0.0, 1.0)
            } else {
                ((value - self.min) / span).clamp(0.0, 1.0)
            }
        };
        let normalized_to_value = |normalized: f32| {
            if self.log_scale {
                self.min * log_span.powf(normalized)
            } else {
                self.min + normalized * span
            }
        };
        let mut normalized = value_to_normalized(*self.value);

        if resp.dragged() {
            let (pointer_delta, fine_mode) =
                ui.input(|input| (input.pointer.delta(), input.modifiers.shift));
            let speed = if fine_mode { 0.16 } else { 1.0 };
            let movement = (-pointer_delta.y + pointer_delta.x) * speed / 190.0;
            normalized = (normalized + movement).clamp(0.0, 1.0);
        }
        if resp.double_clicked()
            && let Some(default_value) = self.default
        {
            normalized = value_to_normalized(default_value);
        }
        let value = normalized_to_value(normalized);
        let changed = (value - *self.value).abs() > f32::EPSILON;
        if changed {
            *self.value = value;
            resp.mark_changed();
        }
        let radius = self.size * 0.5;
        let center = pos2(rect.left() + 12.0 + radius, rect.top() + radius);
        let linked = has_depth;
        let arc_color = if linked { th::MOD } else { self.accent };
        let painter = ui.painter();

        if resp.hovered() || resp.dragged() {
            painter.circle_filled(center, radius + 3.0, th::alpha(arc_color, 26));
        }
        painter.circle_filled(center, radius, th::KNOB_RING);
        painter.add(th::arc(
            center,
            radius - 1.6,
            0.0,
            1.0,
            Stroke::new(2.6, th::KNOB_TRACK),
        ));
        if self.bipolar {
            painter.add(th::arc(
                center,
                radius - 1.6,
                0.5,
                normalized,
                Stroke::new(2.6, arc_color),
            ));
        } else {
            painter.add(th::arc(
                center,
                radius - 1.6,
                0.0,
                normalized,
                Stroke::new(2.6, arc_color),
            ));
        }
        painter.add(th::disc_gradient(
            center,
            radius * 0.74,
            th::KNOB_TOP,
            th::KNOB_BOTTOM,
        ));
        painter.circle_stroke(
            center,
            radius * 0.74,
            Stroke::new(1.0, Color32::from_black_alpha(110)),
        );
        let direction = th::knob_dir(normalized);
        painter.line_segment(
            [
                center + direction * (radius * 0.20),
                center + direction * (radius * 0.66),
            ],
            Stroke::new(2.0, th::POINTER),
        );

        let text = match self.formatter {
            Some(formatter) => formatter(value),
            None => fmt_param(self.label, value, self.min, self.max),
        };
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
                pos2(center.x - radius, rect.top() + self.size + 25.0),
                vec2(self.size, 4.0),
            );
            let depth_response = ui.interact(bar, resp.id.with("depth"), Sense::click_and_drag());
            let mut depth_value = *depth;
            if depth_response.dragged() {
                depth_value = (depth_value + ui.input(|input| input.pointer.delta().x) / 90.0)
                    .clamp(-1.0, 1.0);
            }
            if depth_response.double_clicked() {
                depth_value = 1.0;
            }
            if (depth_value - *depth).abs() > f32::EPSILON {
                *depth = depth_value;
                resp.mark_changed();
            }
            let painter = ui.painter();
            painter.rect_filled(bar, 2.0, th::KNOB_TRACK);
            let midpoint = bar.center().x;
            let fill_end = midpoint + bar.width() * 0.5 * *depth;
            let fill = Rect::from_min_max(
                pos2(midpoint.min(fill_end), bar.top()),
                pos2(midpoint.max(fill_end), bar.bottom()),
            );
            painter.rect_filled(fill, 2.0, th::MOD);
            painter.rect_filled(
                Rect::from_center_size(pos2(midpoint, bar.center().y), vec2(1.0, bar.height())),
                0.0,
                th::TEXT_FAINT,
            );
            if depth_response.hovered() {
                painter.rect_stroke(
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

pub fn fmt_param(name: &str, value: f32, min: f32, max: f32) -> String {
    let lowercase_name = name.to_ascii_lowercase();
    let span = max - min;
    if span >= 999.0 {
        if value.abs() >= 1000.0 {
            return format!("{:.2}k", value / 1000.0);
        }
        return format!("{:.0}", value.round());
    }
    if lowercase_name.contains("attack")
        || lowercase_name.contains("decay")
        || lowercase_name.contains("release")
        || lowercase_name.contains("time")
    {
        return format!("{:.0} ms", value);
    }
    if lowercase_name.contains("freq")
        || lowercase_name.contains("cutoff")
        || lowercase_name.contains("hz")
    {
        return if value >= 1000.0 {
            format!("{:.2} kHz", value / 1000.0)
        } else {
            format!("{:.0} Hz", value)
        };
    }
    if lowercase_name.contains("detune")
        || lowercase_name.contains("spread")
        || lowercase_name.contains("semitone")
    {
        return format!("{:+.2} st", value);
    }
    if lowercase_name.contains("ratio") {
        return format!("{:.0}:1", value);
    }
    if span <= 1.001 && min >= 0.0 {
        return format!("{:.0}%", value * 100.0);
    }
    if min < 0.0 {
        return format!("{:+.2}", value);
    }
    format!("{:.2}", value)
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
    let background = if active {
        th::alpha(accent, 46)
    } else if resp.hovered() {
        th::WIDGET_HOT
    } else {
        th::WIDGET
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, background);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, if active { accent } else { th::INSET_EDGE }),
        egui::StrokeKind::Inside,
    );
    painter.galley(
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
    let height = 20.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let base = ui.scope_id().with(id);
    let cell_width = width / options.len().max(1) as f32;
    let mut changed = false;
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 4.0, th::INSET_BG);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, th::INSET_EDGE),
        egui::StrokeKind::Inside,
    );
    for (option_index, (option_value, label)) in options.iter().enumerate() {
        let option_rect = Rect::from_min_size(
            pos2(rect.left() + option_index as f32 * cell_width, rect.top()),
            vec2(cell_width, height),
        );
        let resp = ui.interact(option_rect, base.with(option_index), Sense::click());
        if resp.clicked() && *value != *option_value {
            *value = *option_value;
            changed = true;
        }
        let active = *value == *option_value;
        let color = if active {
            accent
        } else if resp.hovered() {
            th::TEXT_BRIGHT
        } else {
            th::TEXT_FAINT
        };
        if resp.hovered() && !active {
            painter.rect_filled(option_rect.shrink(1.0), 3.0, th::alpha(Color32::WHITE, 8));
        }
        painter.text(
            option_rect.center() - vec2(0.0, 1.0),
            egui::Align2::CENTER_CENTER,
            *label,
            egui::FontId::proportional(10.0),
            color,
        );
        if active {
            painter.rect_filled(
                Rect::from_min_size(
                    pos2(option_rect.left() + 5.0, option_rect.bottom() - 3.0),
                    vec2(cell_width - 10.0, 2.0),
                ),
                1.0,
                accent,
            );
        }
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
    let height = 20.0;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let base = ui.scope_id().with(id);
    let painter = ui.painter().clone();
    painter.text(
        pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(10.0),
        th::TEXT_DIM,
    );
    let box_width = 62.0;
    let box_left = rect.right() - box_width;
    let panel = Rect::from_min_size(pos2(box_left, rect.top()), vec2(box_width, height));
    painter.rect_filled(panel, 4.0, th::INSET_BG);
    painter.rect_stroke(
        panel,
        4.0,
        Stroke::new(1.0, th::INSET_EDGE),
        egui::StrokeKind::Inside,
    );
    let mut changed = false;
    for (button_index, step_direction) in [(0usize, -1i32), (1usize, 1i32)] {
        let button_rect = Rect::from_min_size(
            pos2(
                if step_direction < 0 {
                    box_left
                } else {
                    panel.right() - 18.0
                },
                rect.top(),
            ),
            vec2(18.0, height),
        );
        let resp = ui.interact(button_rect, base.with(button_index), Sense::click());
        if resp.clicked() {
            let next = (*value as i32 + step_direction).clamp(min as i32, max as i32) as u8;
            if next != *value {
                *value = next;
                changed = true;
            }
        }
        if resp.hovered() {
            painter.rect_filled(button_rect.shrink(1.0), 3.0, th::alpha(Color32::WHITE, 12));
        }
        let center = button_rect.center();
        let arrow_size = 3.2;
        let arrow_points = if step_direction < 0 {
            vec![
                pos2(center.x + arrow_size * 0.6, center.y - arrow_size),
                pos2(center.x + arrow_size * 0.6, center.y + arrow_size),
                pos2(center.x - arrow_size * 0.7, center.y),
            ]
        } else {
            vec![
                pos2(center.x - arrow_size * 0.6, center.y - arrow_size),
                pos2(center.x - arrow_size * 0.6, center.y + arrow_size),
                pos2(center.x + arrow_size * 0.7, center.y),
            ]
        };
        painter.add(egui::Shape::convex_polygon(
            arrow_points,
            th::TEXT_DIM,
            Stroke::NONE,
        ));
    }
    painter.text(
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
        let drag_delta = resp.drag_delta();
        let next_x = (*x + drag_delta.x / size).clamp(0.0, 1.0);
        let next_y = (*y - drag_delta.y / size).clamp(0.0, 1.0);
        if (next_x - *x).abs() > f32::EPSILON || (next_y - *y).abs() > f32::EPSILON {
            *x = next_x;
            *y = next_y;
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
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, th::INSET_BG);
    for division in 1..4 {
        let fraction = division as f32 / 4.0;
        painter.line_segment(
            [
                pos2(rect.left() + rect.width() * fraction, rect.top()),
                pos2(rect.left() + rect.width() * fraction, rect.bottom()),
            ],
            Stroke::new(1.0, th::GRID_LINE),
        );
        painter.line_segment(
            [
                pos2(rect.left(), rect.top() + rect.height() * fraction),
                pos2(rect.right(), rect.top() + rect.height() * fraction),
            ],
            Stroke::new(1.0, th::GRID_LINE),
        );
    }
    painter.rect_stroke(
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
    painter.line_segment(
        [pos2(dot.x, rect.top()), pos2(dot.x, rect.bottom())],
        Stroke::new(1.0, th::alpha(accent, 70)),
    );
    painter.line_segment(
        [pos2(rect.left(), dot.y), pos2(rect.right(), dot.y)],
        Stroke::new(1.0, th::alpha(accent, 70)),
    );
    painter.circle_filled(dot, 7.0, th::alpha(accent, 40));
    painter.circle_filled(dot, 3.4, accent);
    painter.circle_filled(dot, 1.3, th::POINTER);
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
