use super::knob::{self, Knob};
use super::preview;
use super::shape;
use super::state::{DragState, MAX_MOD_PARAMS, NodeGeom, ensure_scratch};
use super::theme::{self as th, BODY_W, Glyph, HEADER_H, NODE_W, PAD};
use super::widgets::{KIND_LIST, kind_label};
use crate::engine::graph::{InputSel, ModLink, Patch};
use crate::engine::lfo::LfoShape;
use crate::engine::node::{NodeKind, NodeParams, ParamDef};
use crate::engine::osc::{AnalogShape, NoiseColor, OperatorWave, unison};
use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, pos2, vec2};
use std::sync::OnceLock;
const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 2.5;
const GRID_MIN_PX: f32 = 26.0;
const GRID_MAX_AXIS: i32 = 160;
const CULL_PAD: egui::Vec2 = vec2(40.0, 140.0);
const HIT_MOD_SQ: f32 = 64.0;
const HIT_PORT_SQ: f32 = 121.0;

const VOICE_OPTIONS: [(u8, &str); 5] = [(1, "1"), (2, "2"), (4, "4"), (8, "8"), (16, "16")];

const STACK_OPTIONS: [(unison::StackMode, &str); 4] = [
    (unison::StackMode::Unison, "UNI"),
    (unison::StackMode::Octaves, "OCT"),
    (unison::StackMode::Fifths, "5TH"),
    (unison::StackMode::OctavesAndFifths, "OCT+5"),
];

const PAN_MODE_OPTIONS: [(unison::PanMode, &str); 5] = [
    (unison::PanMode::Linear, "LIN"),
    (unison::PanMode::Center, "CEN"),
    (unison::PanMode::Outer, "OUT"),
    (unison::PanMode::Alternating, "ALT"),
    (unison::PanMode::Random, "RND"),
];

const DETUNE_MODE_OPTIONS: [(unison::DetuneMode, &str); 3] = [
    (unison::DetuneMode::Power, "CURVE"),
    (unison::DetuneMode::Random, "RAND"),
    (unison::DetuneMode::Analog, "ANLG"),
];

const OPERATOR_WAVE_OPTIONS: [(OperatorWave, &str); 3] = [
    (OperatorWave::Saw, "SAW"),
    (OperatorWave::Triangle, "TRI"),
    (OperatorWave::Sine, "SIN"),
];

struct PaletteEntry {
    kind: NodeKind,
    accent: Color32,
    glyph: Glyph,
    label: String,
}

fn palette_entries() -> &'static [PaletteEntry] {
    static ENTRIES: OnceLock<Vec<PaletteEntry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        KIND_LIST
            .iter()
            .map(|&kind| {
                let default_params = NodeParams::default_for(kind);
                PaletteEntry {
                    kind,
                    accent: th::accent_for(Some(&default_params)),
                    glyph: th::glyph_for(Some(&default_params)),
                    label: kind_label(Some(kind)).to_string(),
                }
            })
            .collect()
    })
}

fn add_node(
    patch: &mut Patch,
    positions: &mut Vec<Pos2>,
    geom: &mut Vec<NodeGeom>,
    selected: &mut Option<usize>,
    kind: NodeKind,
    at: Pos2,
) {
    let slot = patch.grid.insert(
        NodeParams::default_for(kind),
        InputSel::None,
        InputSel::None,
    );
    if positions.len() <= slot {
        positions.resize(slot + 1, Pos2::ZERO);
    }
    if geom.len() <= slot {
        geom.resize(slot + 1, NodeGeom::default());
    }
    positions[slot] = at;
    geom[slot] = NodeGeom::default();
    *selected = Some(slot);
    patch.version = patch.version.wrapping_add(1);
}

#[allow(clippy::too_many_arguments)]
pub fn canvas(
    ui: &mut egui::Ui,
    patch: &mut Patch,
    positions: &mut Vec<Pos2>,
    selected: &mut Option<usize>,
    drag: &mut DragState,
    scene_rect: &mut egui::Rect,
    geom: &mut Vec<NodeGeom>,
    context_menu_pos: &mut Option<Pos2>,
    context_menu_world: &mut Option<Pos2>,
) {
    ensure_scratch(patch, positions, geom);
    let cell_count = patch.grid.cells.len();

    let kinds: Vec<Option<NodeKind>> = patch
        .grid
        .cells
        .iter()
        .map(|cell| cell.params.as_ref().map(|params| params.kind()))
        .collect();

    ui.painter()
        .rect_filled(ui.available_rect_before_wrap(), 0.0, th::CANVAS_BG);

    egui::Scene::new()
        .zoom_range(egui::Rangef::new(ZOOM_MIN, ZOOM_MAX))
        .scroll_zooms(true)
        .drag_pan_buttons(egui::DragPanButtons::MIDDLE)
        .max_inner_size(vec2(1_000_000.0, 1_000_000.0))
        .show(ui, scene_rect, |ui| {
            let to_scene = ui
                .ctx()
                .layer_transform_from_global(ui.layer_id())
                .unwrap_or(egui::emath::TSTransform::IDENTITY);
            let zoom = 1.0 / to_scene.scaling.max(1e-6);
            let clip = ui.clip_rect();

            let (hover_global, primary_pressed, primary_released, secondary_pressed, delete_key) =
                ui.input(|input| {
                    (
                        input.pointer.hover_pos(),
                        input.pointer.primary_pressed(),
                        input.pointer.primary_released(),
                        input.pointer.secondary_pressed(),
                        input.key_pressed(egui::Key::Delete)
                            || input.key_pressed(egui::Key::Backspace),
                    )
                });
            let hover_position = hover_global.map(|position| to_scene * position);
            let hover_inside = hover_position.is_some_and(|position| clip.contains(position));

            draw_grid(ui, zoom, clip);
            if patch.grid.cells.iter().all(|cell| cell.params.is_none()) {
                let canvas_center = clip.center();
                let painter = ui.painter();
                painter.text(
                    canvas_center - vec2(0.0, 11.0 / zoom),
                    egui::Align2::CENTER_CENTER,
                    "RIGHT-CLICK TO ADD A MODULE",
                    egui::FontId::proportional(13.0 / zoom),
                    th::TEXT_FAINT,
                );
                painter.text(
                    canvas_center + vec2(0.0, 11.0 / zoom),
                    egui::Align2::CENTER_CENTER,
                    "middle-drag to pan · wheel to zoom",
                    egui::FontId::proportional(10.0 / zoom),
                    th::alpha(th::TEXT_FAINT, 150),
                );
            }
            if delete_key && let Some(slot) = *selected {
                let cell = &mut patch.grid.cells[slot];
                cell.params = None;
                cell.in0 = InputSel::None;
                cell.in1 = InputSel::None;
                cell.mods.clear();
                for other_cell in patch.grid.cells.iter_mut() {
                    if other_cell.in0 == InputSel::Slot(slot as u32) {
                        other_cell.in0 = InputSel::None;
                    }
                    if other_cell.in1 == InputSel::Slot(slot as u32) {
                        other_cell.in1 = InputSel::None;
                    }
                    other_cell.mods.retain(|link| link.source as usize != slot);
                }
                geom[slot].valid = false;
                patch.version = patch.version.wrapping_add(1);
                *selected = None;
            }

            if secondary_pressed
                && hover_inside
                && let Some(position) = hover_global
            {
                *context_menu_pos = Some(position);
                *context_menu_world = Some(to_scene * position);
            }

            for slot in 0..cell_count {
                let cell = &mut patch.grid.cells[slot];
                if cell.params.is_none() {
                    geom[slot].valid = false;
                    continue;
                }

                let world_pos = positions[slot];
                let cached = geom[slot];
                let estimated_rect = if cached.valid {
                    cached.translated(world_pos - cached.rect.min).rect
                } else {
                    Rect::from_min_size(world_pos, vec2(NODE_W, 210.0))
                };

                if !estimated_rect.expand2(CULL_PAD).intersects(clip) {
                    geom[slot] = if cached.valid {
                        cached.translated(world_pos - cached.rect.min)
                    } else {
                        NodeGeom {
                            rect: estimated_rect,
                            ..NodeGeom::default()
                        }
                    };
                    continue;
                }

                let is_selected = *selected == Some(slot);
                let accent = th::accent_for(cell.params.as_ref());
                let title = kind_label(kinds[slot]).to_string();
                let mut dirty = false;
                geom[slot].mod_input_count = 0;

                let card = ui
                    .scope_builder(
                        egui::UiBuilder::new()
                            .id_salt(("node", slot))
                            .max_rect(Rect::from_min_size(world_pos, vec2(NODE_W, f32::INFINITY))),
                        |ui| {
                            ui.set_width(NODE_W);
                            let background_shape = ui.painter().add(Shape::Noop);
                            let header =
                                ui.allocate_response(vec2(NODE_W, HEADER_H), Sense::drag());
                            if header.dragged() {
                                positions[slot] += header.drag_delta();
                            }
                            if header.clicked() {
                                *selected = Some(slot);
                            }

                            ui.add_space(7.0);
                            let body = ui
                                .horizontal(|ui| {
                                    ui.add_space(PAD);
                                    ui.vertical(|ui| {
                                        ui.set_width(BODY_W);
                                        ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                                        if let Some(params) = cell.params.as_mut() {
                                            kind_controls(ui, params, accent, slot, &mut dirty);
                                        }
                                        let defs: &'static [ParamDef] = cell
                                            .params
                                            .as_ref()
                                            .map(|params| params.param_defs())
                                            .unwrap_or(&[]);
                                        if !defs.is_empty() {
                                            ui.add_space(2.0);
                                            param_grid(
                                                ui, cell, defs, accent, slot, geom, &mut dirty,
                                            );
                                        }
                                    });
                                    ui.add_space(PAD);
                                })
                                .response
                                .rect;

                            let card = Rect::from_min_max(
                                world_pos,
                                pos2(world_pos.x + NODE_W, body.bottom() + 9.0),
                            );
                            ui.painter().set(
                                background_shape,
                                Shape::Vec(th::card(card, accent, is_selected, header.hovered())),
                            );

                            let header_painter = ui.painter();

                            let badge_left =
                                Rect::from_min_size(card.min + vec2(8.0, 7.0), vec2(15.0, 14.0));
                            th::badge(
                                header_painter,
                                badge_left,
                                th::alpha(accent, 60),
                                &title.chars().next().unwrap_or('N').to_string(),
                                accent,
                            );

                            header_painter.text(
                                pos2(card.center().x, card.top() + HEADER_H * 0.5),
                                egui::Align2::CENTER_CENTER,
                                &title,
                                egui::FontId::proportional(12.0),
                                accent,
                            );

                            let badge_right = Rect::from_min_size(
                                pos2(card.right() - 23.0, card.top() + 7.0),
                                vec2(15.0, 14.0),
                            );
                            th::badge_arrow(
                                header_painter,
                                badge_right,
                                th::alpha(th::CV, 52),
                                th::CV,
                            );

                            ui.advance_cursor_after_rect(card);
                            card
                        },
                    )
                    .inner;

                let geometry = &mut geom[slot];
                geometry.rect = card;
                geometry.valid = true;
                if dirty {
                    patch.version = patch.version.wrapping_add(1);
                }
            }

            let mut hovered_mod_in: Option<(usize, usize)> = None;
            let mut hovered_mod_out: Option<usize> = None;
            let mut hovered_port: Option<(usize, usize, bool)> = None;
            let mut hovered_node: Option<usize> = None;

            if let Some(hover_position) = hover_position {
                'mods: for (slot, geometry) in geom.iter().enumerate().take(cell_count) {
                    if !geometry.valid {
                        continue;
                    }
                    for (param, mod_input_position) in geometry.mod_inputs
                        [..geometry.mod_input_count]
                        .iter()
                        .enumerate()
                    {
                        if mod_input_position.distance_sq(hover_position) < HIT_MOD_SQ {
                            hovered_mod_in = Some((slot, param));
                            break 'mods;
                        }
                    }
                    if geometry.mod_out().distance_sq(hover_position) < HIT_MOD_SQ {
                        hovered_mod_out = Some(slot);
                        break 'mods;
                    }
                }

                if hovered_mod_in.is_none() && hovered_mod_out.is_none() {
                    'ports: for slot in 0..cell_count {
                        let geometry = geom[slot];
                        if !geometry.valid || patch.grid.cells[slot].params.is_none() {
                            continue;
                        }
                        let inputs = patch.grid.cells[slot]
                            .params
                            .as_ref()
                            .unwrap()
                            .input_count();
                        for port in 0..inputs {
                            if geometry.audio_in(port).distance_sq(hover_position) < HIT_PORT_SQ {
                                hovered_port = Some((slot, port, true));
                                break 'ports;
                            }
                        }
                        if geometry.audio_out().distance_sq(hover_position) < HIT_PORT_SQ {
                            hovered_port = Some((slot, 0, false));
                            break 'ports;
                        }
                    }
                    if hovered_port.is_none() {
                        for slot in (0..cell_count).rev() {
                            if geom[slot].valid && geom[slot].rect.contains(hover_position) {
                                hovered_node = Some(slot);
                                break;
                            }
                        }
                    }
                }
            }

            if primary_pressed {
                if let Some((slot, param)) = hovered_mod_in {
                    let mods = &mut patch.grid.cells[slot].mods;
                    let link_count_before = mods.len();
                    mods.retain(|link| link.param as usize != param);
                    if mods.len() != link_count_before {
                        patch.version = patch.version.wrapping_add(1);
                    }
                } else if let Some(slot) = hovered_mod_out {
                    *drag = DragState::ModCable {
                        from: slot,
                        current: geom[slot].mod_out(),
                    };
                } else if let Some((slot, port, is_input)) = hovered_port {
                    if is_input {
                        let cell = &mut patch.grid.cells[slot];
                        if port == 0 {
                            cell.in0 = InputSel::None;
                        } else {
                            cell.in1 = InputSel::None;
                        }
                        patch.version = patch.version.wrapping_add(1);
                    } else {
                        *drag = DragState::Cable {
                            from: slot,
                            current: geom[slot].audio_out(),
                        };
                    }
                } else if let Some(slot) = hovered_node {
                    *selected = Some(slot);
                } else if hover_inside {
                    *selected = None;
                }
            }

            if primary_released {
                match *drag {
                    DragState::Cable { from, .. } => {
                        if let Some((slot, port, true)) = hovered_port
                            && slot != from
                        {
                            let cell = &mut patch.grid.cells[slot];
                            if port == 0 {
                                cell.in0 = InputSel::Slot(from as u32);
                            } else {
                                cell.in1 = InputSel::Slot(from as u32);
                            }
                            patch.version = patch.version.wrapping_add(1);
                        }
                    }
                    DragState::ModCable { from, .. } => {
                        if let Some((slot, param)) = hovered_mod_in
                            && slot != from
                        {
                            let mods = &mut patch.grid.cells[slot].mods;
                            mods.retain(|link| link.param as usize != param);
                            mods.push(ModLink {
                                source: from as u32,
                                param: param as u8,
                                depth: 1.0,
                            });
                            patch.version = patch.version.wrapping_add(1);
                        }
                    }
                    DragState::None => {}
                }
                *drag = DragState::None;
            }

            match drag {
                DragState::Cable { current, .. } | DragState::ModCable { current, .. } => {
                    if let Some(position) = hover_position {
                        *current = position;
                    }
                }
                DragState::None => {}
            }

            let painter = ui.painter().clone();

            for (slot, cell) in patch.grid.cells.iter().enumerate() {
                if cell.params.is_none() || !geom[slot].valid {
                    continue;
                }
                let inputs = cell.params.as_ref().unwrap().input_count();
                for (port, selection) in [(0usize, cell.in0), (1usize, cell.in1)] {
                    if port >= inputs {
                        continue;
                    }
                    if let Some(source) = patch.grid.resolve(slot, selection) {
                        let source = source as usize;
                        if !geom[source].valid {
                            continue;
                        }
                        let from = geom[source].audio_out();
                        let to = geom[slot].audio_in(port);
                        if cable_visible(clip, from, to, 52.0) {
                            th::cable(
                                &painter,
                                from,
                                to,
                                52.0,
                                2.2,
                                th::accent_for(patch.grid.cells[source].params.as_ref()),
                            );
                        }
                    }
                }
            }

            for (slot, cell) in patch.grid.cells.iter().enumerate() {
                if !geom[slot].valid {
                    continue;
                }
                for link in cell.mods.iter() {
                    let source = link.source as usize;
                    let param = link.param as usize;
                    if !geom[source].valid || param >= geom[slot].mod_input_count {
                        continue;
                    }
                    let from = geom[source].mod_out();
                    let to = geom[slot].mod_inputs[param];
                    if cable_visible(clip, from, to, 38.0) {
                        th::cable(&painter, from, to, 38.0, 1.8, th::MOD);
                    }
                }
            }

            match *drag {
                DragState::Cable { from, current } if geom[from].valid => {
                    th::cable(
                        &painter,
                        geom[from].audio_out(),
                        current,
                        52.0,
                        2.4,
                        th::GATE,
                    );
                }
                DragState::ModCable { from, current } if geom[from].valid => {
                    th::cable(&painter, geom[from].mod_out(), current, 38.0, 2.0, th::MOD);
                }
                _ => {}
            }

            for slot in 0..cell_count {
                let geometry = geom[slot];
                if !geometry.valid || patch.grid.cells[slot].params.is_none() {
                    continue;
                }
                if !geometry.rect.expand(16.0).intersects(clip) {
                    continue;
                }
                let cell = &patch.grid.cells[slot];
                let inputs = cell.params.as_ref().unwrap().input_count();
                let accent = th::accent_for(cell.params.as_ref());
                for port in 0..inputs {
                    let hot = hovered_port == Some((slot, port, true));
                    th::port(
                        &painter,
                        geometry.audio_in(port),
                        if port == 0 { th::AUDIO } else { th::GATE },
                        hot,
                        false,
                    );
                }
                th::port(
                    &painter,
                    geometry.audio_out(),
                    accent,
                    hovered_port == Some((slot, 0, false)),
                    true,
                );
                th::port(
                    &painter,
                    geometry.mod_out(),
                    th::CV,
                    hovered_mod_out == Some(slot),
                    true,
                );
            }

            if let Some(hover_position) = hover_position {
                let tooltip = if let Some((slot, param)) = hovered_mod_in {
                    Some((
                        format!("Mod in · param {} · click to unlink", param + 1),
                        th::MOD,
                        slot,
                    ))
                } else if let Some(slot) = hovered_mod_out {
                    Some((format!("Mod out · S{}", slot), th::CV, slot))
                } else if let Some((slot, port, is_input)) = hovered_port {
                    Some((
                        if is_input {
                            format!("Audio in {} · S{}", port + 1, slot)
                        } else {
                            format!("Audio out · S{}", slot)
                        },
                        if is_input {
                            th::AUDIO
                        } else {
                            th::accent_for(patch.grid.cells[slot].params.as_ref())
                        },
                        slot,
                    ))
                } else {
                    None
                };
                if let Some((text, color, _)) = tooltip {
                    let font = egui::FontId::proportional(10.0);
                    let galley = painter.layout_no_wrap(text, font, th::TEXT_BRIGHT);
                    let size = galley.size() + vec2(14.0, 8.0);
                    let panel =
                        Rect::from_min_size(hover_position + vec2(14.0, -size.y - 8.0), size);
                    painter.rect_filled(
                        panel,
                        4.0,
                        Color32::from_rgba_unmultiplied(14, 14, 17, 235),
                    );
                    painter.rect_stroke(
                        panel,
                        4.0,
                        Stroke::new(1.0, th::alpha(color, 150)),
                        egui::StrokeKind::Inside,
                    );
                    painter.galley(panel.min + vec2(7.0, 4.0), galley, th::TEXT_BRIGHT);
                }
            }
        });

    if let (Some(screen_pos), Some(world_pos)) = (*context_menu_pos, *context_menu_world) {
        let mut close_menu = false;
        egui::Area::new(egui::Id::unique("node_context_menu"))
            .fixed_pos(screen_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(th::SURFACE_HI)
                    .stroke(Stroke::new(1.0, th::NODE_EDGE))
                    .corner_radius(8.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_width(150.0);
                        ui.spacing_mut().item_spacing = vec2(4.0, 2.0);
                        ui.label(
                            egui::RichText::new("ADD MODULE")
                                .size(9.0)
                                .color(th::TEXT_FAINT),
                        );
                        ui.add_space(3.0);
                        for entry in palette_entries() {
                            let (rect, resp) =
                                ui.allocate_exact_size(vec2(150.0, 22.0), Sense::click());
                            let painter = ui.painter();
                            if resp.hovered() {
                                painter.rect_filled(rect, 4.0, th::alpha(entry.accent, 34));
                            }
                            th::draw_glyph(
                                painter,
                                Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(20.0, 14.0)),
                                entry.glyph,
                                entry.accent,
                                1.3,
                            );
                            painter.text(
                                pos2(rect.left() + 30.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                &entry.label,
                                egui::FontId::proportional(11.0),
                                if resp.hovered() {
                                    th::TEXT_BRIGHT
                                } else {
                                    th::TEXT_DIM
                                },
                            );
                            if resp.clicked() {
                                add_node(patch, positions, geom, selected, entry.kind, world_pos);
                                close_menu = true;
                            }
                        }
                    });
            });
        if close_menu || ui.input(|input| input.pointer.primary_clicked()) {
            *context_menu_pos = None;
            *context_menu_world = None;
        }
    }
}
fn center_detented(name: &str) -> bool {
    let lowercase_name = name.to_ascii_lowercase();
    ["pan", "balance", "skew", "bias", "detune", "blend"]
        .iter()
        .any(|keyword| lowercase_name.contains(keyword))
}
#[allow(clippy::too_many_arguments)]
fn param_grid(
    ui: &mut egui::Ui,
    cell: &mut crate::engine::graph::Cell,
    defs: &'static [ParamDef],
    accent: Color32,
    slot: usize,
    geom: &mut [NodeGeom],
    dirty: &mut bool,
) {
    for chunk in defs.chunks(4) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
            for (chunk_index, definition) in chunk.iter().enumerate() {
                let param_index = defs
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, definition))
                    .unwrap_or(chunk_index);
                let link_index = cell
                    .mods
                    .iter()
                    .position(|link| link.param as usize == param_index);
                let mut value = cell
                    .params
                    .as_ref()
                    .map(|params| params.param_get(param_index))
                    .unwrap_or(0.0);
                let depth = link_index.map(|index| &mut cell.mods[index].depth);
                let default_value = (definition.min + definition.max) * 0.5;
                let output = Knob::new(&mut value, definition.min, definition.max)
                    .accent(accent)
                    .label(definition.name)
                    .default(default_value)
                    .bipolar(center_detented(definition.name))
                    .depth(depth)
                    .ui(ui);
                let geometry = &mut geom[slot];
                if geometry.mod_input_count < MAX_MOD_PARAMS {
                    geometry.mod_inputs[geometry.mod_input_count] = output.mod_pos;
                    geometry.mod_input_count += 1;
                }
                if output.response.changed() {
                    if let Some(params) = cell.params.as_mut() {
                        params.param_set(param_index, value);
                    }
                    *dirty = true;
                }
                output.response.on_hover_text(format!(
                    "{} · drag to adjust · shift = fine · double-click = reset",
                    definition.name
                ));
            }
        });
    }
}

fn kind_controls(
    ui: &mut egui::Ui,
    params: &mut NodeParams,
    accent: Color32,
    slot: usize,
    dirty: &mut bool,
) {
    match params {
        NodeParams::Analog {
            shape,
            pulse_width,
            unison_voices,
            unison_stack_mode,
            unison_pan_mode,
            unison_detune_mode,
            ..
        } => {
            *dirty |= knob::glyph_row(
                ui,
                "analog_shape",
                shape,
                &[
                    (AnalogShape::Sine, Glyph::Sine),
                    (AnalogShape::Triangle, Glyph::Tri),
                    (AnalogShape::Saw, Glyph::Saw),
                    (AnalogShape::Square, Glyph::Square),
                ],
                accent,
            );
            preview::preview_analog(ui, BODY_W, accent, *shape, *pulse_width);
            knob::section_label(ui, "UNISON VOICES");
            *dirty |= knob::segmented(ui, "analog_voices", unison_voices, &VOICE_OPTIONS, accent);
            *dirty |= knob::segmented(
                ui,
                "analog_stack",
                unison_stack_mode,
                &STACK_OPTIONS,
                accent,
            );
            *dirty |= knob::segmented(
                ui,
                "analog_pan_mode",
                unison_pan_mode,
                &PAN_MODE_OPTIONS,
                accent,
            );
            *dirty |= knob::segmented(
                ui,
                "analog_detune_mode",
                unison_detune_mode,
                &DETUNE_MODE_OPTIONS,
                accent,
            );
        }
        NodeParams::Noise { color, .. } => {
            let mut selected_index = NoiseColor::ALL
                .iter()
                .position(|candidate| candidate == color)
                .unwrap_or(0);
            let options: Vec<(usize, String)> = NoiseColor::ALL
                .iter()
                .enumerate()
                .map(|(index, candidate)| (index, candidate.to_string()))
                .collect();
            let option_labels: Vec<(usize, &str)> = options
                .iter()
                .map(|(index, label)| (*index, label.as_str()))
                .collect();
            if knob::segmented(
                ui,
                "noise_color",
                &mut selected_index,
                &option_labels,
                accent,
            ) {
                *color = NoiseColor::ALL[selected_index];
                *dirty = true;
            }
            preview::preview_noise(ui, BODY_W, accent, *color, slot as u32);
        }
        NodeParams::Lfo { shape, .. } => {
            *dirty |= shape::wave_shape_picker(ui, "lfo_shape", shape, &LfoShape::ALL, accent);
            preview::preview_lfo(ui, BODY_W, accent, *shape);
        }
        NodeParams::Fm {
            carrier_wave,
            modulator_wave,
            amount,
            carrier_ratio,
            modulator_ratio,
            exponential,
            ..
        } => {
            knob::section_label(ui, "CARRIER");
            *dirty |= knob::segmented(
                ui,
                "fm_c_wave",
                carrier_wave,
                &OPERATOR_WAVE_OPTIONS,
                accent,
            );
            knob::section_label(ui, "MODULATOR");
            *dirty |= knob::segmented(
                ui,
                "fm_m_wave",
                modulator_wave,
                &OPERATOR_WAVE_OPTIONS,
                accent,
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                *dirty |= knob::chip(ui, "fm_exp", exponential, "EXP FM", accent);
            });
            preview::preview_fm(
                ui,
                BODY_W,
                accent,
                *carrier_wave,
                *modulator_wave,
                *amount,
                *carrier_ratio as f32,
                *modulator_ratio as f32,
                *exponential,
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
                ui.vertical(|ui| {
                    ui.set_width(BODY_W * 0.5 - 3.0);
                    *dirty |= knob::stepper(ui, "fm_cr", carrier_ratio, 1, 16, "C RATIO", accent);
                });
                ui.vertical(|ui| {
                    ui.set_width(BODY_W * 0.5 - 3.0);
                    *dirty |= knob::stepper(ui, "fm_mr", modulator_ratio, 1, 16, "M RATIO", accent);
                });
            });
        }
        NodeParams::Pm {
            carrier_wave,
            modulator_wave,
            amount,
            carrier_ratio,
            modulator_ratio,
            ..
        } => {
            knob::section_label(ui, "CARRIER");
            *dirty |= knob::segmented(
                ui,
                "pm_c_wave",
                carrier_wave,
                &OPERATOR_WAVE_OPTIONS,
                accent,
            );
            knob::section_label(ui, "MODULATOR");
            *dirty |= knob::segmented(
                ui,
                "pm_m_wave",
                modulator_wave,
                &OPERATOR_WAVE_OPTIONS,
                accent,
            );
            preview::preview_pm(
                ui,
                BODY_W,
                accent,
                *carrier_wave,
                *modulator_wave,
                *amount,
                *carrier_ratio as f32,
                *modulator_ratio as f32,
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
                ui.vertical(|ui| {
                    ui.set_width(BODY_W * 0.5 - 3.0);
                    *dirty |= knob::stepper(ui, "pm_cr", carrier_ratio, 1, 16, "C RATIO", accent);
                });
                ui.vertical(|ui| {
                    ui.set_width(BODY_W * 0.5 - 3.0);
                    *dirty |= knob::stepper(ui, "pm_mr", modulator_ratio, 1, 16, "M RATIO", accent);
                });
            });
        }
        NodeParams::Vector { x, y, .. } => {
            preview::preview_vector(ui, BODY_W, accent, *x, *y);
            knob::section_label(ui, "VECTOR PAD · DRAG");
            ui.horizontal(|ui| {
                *dirty |= knob::xy_pad(ui, "vec_pad", x, y, 74.0, accent);
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 4.0);
                    let mut x_value = *x;
                    let mut y_value = *y;
                    let x_knob = Knob::new(&mut x_value, 0.0, 1.0)
                        .accent(accent)
                        .size(28.0)
                        .label("X")
                        .default(0.5)
                        .ui(ui);
                    let y_knob = Knob::new(&mut y_value, 0.0, 1.0)
                        .accent(accent)
                        .size(28.0)
                        .label("Y")
                        .default(0.5)
                        .ui(ui);
                    if x_knob.response.changed() {
                        *x = x_value;
                        *dirty = true;
                    }
                    if y_knob.response.changed() {
                        *y = y_value;
                        *dirty = true;
                    }
                });
            });
        }
        NodeParams::Env {
            attack,
            decay,
            sustain,
            release,
            ..
        } => {
            preview::preview_env(ui, BODY_W, accent, *attack, *decay, *sustain, *release);
        }
        _ => {}
    }
}

fn draw_grid(ui: &egui::Ui, zoom: f32, rect: Rect) {
    let mut step = th::GRID_STEP;
    while step * zoom < GRID_MIN_PX {
        step *= 2.0;
    }
    let grid_start_x = rect.min.x - rect.min.x.rem_euclid(step);
    let grid_start_y = rect.min.y - rect.min.y.rem_euclid(step);
    let column_count = (((rect.max.x - grid_start_x) / step) as i32 + 1).clamp(0, GRID_MAX_AXIS);
    let row_count = (((rect.max.y - grid_start_y) / step) as i32 + 1).clamp(0, GRID_MAX_AXIS);
    let painter = ui.painter();
    let x_range = rect.x_range();
    let y_range = rect.y_range();
    for column in 0..column_count {
        let x = grid_start_x + column as f32 * step;
        let major = (x / step).round() as i64 % 4 == 0;
        painter.vline(
            x,
            y_range,
            Stroke::new(1.0, if major { th::GRID_MAJOR } else { th::GRID_LINE }),
        );
    }
    for row in 0..row_count {
        let y = grid_start_y + row as f32 * step;
        let major = (y / step).round() as i64 % 4 == 0;
        painter.hline(
            x_range,
            y,
            Stroke::new(1.0, if major { th::GRID_MAJOR } else { th::GRID_LINE }),
        );
    }
}

#[inline(always)]
fn cable_visible(clip: Rect, start: Pos2, end: Pos2, handle: f32) -> bool {
    let min = start.min(end) - vec2(handle, 0.0);
    let max = start.max(end) + vec2(handle, 0.0);
    clip.intersects(Rect::from_min_max(min, max))
}
