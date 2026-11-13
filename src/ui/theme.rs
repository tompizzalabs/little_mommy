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
pub const PHASE: Color32 = Color32::from_rgb(176, 108, 255);
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
pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

#[inline(always)]
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub fn accent_for(params: Option<&NodeParams>) -> Color32 {
    match params {
        Some(NodeParams::Env { .. } | NodeParams::Lfo { .. }) => MOD,
        Some(NodeParams::Noise { .. }) => GATE,
        Some(NodeParams::Vector { .. } | NodeParams::Multi { .. }) => PHASE,
        Some(NodeParams::Fm { .. } | NodeParams::Pm { .. }) => CV,
        None => TEXT_FAINT,
        _ => super::widgets::kind_color(params),
    }
}

fn arc_units() -> &'static [Vec2; ARC_SEG + 1] {
    static U: OnceLock<[Vec2; ARC_SEG + 1]> = OnceLock::new();
    U.get_or_init(|| {
        std::array::from_fn(|i| {
            let a = (-135.0 + 270.0 * i as f32 / ARC_SEG as f32) * DEG;
            vec2(a.sin(), -a.cos())
        })
    })
}

fn disc_units() -> &'static [Vec2; DISC_SEG] {
    static U: OnceLock<[Vec2; DISC_SEG]> = OnceLock::new();
    U.get_or_init(|| {
        std::array::from_fn(|i| {
            let a = std::f32::consts::TAU * i as f32 / DISC_SEG as f32;
            vec2(a.cos(), a.sin())
        })
    })
}

#[inline]
pub fn knob_dir(t: f32) -> Vec2 {
    let a = (-135.0 + 270.0 * t.clamp(0.0, 1.0)) * DEG;
    vec2(a.sin(), -a.cos())
}

#[inline]
fn sample_unit(units: &[Vec2], x: f32) -> Vec2 {
    let i = (x.floor() as usize).min(units.len() - 1);
    let j = (i + 1).min(units.len() - 1);
    let f = x - i as f32;
    units[i] + (units[j] - units[i]) * f
}

pub fn arc(center: Pos2, r: f32, from: f32, to: f32, stroke: Stroke) -> Shape {
    let (from, to) = if to >= from { (from, to) } else { (to, from) };
    if to - from < 1e-4 {
        return Shape::Noop;
    }
    let units = arc_units();
    let a = from.clamp(0.0, 1.0) * ARC_SEG as f32;
    let b = to.clamp(0.0, 1.0) * ARC_SEG as f32;
    let i0 = a.floor() as usize;
    let i1 = b.floor() as usize;
    let mut pts = Vec::with_capacity(i1 - i0 + 3);
    pts.push(center + sample_unit(units, a) * r);
    for i in (i0 + 1)..=i1 {
        pts.push(center + units[i] * r);
    }
    pts.push(center + sample_unit(units, b) * r);
    Shape::line(pts, stroke)
}

fn fan_mesh(
    outline: &[Pos2],
    center: Pos2,
    top: Color32,
    bottom: Color32,
    y0: f32,
    y1: f32,
) -> Mesh {
    let uv = WHITE_UV;
    let span = (y1 - y0).max(1e-3);
    let col = |y: f32| mix(top, bottom, (y - y0) / span);
    let mut mesh = Mesh::default();
    mesh.vertices.reserve(outline.len() + 1);
    mesh.indices.reserve(outline.len() * 3);
    mesh.vertices.push(Vertex {
        pos: center,
        uv,
        color: col(center.y),
    });
    for &p in outline {
        mesh.vertices.push(Vertex {
            pos: p,
            uv,
            color: col(p.y),
        });
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.indices.extend_from_slice(&[0, i + 1, (i + 1) % n + 1]);
    }
    mesh
}

pub fn disc_gradient(center: Pos2, r: f32, top: Color32, bottom: Color32) -> Shape {
    let units = disc_units();
    let mut outline = [Pos2::ZERO; DISC_SEG];
    for i in 0..DISC_SEG {
        outline[i] = center + units[i] * r;
    }
    Shape::mesh(fan_mesh(
        &outline,
        center,
        top,
        bottom,
        center.y - r,
        center.y + r,
    ))
}

pub fn rounded_gradient(rect: Rect, radius: f32, top: Color32, bottom: Color32) -> Shape {
    let r = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let corners = [
        (pos2(rect.right() - r, rect.top() + r), -90.0f32, 0.0f32),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0, 90.0),
        (pos2(rect.left() + r, rect.bottom() - r), 90.0, 180.0),
        (pos2(rect.left() + r, rect.top() + r), 180.0, 270.0),
    ];
    let mut outline = Vec::with_capacity(24);
    for (c, a0, a1) in corners {
        for i in 0..6 {
            let a = (a0 + (a1 - a0) * i as f32 / 5.0) * DEG;
            outline.push(c + vec2(a.cos(), a.sin()) * r);
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

pub fn port(painter: &egui::Painter, pos: Pos2, color: Color32, hot: bool, output: bool) {
    let r = if output { 5.0 } else { 4.5 };
    if hot {
        painter.circle_filled(pos, r + 5.0, alpha(color, 44));
    }
    painter.circle_filled(pos, r + 1.0, Color32::from_black_alpha(150));
    painter.circle_filled(pos, r, color);
    painter.circle_filled(pos, r * 0.42, Color32::from_rgb(11, 11, 13));
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

pub fn dashed(painter: &egui::Painter, a: Pos2, b: Pos2, dash: f32, gap: f32, stroke: Stroke) {
    let d = b - a;
    let len = d.length();
    if len < 1e-3 {
        return;
    }
    let dir = d / len;
    let mut t = 0.0;
    while t < len {
        let e = (t + dash).min(len);
        painter.line_segment([a + dir * t, a + dir * e], stroke);
        t = e + gap;
    }
}

pub fn fill_under(pts: &[Pos2], baseline: f32, color: Color32) -> Shape {
    if pts.len() < 2 {
        return Shape::Noop;
    }
    let uv = WHITE_UV;
    let mut mesh = Mesh::default();
    mesh.vertices.reserve(pts.len() * 2);
    mesh.indices.reserve((pts.len() - 1) * 6);
    for (i, &p) in pts.iter().enumerate() {
        mesh.vertices.push(Vertex { pos: p, uv, color });
        mesh.vertices.push(Vertex {
            pos: pos2(p.x, baseline),
            uv,
            color: alpha(color, 4),
        });
        if i > 0 {
            let b = (i * 2) as u32;
            mesh.indices
                .extend_from_slice(&[b - 2, b - 1, b, b, b - 1, b + 1]);
        }
    }
    Shape::mesh(mesh)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Sine,
    Tri,
    Saw,
    Square,
    Noise,
    Env,
    Lfo,
    Vector,
    Multi,
    Fm,
    Pm,
    Chip,
    Table,
    Generic,
}

pub fn glyph_for(params: Option<&NodeParams>) -> Glyph {
    match params {
        Some(NodeParams::Analog { .. }) => Glyph::Saw,
        Some(NodeParams::Noise { .. }) => Glyph::Noise,
        Some(NodeParams::Wave { .. }) => Glyph::Table,
        Some(NodeParams::Lfo { .. }) => Glyph::Lfo,
        Some(NodeParams::Chiptune { .. }) => Glyph::Chip,
        Some(NodeParams::Multi { .. }) => Glyph::Multi,
        Some(NodeParams::Vector { .. }) => Glyph::Vector,
        Some(NodeParams::Fm { .. }) => Glyph::Fm,
        Some(NodeParams::Pm { .. }) => Glyph::Pm,
        Some(NodeParams::Env { .. }) => Glyph::Env,
        _ => Glyph::Generic,
    }
}

pub fn draw_glyph(painter: &egui::Painter, rect: Rect, glyph: Glyph, color: Color32, width: f32) {
    let r = rect.shrink2(vec2(rect.width() * 0.12, rect.height() * 0.22));
    let p = |x: f32, y: f32| pos2(r.left() + r.width() * x, r.top() + r.height() * y);
    let stroke = Stroke::new(width, color);
    let wave = |f: &dyn Fn(f32) -> f32, n: usize| -> Vec<Pos2> {
        (0..n)
            .map(|i| {
                let t = i as f32 / (n - 1) as f32;
                p(t, 0.5 - f(t) * 0.5)
            })
            .collect()
    };
    let shape = match glyph {
        Glyph::Sine => Shape::line(wave(&|t| (t * std::f32::consts::TAU).sin(), 26), stroke),
        Glyph::Lfo => Shape::line(
            wave(&|t| (t * std::f32::consts::TAU).sin() * 0.85, 26),
            stroke,
        ),
        Glyph::Tri => Shape::line(
            vec![p(0.0, 0.5), p(0.25, 0.0), p(0.75, 1.0), p(1.0, 0.5)],
            stroke,
        ),
        Glyph::Saw => Shape::line(
            vec![p(0.0, 1.0), p(0.5, 0.0), p(0.5, 1.0), p(1.0, 0.0)],
            stroke,
        ),
        Glyph::Square => Shape::line(
            vec![
                p(0.0, 1.0),
                p(0.0, 0.0),
                p(0.5, 0.0),
                p(0.5, 1.0),
                p(1.0, 1.0),
                p(1.0, 0.0),
            ],
            stroke,
        ),
        Glyph::Chip => Shape::line(
            vec![
                p(0.0, 1.0),
                p(0.0, 0.1),
                p(0.25, 0.1),
                p(0.25, 0.9),
                p(0.5, 0.9),
                p(0.5, 0.2),
                p(0.75, 0.2),
                p(0.75, 0.8),
                p(1.0, 0.8),
            ],
            stroke,
        ),
        Glyph::Noise => Shape::line(
            vec![
                p(0.0, 0.7),
                p(0.1, 0.2),
                p(0.2, 0.85),
                p(0.32, 0.35),
                p(0.44, 0.95),
                p(0.56, 0.15),
                p(0.68, 0.6),
                p(0.8, 0.25),
                p(0.9, 0.8),
                p(1.0, 0.45),
            ],
            stroke,
        ),
        Glyph::Env => Shape::line(
            vec![
                p(0.0, 1.0),
                p(0.22, 0.0),
                p(0.45, 0.55),
                p(0.75, 0.55),
                p(1.0, 1.0),
            ],
            stroke,
        ),
        Glyph::Multi => {
            let mut v = Vec::with_capacity(3);
            for k in 0..3 {
                let off = (k as f32 - 1.0) * 0.22;
                v.push(Shape::line(
                    wave(
                        &|t| ((t + off * 0.12) * std::f32::consts::TAU).sin() * 0.6 + off * 0.5,
                        20,
                    ),
                    Stroke::new(width, alpha(color, if k == 1 { 255 } else { 110 })),
                ));
            }
            Shape::Vec(v)
        }
        Glyph::Table => {
            let mut v = Vec::with_capacity(3);
            for k in 0..3 {
                let s = 0.35 + k as f32 * 0.28;
                v.push(Shape::line(
                    wave(&|t| (t * std::f32::consts::TAU).sin() * s, 20),
                    Stroke::new(width, alpha(color, 90 + k as u8 * 70)),
                ));
            }
            Shape::Vec(v)
        }
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
                r,
                2.0,
                Stroke::new(width, alpha(color, 120)),
                egui::StrokeKind::Inside,
            ),
            Shape::line_segment(
                [p(0.5, 0.0), p(0.5, 1.0)],
                Stroke::new(1.0, alpha(color, 70)),
            ),
            Shape::line_segment(
                [p(0.0, 0.5), p(1.0, 0.5)],
                Stroke::new(1.0, alpha(color, 70)),
            ),
            Shape::circle_filled(p(0.68, 0.32), 2.4, color),
        ]),
        Glyph::Generic => Shape::Vec(vec![
            Shape::circle_stroke(r.center(), r.height() * 0.42, stroke),
            Shape::circle_filled(r.center(), 1.8, color),
        ]),
    };
    painter.add(shape);
}
