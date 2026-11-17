use crate::engine::node::NodeParams;
use egui::epaint::{Vertex, WHITE_UV};
use egui::{Color32, Mesh, Pos2, Rect, Shape, Stroke, Vec2, pos2, vec2};
use std::sync::OnceLock;

pub const CANVAS_BG: Color32 = Color32::from_rgb(9, 9, 11);
pub const SURFACE: Color32 = Color32::from_rgb(20, 20, 23);
pub const SURFACE_HI: Color32 = Color32::from_rgb(28, 28, 33);
pub const WIDGET: Color32 = Color32::from_rgb(38, 38, 44);
pub const WIDGET_HOT: Color32 = Color32::from_rgb(52, 52, 60);
pub const WIDGET_ON: Color32 = Color32::from_rgb(64, 64, 74);

pub const GRID_LINE: Color32 = Color32::from_rgb(22, 22, 26);
pub const GRID_MAJOR: Color32 = Color32::from_rgb(33, 33, 39);

pub const NODE_TOP: Color32 = Color32::from_rgb(48, 48, 55);
pub const NODE_BOTTOM: Color32 = Color32::from_rgb(32, 32, 37);
pub const NODE_EDGE: Color32 = Color32::from_rgb(64, 64, 73);
pub const INSET_BG: Color32 = Color32::from_rgb(17, 17, 20);
pub const INSET_EDGE: Color32 = Color32::from_rgb(38, 38, 45);

pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(228, 228, 236);
pub const TEXT_DIM: Color32 = Color32::from_rgb(146, 146, 158);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(96, 96, 106);

pub const AUDIO: Color32 = Color32::from_rgb(240, 52, 44);
pub const MOD: Color32 = Color32::from_rgb(38, 214, 122);
pub const GATE: Color32 = Color32::from_rgb(255, 176, 32);

pub const CV: Color32 = Color32::from_rgb(41, 182, 255);
pub const WARN: Color32 = Color32::from_rgb(255, 138, 40);

pub const KNOB_RING: Color32 = Color32::from_rgb(22, 22, 26);
pub const KNOB_TRACK: Color32 = Color32::from_rgb(30, 30, 36);
pub const KNOB_TOP: Color32 = Color32::from_rgb(82, 82, 93);
pub const KNOB_BOTTOM: Color32 = Color32::from_rgb(44, 44, 52);
pub const POINTER: Color32 = Color32::from_rgb(240, 240, 246);

pub const NODE_W: f32 = 236.0;
pub const HEADER_H: f32 = 28.0;
pub const PAD: f32 = 10.0;
pub const BODY_W: f32 = NODE_W - PAD * 2.0;
pub const GRID_STEP: f32 = 54.0;

const DEG: f32 = std::f32::consts::PI / 180.0;
pub const ARC_SEG: usize = 30;
const DISC_SEG: usize = 28;

#[inline(always)]
pub fn alpha(color: Color32, opacity: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), opacity)
}

#[inline(always)]
pub fn mix(from: Color32, to: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let blend_channel =
        |start: u8, end: u8| (start as f32 + (end as f32 - start as f32) * amount).round() as u8;
    Color32::from_rgb(
        blend_channel(from.r(), to.r()),
        blend_channel(from.g(), to.g()),
        blend_channel(from.b(), to.b()),
    )
}

pub fn accent_for(params: Option<&NodeParams>) -> Color32 {
    match params {
        Some(NodeParams::Env { .. } | NodeParams::Lfo { .. }) => MOD,
        Some(NodeParams::Noise { .. }) => GATE,
        Some(NodeParams::Fm { .. } | NodeParams::Pm { .. }) => CV,
        None => TEXT_FAINT,
        _ => super::widgets::kind_color(params),
    }
}

fn arc_units() -> &'static [Vec2; ARC_SEG + 1] {
    static UNITS: OnceLock<[Vec2; ARC_SEG + 1]> = OnceLock::new();
    UNITS.get_or_init(|| {
        std::array::from_fn(|index| {
            let angle = (-135.0 + 270.0 * index as f32 / ARC_SEG as f32) * DEG;
            vec2(angle.sin(), -angle.cos())
        })
    })
}

fn disc_units() -> &'static [Vec2; DISC_SEG] {
    static UNITS: OnceLock<[Vec2; DISC_SEG]> = OnceLock::new();
    UNITS.get_or_init(|| {
        std::array::from_fn(|index| {
            let angle = std::f32::consts::TAU * index as f32 / DISC_SEG as f32;
            vec2(angle.cos(), angle.sin())
        })
    })
}

#[inline]
pub fn knob_dir(normalized_position: f32) -> Vec2 {
    let angle = (-135.0 + 270.0 * normalized_position.clamp(0.0, 1.0)) * DEG;
    vec2(angle.sin(), -angle.cos())
}

#[inline]
fn sample_unit(units: &[Vec2], position: f32) -> Vec2 {
    let first_index = (position.floor() as usize).min(units.len() - 1);
    let next_index = (first_index + 1).min(units.len() - 1);
    let fraction = position - first_index as f32;
    units[first_index] + (units[next_index] - units[first_index]) * fraction
}

pub fn arc(center: Pos2, radius: f32, from: f32, to: f32, stroke: Stroke) -> Shape {
    let (from, to) = if to >= from { (from, to) } else { (to, from) };
    if to - from < 1e-4 {
        return Shape::Noop;
    }
    let units = arc_units();
    let start_position = from.clamp(0.0, 1.0) * ARC_SEG as f32;
    let end_position = to.clamp(0.0, 1.0) * ARC_SEG as f32;
    let first_index = start_position.floor() as usize;
    let last_index = end_position.floor() as usize;
    let mut points = Vec::with_capacity(last_index - first_index + 3);
    points.push(center + sample_unit(units, start_position) * radius);
    for unit in &units[first_index + 1..=last_index] {
        points.push(center + *unit * radius);
    }
    points.push(center + sample_unit(units, end_position) * radius);
    Shape::line(points, stroke)
}

fn fan_mesh(
    outline: &[Pos2],
    center: Pos2,
    top: Color32,
    bottom: Color32,
    top_y: f32,
    bottom_y: f32,
) -> Mesh {
    let uv = WHITE_UV;
    let vertical_span = (bottom_y - top_y).max(1e-3);
    let color_at_height = |y: f32| mix(top, bottom, (y - top_y) / vertical_span);
    let mut mesh = Mesh::default();
    mesh.vertices.reserve(outline.len() + 1);
    mesh.indices.reserve(outline.len() * 3);
    mesh.vertices.push(Vertex {
        pos: center,
        uv,
        color: color_at_height(center.y),
    });
    for &point in outline {
        mesh.vertices.push(Vertex {
            pos: point,
            uv,
            color: color_at_height(point.y),
        });
    }
    let outline_length = outline.len() as u32;
    for index in 0..outline_length {
        mesh.indices
            .extend_from_slice(&[0, index + 1, (index + 1) % outline_length + 1]);
    }
    mesh
}

pub fn disc_gradient(center: Pos2, radius: f32, top: Color32, bottom: Color32) -> Shape {
    let units = disc_units();
    let mut outline = [Pos2::ZERO; DISC_SEG];
    for (index, unit) in units.iter().enumerate() {
        outline[index] = center + *unit * radius;
    }
    Shape::mesh(fan_mesh(
        &outline,
        center,
        top,
        bottom,
        center.y - radius,
        center.y + radius,
    ))
}

pub fn rounded_gradient(rect: Rect, radius: f32, top: Color32, bottom: Color32) -> Shape {
    let radius = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let corners = [
        (
            pos2(rect.right() - radius, rect.top() + radius),
            -90.0f32,
            0.0f32,
        ),
        (
            pos2(rect.right() - radius, rect.bottom() - radius),
            0.0,
            90.0,
        ),
        (
            pos2(rect.left() + radius, rect.bottom() - radius),
            90.0,
            180.0,
        ),
        (
            pos2(rect.left() + radius, rect.top() + radius),
            180.0,
            270.0,
        ),
    ];
    let mut outline = Vec::with_capacity(24);
    for (corner_center, angle_start, angle_end) in corners {
        for step in 0..6 {
            let angle = (angle_start + (angle_end - angle_start) * step as f32 / 5.0) * DEG;
            outline.push(corner_center + vec2(angle.cos(), angle.sin()) * radius);
        }
    }
    Shape::mesh(fan_mesh(
        &outline,
        rect.center(),
        top,
        bottom,
        rect.top(),
        rect.bottom(),
    ))
}

pub fn card(rect: Rect, accent: Color32, selected: bool, hot: bool) -> Vec<Shape> {
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), HEADER_H));
    let edge = if selected {
        accent
    } else if hot {
        NODE_EDGE.gamma_multiply(1.35)
    } else {
        NODE_EDGE
    };
    vec![
        Shape::rect_filled(
            rect.translate(vec2(0.0, 4.0)).expand(1.5),
            10.0,
            Color32::from_black_alpha(96),
        ),
        rounded_gradient(rect, 8.0, NODE_TOP, NODE_BOTTOM),
        Shape::rect_filled(
            header,
            egui::CornerRadius {
                nw: 8,
                ne: 8,
                sw: 0,
                se: 0,
            },
            alpha(accent, 20),
        ),
        Shape::line_segment(
            [
                pos2(rect.left() + 9.0, header.bottom()),
                pos2(rect.right() - 9.0, header.bottom()),
            ],
            Stroke::new(1.0, alpha(accent, 58)),
        ),
        Shape::rect_stroke(
            rect,
            8.0,
            Stroke::new(if selected { 1.6 } else { 1.0 }, edge),
            egui::StrokeKind::Inside,
        ),
    ]
}

pub fn badge(painter: &egui::Painter, rect: Rect, fill: Color32, text: &str, text_color: Color32) {
    painter.rect_filled(rect, 4.0, fill);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(10.0),
        text_color,
    );
}
pub fn badge_arrow(painter: &egui::Painter, rect: Rect, fill: Color32, color: Color32) {
    painter.rect_filled(rect, 4.0, fill);
    let center = rect.center();
    let size = rect.width().min(rect.height()) * 0.30;
    let tip = pos2(center.x + size, center.y);
    let stroke = Stroke::new(1.4, color);
    painter.line_segment([pos2(center.x - size, center.y), tip], stroke);
    painter.line_segment(
        [tip, pos2(tip.x - size * 0.62, tip.y - size * 0.62)],
        stroke,
    );
    painter.line_segment(
        [tip, pos2(tip.x - size * 0.62, tip.y + size * 0.62)],
        stroke,
    );
}
pub fn port(painter: &egui::Painter, position: Pos2, color: Color32, hot: bool, output: bool) {
    let radius = if output { 5.0 } else { 4.5 };
    if hot {
        painter.circle_filled(position, radius + 5.0, alpha(color, 44));
    }
    painter.circle_filled(position, radius + 1.0, Color32::from_black_alpha(150));
    painter.circle_filled(position, radius, color);
    painter.circle_filled(position, radius * 0.42, Color32::from_rgb(11, 11, 13));
}

pub fn cable(
    painter: &egui::Painter,
    from: Pos2,
    to: Pos2,
    handle: f32,
    width: f32,
    color: Color32,
) {
    let pts = [from, from + vec2(handle, 0.0), to - vec2(handle, 0.0), to];
    painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
        pts,
        false,
        Color32::TRANSPARENT,
        Stroke::new(width + 4.0, alpha(color, 30)),
    ));
    painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
        pts,
        false,
        Color32::TRANSPARENT,
        Stroke::new(width, color),
    ));
}

pub fn dashed(
    painter: &egui::Painter,
    start: Pos2,
    end: Pos2,
    dash: f32,
    gap: f32,
    stroke: Stroke,
) {
    let delta = end - start;
    let distance = delta.length();
    if distance < 1e-3 {
        return;
    }
    let direction = delta / distance;
    let mut progress = 0.0;
    while progress < distance {
        let segment_end = (progress + dash).min(distance);
        painter.line_segment(
            [
                start + direction * progress,
                start + direction * segment_end,
            ],
            stroke,
        );
        progress = segment_end + gap;
    }
}

pub fn fill_under(points: &[Pos2], baseline: f32, color: Color32) -> Shape {
    if points.len() < 2 {
        return Shape::Noop;
    }
    let uv = WHITE_UV;
    let mut mesh = Mesh::default();
    mesh.vertices.reserve(points.len() * 2);
    mesh.indices.reserve((points.len() - 1) * 6);
    for (index, &point) in points.iter().enumerate() {
        mesh.vertices.push(Vertex {
            pos: point,
            uv,
            color,
        });
        mesh.vertices.push(Vertex {
            pos: pos2(point.x, baseline),
            uv,
            color: alpha(color, 4),
        });
        if index > 0 {
            let pair_index = (index * 2) as u32;
            mesh.indices.extend_from_slice(&[
                pair_index - 2,
                pair_index - 1,
                pair_index,
                pair_index,
                pair_index - 1,
                pair_index + 1,
            ]);
        }
    }
    Shape::mesh(mesh)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Saw,
    Noise,
    Env,
    Lfo,
    Fm,
    Pm,
    Vector,
    Generic,
}

pub fn glyph_for(params: Option<&NodeParams>) -> Glyph {
    match params {
        Some(NodeParams::Analog { .. }) => Glyph::Saw,
        Some(NodeParams::Noise { .. }) => Glyph::Noise,
        Some(NodeParams::Lfo { .. }) => Glyph::Lfo,
        Some(NodeParams::Fm { .. }) => Glyph::Fm,
        Some(NodeParams::Pm { .. }) => Glyph::Pm,
        Some(NodeParams::Vector { .. }) => Glyph::Vector,
        Some(NodeParams::Env { .. }) => Glyph::Env,
        _ => Glyph::Generic,
    }
}

pub fn draw_glyph(painter: &egui::Painter, rect: Rect, glyph: Glyph, color: Color32, width: f32) {
    let inner_rect = rect.shrink2(vec2(rect.width() * 0.12, rect.height() * 0.22));
    let point_at = |normalized_x: f32, normalized_y: f32| {
        pos2(
            inner_rect.left() + inner_rect.width() * normalized_x,
            inner_rect.top() + inner_rect.height() * normalized_y,
        )
    };
    let stroke = Stroke::new(width, color);
    let wave = |curve: &dyn Fn(f32) -> f32, sample_count: usize| -> Vec<Pos2> {
        (0..sample_count)
            .map(|sample_index| {
                let normalized_time = sample_index as f32 / (sample_count - 1) as f32;
                point_at(normalized_time, 0.5 - curve(normalized_time) * 0.5)
            })
            .collect()
    };
    let shape = match glyph {
        Glyph::Lfo => Shape::line(
            wave(&|t| (t * std::f32::consts::TAU).sin() * 0.85, 26),
            stroke,
        ),
        Glyph::Saw => Shape::line(
            vec![
                point_at(0.0, 1.0),
                point_at(0.5, 0.0),
                point_at(0.5, 1.0),
                point_at(1.0, 0.0),
            ],
            stroke,
        ),
        Glyph::Noise => Shape::line(
            vec![
                point_at(0.0, 0.7),
                point_at(0.1, 0.2),
                point_at(0.2, 0.85),
                point_at(0.32, 0.35),
                point_at(0.44, 0.95),
                point_at(0.56, 0.15),
                point_at(0.68, 0.6),
                point_at(0.8, 0.25),
                point_at(0.9, 0.8),
                point_at(1.0, 0.45),
            ],
            stroke,
        ),
        Glyph::Env => Shape::line(
            vec![
                point_at(0.0, 1.0),
                point_at(0.22, 0.0),
                point_at(0.45, 0.55),
                point_at(0.75, 0.55),
                point_at(1.0, 1.0),
            ],
            stroke,
        ),
        Glyph::Fm => Shape::line(
            wave(&|t| (t * std::f32::consts::TAU * (1.0 + 2.2 * t)).sin(), 40),
            stroke,
        ),
        Glyph::Pm => Shape::line(
            wave(
                &|t| {
                    ((t + 0.18 * (t * std::f32::consts::TAU * 3.0).sin()) * std::f32::consts::TAU)
                        .sin()
                },
                40,
            ),
            stroke,
        ),
        Glyph::Vector => Shape::Vec(vec![
            Shape::rect_stroke(
                inner_rect,
                2.0,
                Stroke::new(width, alpha(color, 120)),
                egui::StrokeKind::Inside,
            ),
            Shape::line_segment(
                [point_at(0.5, 0.0), point_at(0.5, 1.0)],
                Stroke::new(1.0, alpha(color, 70)),
            ),
            Shape::line_segment(
                [point_at(0.0, 0.5), point_at(1.0, 0.5)],
                Stroke::new(1.0, alpha(color, 70)),
            ),
            Shape::circle_filled(point_at(0.68, 0.32), 2.4, color),
        ]),
        Glyph::Generic => Shape::Vec(vec![
            Shape::circle_stroke(inner_rect.center(), inner_rect.height() * 0.42, stroke),
            Shape::circle_filled(inner_rect.center(), 1.8, color),
        ]),
    };
    painter.add(shape);
}
