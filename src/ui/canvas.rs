//! Node graph canvas: infinite zoomable/panable scene, inline node Areas,
//! cables, mod routing, context menu.
//!
//! The whole scene lives inside egui's `Scene` container, which provides an
//! infinite pan/zoom transform. Everything is laid out in *scene* coordinates
//! (world units); `Scene` maps them to the screen and applies the pan/zoom
//! so that widgets, ports and cables all scale together.

use super::preview;
use super::state::{DragState, GRID_CELLS};
use super::widgets::{KIND_LIST, kind_color, kind_label, stock_label, table_label};
use crate::engine::graph::{InputSel, ModLink, Patch};
use crate::engine::lfo::LfoShape;
use crate::engine::node::{NodeKind, NodeParams, ParamDef};
use crate::engine::osc::{AnalogShape, NoiseColor, StockTable, TableSel, wavetable};

const NODE_W: f32 = 180.0;
const HEADER_H: f32 = 24.0;

/// Allowed zoom levels for the scene.
const ZOOM_MIN: f32 = 0.2;
const ZOOM_MAX: f32 = 3.0;
/// World-space distance between background grid dots.
const GRID_STEP: f32 = 24.0;

// UI state is passed through explicitly; splitting into a struct adds indirection without benefit.
#[allow(clippy::too_many_arguments)]
pub fn canvas(
    ui: &mut egui::Ui,
    patch: &mut Patch,
    positions: &mut [egui::Pos2],
    selected: &mut Option<usize>,
    drag: &mut DragState,
    scene_rect: &mut egui::Rect,
    context_menu_pos: &mut Option<egui::Pos2>,
    context_menu_world: &mut Option<egui::Pos2>,
) {
    let lib = wavetable::read_guard();

    // Immutable snapshot of every slot's kind, used inside the node editor
    // closure (which holds `&mut` on individual cells and so cannot also borrow
    // `patch.grid` while laying out mod-depth rows).
    let node_kinds: Vec<Option<NodeKind>> = patch
        .grid
        .cells
        .iter()
        .map(|c| c.params.as_ref().map(|p| p.kind()))
        .collect();

    // Paint the window background first (in global space); everything else is
    // drawn inside the transformed scene below. Use the available area only
    // (from the cursor onward) so the top toolbar above stays visible.
    ui.painter().rect_filled(
        ui.available_rect_before_wrap(),
        0.0,
        egui::Color32::from_rgb(18, 18, 22),
    );

    let scene = egui::Scene::new()
        .zoom_range(egui::Rangef::new(ZOOM_MIN, ZOOM_MAX))
        .scroll_zooms(true) // plain mouse wheel zooms in/out
        .drag_pan_buttons(egui::DragPanButtons::MIDDLE) // middle-button drag pans
        .max_inner_size(egui::vec2(1_000_000.0, 1_000_000.0)); // effectively unbounded scene

    scene.show(ui, scene_rect, |ui| {
        // `ui` is now in scene/world coordinates; `Scene` handles all panning
        // and zooming, anchored on the pointer.
        draw_grid(ui);

        // Map the global pointer position into scene coordinates for hit testing.
        let to_scene = ui
            .ctx()
            .layer_transform_from_global(ui.layer_id())
            .unwrap_or(egui::emath::TSTransform::IDENTITY);
        let world_hover = ui.input(|i| i.pointer.hover_pos()).map(|p| to_scene * p);
        let hover_inside = world_hover
            .map(|p| ui.clip_rect().contains(p))
            .unwrap_or(false);

        // Delete selected node and purge all links referencing it
        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
            && let Some(slot) = *selected
        {
            patch.grid.cells[slot].params = None;
            patch.grid.cells[slot].in0 = InputSel::None;
            patch.grid.cells[slot].in1 = InputSel::None;
            patch.grid.cells[slot].mods.clear();
            for cell in patch.grid.cells.iter_mut() {
                if cell.in0 == InputSel::Slot(slot as u8) {
                    cell.in0 = InputSel::None;
                }
                if cell.in1 == InputSel::Slot(slot as u8) {
                    cell.in1 = InputSel::None;
                }
                cell.mods.retain(|l| l.source as usize != slot);
            }
            patch.version = patch.version.wrapping_add(1);
            *selected = None;
        }

        // Right click opens the add-node context menu (at global screen pos,
        // but the world coordinate is stored in scene space for placement).
        let secondary_pressed = ui.input(|i| i.pointer.secondary_pressed());
        if secondary_pressed
            && hover_inside
            && let Some(p) = ui.input(|i| i.pointer.hover_pos())
        {
            *context_menu_pos = Some(p);
            *context_menu_world = Some(to_scene * p);
        }

        let mut node_rects: Vec<Option<egui::Rect>> = vec![None; GRID_CELLS];
        let mut mod_in_pos: Vec<Vec<egui::Pos2>> = vec![vec![]; GRID_CELLS];
        let mut mod_out_pos: Vec<Option<egui::Pos2>> = vec![None; GRID_CELLS];

        // ---------------------------------------------------------------------------------------------
        // Node areas
        // ---------------------------------------------------------------------------------------------
        for slot in 0..GRID_CELLS {
            let cell = &mut patch.grid.cells[slot];
            if cell.params.is_none() {
                continue;
            }

            let world_pos = positions[slot];
            let is_selected = *selected == Some(slot);
            let base_color = kind_color(cell.params.as_ref());
            let mut dirty = false;

            let scope = ui.scope_builder(
                egui::UiBuilder::new()
                    .id_salt(("node", slot))
                    .max_rect(egui::Rect::from_min_size(
                        world_pos,
                        egui::vec2(NODE_W, f32::INFINITY),
                    )),
                |ui| {
                    ui.set_width(NODE_W);
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(30, 30, 36))
                        .corner_radius(6.0)
                        .stroke(egui::Stroke::new(
                            1.5,
                            if is_selected {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_rgb(60, 60, 60)
                            },
                        ))
                        .show(ui, |ui| {
                            ui.set_width(NODE_W);

                            // Header (drag to move, click to select)
                            let header_resp = ui.allocate_response(
                                egui::vec2(NODE_W, HEADER_H),
                                egui::Sense::drag(),
                            );
                            let header_painter = ui.painter_at(header_resp.rect);
                            header_painter.rect_filled(header_resp.rect, 6.0, base_color);
                            header_painter.rect_filled(
                                egui::Rect::from_min_size(
                                    header_resp.rect.min + egui::vec2(0.0, 12.0),
                                    egui::vec2(NODE_W, 12.0),
                                ),
                                0.0,
                                base_color,
                            );
                            header_painter.text(
                                header_resp.rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!(
                                    "S{}  {}",
                                    slot,
                                    kind_label(cell.params.as_ref().map(|p| p.kind()))
                                ),
                                egui::FontId::proportional(12.0),
                                egui::Color32::WHITE,
                            );
                            // `drag_delta()` is already reported in scene units,
                            // so the node tracks the cursor at any zoom level.
                            if header_resp.dragged() {
                                positions[slot] += header_resp.drag_delta();
                            }
                            if header_resp.clicked() {
                                *selected = Some(slot);
                            }

                            // Body
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                                ui.add_space(4.0);

                                // Enum combos and integer voice counts
                                if let Some(np) = cell.params.as_mut() {
                                    match np {
                                        NodeParams::Analog {
                                            shape,
                                            pulse_width,
                                            unison_voices,
                                            ..
                                        } => {
                                            let mut shape_changed = false;
                                            egui::ComboBox::from_id_salt("shape")
                                                .selected_text(shape.to_string())
                                                .width(100.0)
                                                .show_ui(ui, |ui| {
                                                    for s in AnalogShape::ALL {
                                                        shape_changed |= ui
                                                            .selectable_value(
                                                                shape,
                                                                s,
                                                                s.to_string(),
                                                            )
                                                            .changed();
                                                    }
                                                });
                                            dirty |= shape_changed;
                                            dirty |= inline_slider_int(
                                                ui,
                                                "Voices",
                                                unison_voices,
                                                1..=16,
                                            );
                                            preview::preview_analog(
                                                ui,
                                                NODE_W - 12.0,
                                                base_color,
                                                *shape,
                                                *pulse_width,
                                            );
                                        }
                                        NodeParams::Noise { color, .. } => {
                                            let mut color_changed = false;
                                            egui::ComboBox::from_id_salt("color")
                                                .selected_text(color.to_string())
                                                .width(100.0)
                                                .show_ui(ui, |ui| {
                                                    for c in NoiseColor::ALL {
                                                        color_changed |= ui
                                                            .selectable_value(
                                                                color,
                                                                c,
                                                                c.to_string(),
                                                            )
                                                            .changed();
                                                    }
                                                });
                                            dirty |= color_changed;
                                            preview::preview_noise(
                                                ui,
                                                NODE_W - 12.0,
                                                base_color,
                                                *color,
                                                slot as u32,
                                            );
                                        }
                                        NodeParams::Wave {
                                            table,
                                            position,
                                            formant,
                                            skew_horizontal,
                                            skew_vertical,
                                            saturation,
                                            unison_voices,
                                            ..
                                        } => {
                                            let mut sel_table = table.clone();
                                            egui::ComboBox::from_id_salt("table")
                                                .selected_text(table_label(&sel_table))
                                                .width(120.0)
                                                .show_ui(ui, |ui| {
                                                    for t in lib.tables.iter() {
                                                        ui.selectable_value(
                                                            &mut sel_table,
                                                            TableSel::Custom(t.name.clone()),
                                                            t.name.clone(),
                                                        );
                                                    }
                                                    for s in StockTable::ALL {
                                                        ui.selectable_value(
                                                            &mut sel_table,
                                                            TableSel::Stock(s),
                                                            stock_label(&s),
                                                        );
                                                    }
                                                });
                                            if sel_table != *table {
                                                *table = sel_table;
                                                dirty = true;
                                            }
                                            dirty |= inline_slider_int(
                                                ui,
                                                "Voices",
                                                unison_voices,
                                                1..=16,
                                            );
                                            let src = wavetable::resolve(table, &lib);
                                            preview::preview_wave(
                                                ui,
                                                NODE_W - 12.0,
                                                base_color,
                                                &src,
                                                *position,
                                                *formant,
                                                *skew_horizontal,
                                                *skew_vertical,
                                                *saturation,
                                            );
                                        }
                                        NodeParams::Lfo { shape, .. } => {
                                            let mut shape_changed = false;
                                            egui::ComboBox::from_id_salt("lfoshape")
                                                .selected_text(shape.to_string())
                                                .width(100.0)
                                                .show_ui(ui, |ui| {
                                                    for s in LfoShape::ALL {
                                                        shape_changed |= ui
                                                            .selectable_value(
                                                                shape,
                                                                s,
                                                                s.to_string(),
                                                            )
                                                            .changed();
                                                    }
                                                });
                                            dirty |= shape_changed;
                                            preview::preview_lfo(
                                                ui,
                                                NODE_W - 12.0,
                                                base_color,
                                                *shape,
                                            );
                                        }
                                        NodeParams::Chiptune {
                                            table,
                                            position,
                                            arp_enabled,
                                            arp_three,
                                            arp_steps,
                                            noise,
                                            ..
                                        } => {
                                            let mut sel_table = table.clone();
                                            egui::ComboBox::from_id_salt("chiptune_table")
                                                .selected_text(table_label(&sel_table))
                                                .width(120.0)
                                                .show_ui(ui, |ui| {
                                                    for t in lib.tables.iter() {
                                                        ui.selectable_value(
                                                            &mut sel_table,
                                                            TableSel::Custom(t.name.clone()),
                                                            t.name.clone(),
                                                        );
                                                    }
                                                    for s in StockTable::ALL {
                                                        ui.selectable_value(
                                                            &mut sel_table,
                                                            TableSel::Stock(s),
                                                            stock_label(&s),
                                                        );
                                                    }
                                                });
                                            if sel_table != *table {
                                                *table = sel_table;
                                                dirty = true;
                                            }
                                            dirty |= ui.checkbox(arp_enabled, "Arp").changed();
                                            dirty |= ui.checkbox(arp_three, "3-step").changed();
                                            dirty |= ui.checkbox(noise, "Noise").changed();
                                            for (step_idx, step) in arp_steps.iter_mut().enumerate()
                                            {
                                                let mut as_f32 = *step as f32;
                                                if ui
                                                    .horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "Step{}",
                                                                step_idx + 1
                                                            ))
                                                            .size(10.0)
                                                            .color(egui::Color32::LIGHT_GRAY),
                                                        );
                                                        ui.add(
                                                            egui::Slider::new(
                                                                &mut as_f32,
                                                                -24.0..=24.0,
                                                            )
                                                            .integer(),
                                                        )
                                                        .changed()
                                                    })
                                                    .inner
                                                {
                                                    *step = as_f32.round() as i32;
                                                    dirty = true;
                                                }
                                            }
                                            let src = wavetable::resolve(table, &lib);
                                            preview::preview_chiptune(
                                                ui,
                                                NODE_W - 12.0,
                                                base_color,
                                                &src,
                                                *position,
                                                *noise,
                                            );
                                        }
                                        NodeParams::Multi {
                                            table,
                                            position,
                                            spread,
                                            ..
                                        } => {
                                            let mut sel_table = table.clone();
                                            egui::ComboBox::from_id_salt("multi_table")
                                                .selected_text(table_label(&sel_table))
                                                .width(120.0)
                                                .show_ui(ui, |ui| {
                                                    for t in lib.tables.iter() {
                                                        ui.selectable_value(
                                                            &mut sel_table,
                                                            TableSel::Custom(t.name.clone()),
                                                            t.name.clone(),
                                                        );
                                                    }
                                                    for s in StockTable::ALL {
                                                        ui.selectable_value(
                                                            &mut sel_table,
                                                            TableSel::Stock(s),
                                                            stock_label(&s),
                                                        );
                                                    }
                                                });
                                            if sel_table != *table {
                                                *table = sel_table;
                                                dirty = true;
                                            }
                                            let src = wavetable::resolve(table, &lib);
                                            preview::preview_multi(
                                                ui,
                                                NODE_W - 12.0,
                                                base_color,
                                                &src,
                                                *position,
                                                *spread,
                                            );
                                        }
                                        NodeParams::Vector {
                                            tables,
                                            positions,
                                            x,
                                            y,
                                            ..
                                        } => {
                                            for (edge_idx, table) in tables.iter_mut().enumerate() {
                                                let mut sel_table = table.clone();
                                                egui::ComboBox::from_id_salt((
                                                    "vec_table",
                                                    edge_idx,
                                                ))
                                                .selected_text(format!(
                                                    "E{} {}",
                                                    edge_idx,
                                                    table_label(&sel_table)
                                                ))
                                                .width(120.0)
                                                .show_ui(ui, |ui| {
                                                    for t in lib.tables.iter() {
                                                        ui.selectable_value(
                                                            &mut sel_table,
                                                            TableSel::Custom(t.name.clone()),
                                                            t.name.clone(),
                                                        );
                                                    }
                                                    for s in StockTable::ALL {
                                                        ui.selectable_value(
                                                            &mut sel_table,
                                                            TableSel::Stock(s),
                                                            stock_label(&s),
                                                        );
                                                    }
                                                });
                                                if sel_table != *table {
                                                    *table = sel_table;
                                                    dirty = true;
                                                }
                                            }
                                            let sources = [
                                                wavetable::resolve(&tables[0], &lib),
                                                wavetable::resolve(&tables[1], &lib),
                                                wavetable::resolve(&tables[2], &lib),
                                                wavetable::resolve(&tables[3], &lib),
                                            ];
                                            preview::preview_vector(
                                                ui,
                                                NODE_W - 12.0,
                                                base_color,
                                                &sources,
                                                *positions,
                                                *x,
                                                *y,
                                            );
                                        }
                                        NodeParams::Fm {
                                            carrier_table,
                                            modulator_table,
                                            carrier_position,
                                            modulator_position,
                                            amount,
                                            carrier_ratio,
                                            modulator_ratio,
                                            exponential,
                                            ..
                                        } => {
                                            dirty |=
                                                table_picker(ui, "carrier", carrier_table, &lib);
                                            dirty |=
                                                table_picker(ui, "modulatr", modulator_table, &lib);
                                            dirty |=
                                                ui.checkbox(exponential, "Exponential").changed();
                                            dirty |= inline_slider_int(
                                                ui,
                                                "C ratio",
                                                carrier_ratio,
                                                1..=16,
                                            );
                                            dirty |= inline_slider_int(
                                                ui,
                                                "M ratio",
                                                modulator_ratio,
                                                1..=16,
                                            );
                                            let c_src = wavetable::resolve(carrier_table, &lib);
                                            let m_src = wavetable::resolve(modulator_table, &lib);
                                            preview::preview_fm(
                                                ui,
                                                NODE_W - 12.0,
                                                base_color,
                                                &c_src,
                                                &m_src,
                                                *carrier_position,
                                                *modulator_position,
                                                *amount,
                                                *carrier_ratio as f32,
                                                *modulator_ratio as f32,
                                                *exponential,
                                            );
                                        }
                                        NodeParams::Pm {
                                            carrier_table,
                                            modulator_table,
                                            carrier_position,
                                            modulator_position,
                                            amount,
                                            carrier_ratio,
                                            modulator_ratio,
                                            ..
                                        } => {
                                            dirty |=
                                                table_picker(ui, "carrier", carrier_table, &lib);
                                            dirty |=
                                                table_picker(ui, "modulatr", modulator_table, &lib);
                                            dirty |= inline_slider_int(
                                                ui,
                                                "C ratio",
                                                carrier_ratio,
                                                1..=16,
                                            );
                                            dirty |= inline_slider_int(
                                                ui,
                                                "M ratio",
                                                modulator_ratio,
                                                1..=16,
                                            );
                                            let c_src = wavetable::resolve(carrier_table, &lib);
                                            let m_src = wavetable::resolve(modulator_table, &lib);
                                            preview::preview_pm(
                                                ui,
                                                NODE_W - 12.0,
                                                base_color,
                                                &c_src,
                                                &m_src,
                                                *carrier_position,
                                                *modulator_position,
                                                *amount,
                                                *carrier_ratio as f32,
                                                *modulator_ratio as f32,
                                            );
                                        }
                                        _ => {}
                                    }
                                }

                                // Generic float parameters with mod input sockets
                                let defs: &'static [ParamDef] =
                                    cell.params.as_ref().map(|p| p.param_defs()).unwrap_or(&[]);
                                for (i, def) in defs.iter().enumerate() {
                                    let link_idx =
                                        cell.mods.iter().position(|l| l.param as usize == i);
                                    let has_link = link_idx.is_some();
                                    let mut v =
                                        cell.params.as_ref().map(|p| p.param_get(i)).unwrap_or(0.0);
                                    let mut depth =
                                        link_idx.map(|li| cell.mods[li].depth).unwrap_or(0.0);

                                    let row = ui.horizontal(|ui| {
                                        let (dot_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(10.0, 10.0),
                                            egui::Sense::hover(),
                                        );
                                        let dot_color = if has_link {
                                            egui::Color32::from_rgb(90, 220, 220)
                                        } else {
                                            egui::Color32::from_rgb(90, 90, 96)
                                        };
                                        ui.painter().circle_filled(
                                            dot_rect.center(),
                                            3.5,
                                            dot_color,
                                        );
                                        mod_in_pos[slot].push(dot_rect.center());

                                        ui.add(
                                            egui::Slider::new(&mut v, def.min..=def.max)
                                                .text(def.name),
                                        )
                                        .changed()
                                    });

                                    if row.inner {
                                        if let Some(np) = cell.params.as_mut() {
                                            np.param_set(i, v);
                                        }
                                        dirty = true;
                                    }

                                    if let Some(li) = link_idx {
                                        let src = cell.mods[li].source as usize;
                                        let src_kind = node_kinds.get(src).copied().flatten();
                                        let changed = ui
                                            .horizontal(|ui| {
                                                ui.add_space(14.0);
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} → amount",
                                                        kind_label(src_kind)
                                                    ))
                                                    .size(10.0)
                                                    .color(egui::Color32::LIGHT_GRAY),
                                                );
                                                ui.add(
                                                    egui::Slider::new(&mut depth, -1.0..=1.0)
                                                        .show_value(true),
                                                )
                                                .changed()
                                            })
                                            .inner;
                                        if changed {
                                            cell.mods[li].depth = depth;
                                            dirty = true;
                                        }
                                    }
                                }

                                ui.add_space(8.0);
                            });
                        });
                },
            );

            node_rects[slot] = Some(scope.response.rect);

            if dirty {
                patch.version = patch.version.wrapping_add(1);
            }
        }

        // Mod output socket positions (right edge, below header)
        for slot in 0..GRID_CELLS {
            if let Some(node_rect) = node_rects[slot]
                && patch.grid.cells[slot].params.is_some()
            {
                mod_out_pos[slot] = Some(egui::pos2(
                    node_rect.right(),
                    node_rect.top() + HEADER_H + 10.0,
                ));
            }
        }

        // ---------------------------------------------------------------------------------------------
        // Hit testing
        // ---------------------------------------------------------------------------------------------
        let mut hovered_mod_in: Option<(usize, usize)> = None;
        let mut hovered_mod_out: Option<usize> = None;
        let mut hovered_port: Option<(usize, usize, bool)> = None;
        let mut hovered_node: Option<usize> = None;

        if let Some(h) = world_hover {
            for slot in 0..GRID_CELLS {
                for (param, pos) in mod_in_pos[slot].iter().enumerate() {
                    if (*pos - h).length() < 7.0 {
                        hovered_mod_in = Some((slot, param));
                    }
                }
                if let Some(pos) = mod_out_pos[slot]
                    && (pos - h).length() < 7.0
                {
                    hovered_mod_out = Some(slot);
                }
            }

            for slot in 0..GRID_CELLS {
                if let Some(node_rect) = node_rects[slot] {
                    let cell = &patch.grid.cells[slot];
                    if cell.params.is_none() {
                        continue;
                    }
                    let inputs = cell.params.as_ref().unwrap().input_count();
                    for i in 0..inputs {
                        let py = node_rect.top() + HEADER_H + 12.0 + i as f32 * 18.0;
                        let pos = egui::pos2(node_rect.left(), py);
                        if (pos - h).length() < 10.0 {
                            hovered_port = Some((slot, i, true));
                        }
                    }
                    let out_pos = egui::pos2(node_rect.right(), node_rect.center().y);
                    if (out_pos - h).length() < 10.0 {
                        hovered_port = Some((slot, 0, false));
                    }
                }
            }

            if hovered_mod_in.is_none() && hovered_mod_out.is_none() && hovered_port.is_none() {
                for slot in (0..GRID_CELLS).rev() {
                    if let Some(node_rect) = node_rects[slot]
                        && node_rect.contains(h)
                    {
                        hovered_node = Some(slot);
                        break;
                    }
                }
            }
        }

        // ---------------------------------------------------------------------------------------------
        // Pointer interaction
        // ---------------------------------------------------------------------------------------------
        let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
        let primary_released = ui.input(|i| i.pointer.primary_released());

        if primary_pressed {
            if let Some((slot, param)) = hovered_mod_in {
                let before = patch.grid.cells[slot].mods.len();
                patch.grid.cells[slot]
                    .mods
                    .retain(|l| l.param as usize != param);
                if patch.grid.cells[slot].mods.len() != before {
                    patch.version = patch.version.wrapping_add(1);
                }
            } else if let Some(slot) = hovered_mod_out {
                *drag = DragState::ModCable {
                    from: slot,
                    current: mod_out_pos[slot].unwrap_or_default(),
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
                    let node_rect = node_rects[slot].unwrap();
                    let start = egui::pos2(node_rect.right(), node_rect.center().y);
                    *drag = DragState::Cable {
                        from: slot,
                        current: start,
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
                DragState::Cable { from, current } => {
                    // Connect the dragged output to the hovered input port.
                    if let Some((slot, port, true)) = hovered_port
                        && slot != from
                    {
                        let cell = &mut patch.grid.cells[slot];
                        if port == 0 {
                            cell.in0 = InputSel::Slot(from as u8);
                        } else {
                            cell.in1 = InputSel::Slot(from as u8);
                        }
                        patch.version = patch.version.wrapping_add(1);
                    }
                    let _ = current;
                }
                DragState::ModCable { from, current } => {
                    // Connect the dragged mod output to the hovered mod input,
                    // replacing any previous link to that parameter.
                    if let Some((slot, param)) = hovered_mod_in
                        && slot != from
                    {
                        let cell = &mut patch.grid.cells[slot];
                        cell.mods.retain(|l| l.param as usize != param);
                        cell.mods.push(ModLink {
                            source: from as u8,
                            param: param as u8,
                            // start at full range so the connection is
                            // immediately audible, not silently zero
                            depth: 1.0,
                        });
                        patch.version = patch.version.wrapping_add(1);
                    }
                    let _ = current;
                }
                DragState::None => {}
            }
            *drag = DragState::None;
        }

        match *drag {
            DragState::Cable { from, .. } => {
                if let Some(h) = world_hover {
                    *drag = DragState::Cable { from, current: h };
                }
            }
            DragState::ModCable { from, .. } => {
                if let Some(h) = world_hover {
                    *drag = DragState::ModCable { from, current: h };
                }
            }
            DragState::None => {}
        }

        // ---------------------------------------------------------------------------------------------
        // Cables (drawn first, so node bodies paint on top)
        // ---------------------------------------------------------------------------------------------
        for (slot, cell) in patch.grid.cells.iter().enumerate() {
            if cell.params.is_none() {
                continue;
            }
            let inputs = cell.params.as_ref().unwrap().input_count();
            for (port, sel) in [(0usize, cell.in0), (1usize, cell.in1)] {
                if port >= inputs {
                    continue;
                }
                if let Some(src) = patch.grid.resolve(slot, sel)
                    && let (Some(from_rect), Some(to_rect)) =
                        (node_rects[src as usize], node_rects[slot])
                {
                    let from = egui::pos2(from_rect.right(), from_rect.center().y);
                    let to = egui::pos2(
                        to_rect.left(),
                        to_rect.top() + HEADER_H + 12.0 + port as f32 * 18.0,
                    );
                    draw_cable(
                        ui,
                        from,
                        to,
                        42.0,
                        egui::Color32::TRANSPARENT,
                        2.5,
                        kind_color(patch.grid.cells[src as usize].params.as_ref()),
                    );
                }
            }
        }

        for (slot, cell) in patch.grid.cells.iter().enumerate() {
            for link in cell.mods.iter() {
                let src = link.source as usize;
                if let (Some(from), Some(to)) = (
                    mod_out_pos[src],
                    mod_in_pos[slot].get(link.param as usize).copied(),
                ) {
                    draw_cable(
                        ui,
                        from,
                        to,
                        30.0,
                        egui::Color32::TRANSPARENT,
                        2.0,
                        egui::Color32::from_rgb(90, 220, 220),
                    );
                }
            }
        }

        if let DragState::Cable { from, current } = *drag
            && let Some(from_rect) = node_rects[from]
        {
            let from_pos = egui::pos2(from_rect.right(), from_rect.center().y);
            draw_cable(
                ui,
                from_pos,
                current,
                42.0,
                egui::Color32::TRANSPARENT,
                2.5,
                egui::Color32::from_rgb(240, 200, 80),
            );
        }

        if let DragState::ModCable { from, current } = *drag
            && let Some(from_pos) = mod_out_pos[from]
        {
            draw_cable(
                ui,
                from_pos,
                current,
                30.0,
                egui::Color32::TRANSPARENT,
                2.0,
                egui::Color32::from_rgb(90, 220, 220),
            );
        }

        // ---------------------------------------------------------------------------------------------
        // Ports (drawn last, on top of node bodies)
        // ---------------------------------------------------------------------------------------------
        for slot in 0..GRID_CELLS {
            if let Some(node_rect) = node_rects[slot] {
                let cell = &patch.grid.cells[slot];
                if cell.params.is_none() {
                    continue;
                }
                let inputs = cell.params.as_ref().unwrap().input_count();
                let base_color = kind_color(cell.params.as_ref());

                for i in 0..inputs {
                    let py = node_rect.top() + HEADER_H + 12.0 + i as f32 * 18.0;
                    let pos = egui::pos2(node_rect.left(), py);
                    ui.painter()
                        .circle_filled(pos, 4.0, egui::Color32::from_rgb(205, 205, 205));
                    ui.painter().circle_stroke(
                        pos,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    );
                    ui.painter().text(
                        pos + egui::vec2(8.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        format!("In {}", i + 1),
                        egui::FontId::proportional(10.0),
                        egui::Color32::LIGHT_GRAY,
                    );
                }

                let out_pos = egui::pos2(node_rect.right(), node_rect.center().y);
                ui.painter().circle_filled(out_pos, 4.0, base_color);
                ui.painter().circle_stroke(
                    out_pos,
                    4.0,
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );
                ui.painter().text(
                    out_pos + egui::vec2(-8.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    "Out",
                    egui::FontId::proportional(10.0),
                    egui::Color32::LIGHT_GRAY,
                );

                let mod_out = egui::pos2(node_rect.right(), node_rect.top() + HEADER_H + 10.0);
                ui.painter()
                    .circle_filled(mod_out, 4.0, egui::Color32::from_rgb(90, 220, 220));
                ui.painter().circle_stroke(
                    mod_out,
                    4.0,
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );
            }
        }
    });

    // ---------------------------------------------------------------------------------------------
    // Add-node context menu (global screen space, above the scene)
    // ---------------------------------------------------------------------------------------------
    if let (Some(screen_pos), Some(world_pos)) = (*context_menu_pos, *context_menu_world) {
        let mut close_menu = false;
        egui::Area::new(egui::Id::unique("node_context_menu"))
            .fixed_pos(screen_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(40, 40, 45))
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.set_width(120.0);
                        ui.label("Add Node");
                        ui.separator();
                        for kind in KIND_LIST {
                            if ui.button(kind_label(Some(kind))).clicked() {
                                if let Some(slot) =
                                    patch.grid.cells.iter().position(|c| c.params.is_none())
                                {
                                    patch.grid.cells[slot].params =
                                        Some(NodeParams::default_for(kind));
                                    patch.grid.cells[slot].in0 = InputSel::None;
                                    patch.grid.cells[slot].in1 = InputSel::None;
                                    patch.grid.cells[slot].mods.clear();
                                    positions[slot] = world_pos;
                                    *selected = Some(slot);
                                    patch.version = patch.version.wrapping_add(1);
                                }
                                close_menu = true;
                            }
                        }
                    });
            });

        if close_menu || ui.input(|i| i.pointer.primary_clicked()) {
            *context_menu_pos = None;
            *context_menu_world = None;
        }
    }
}

/// Paint an infinite grid of dots aligned to the world-space origin, clipped
/// to the currently visible scene region.
fn draw_grid(ui: &egui::Ui) {
    let rect = ui.clip_rect();
    let mut y = rect.min.y - rect.min.y.rem_euclid(GRID_STEP);
    while y <= rect.max.y {
        let mut x = rect.min.x - rect.min.x.rem_euclid(GRID_STEP);
        while x <= rect.max.x {
            ui.painter()
                .circle_filled(egui::pos2(x, y), 1.5, egui::Color32::from_rgb(35, 35, 40));
            x += GRID_STEP;
        }
        y += GRID_STEP;
    }
}

/// Draw a cubic bezier cable with horizontal control handles.
fn draw_cable(
    ui: &egui::Ui,
    from: egui::Pos2,
    to: egui::Pos2,
    handle_len: f32,
    fill: egui::Color32,
    width: f32,
    color: egui::Color32,
) {
    ui.painter()
        .add(egui::epaint::CubicBezierShape::from_points_stroke(
            [
                from,
                from + egui::vec2(handle_len, 0.0),
                to - egui::vec2(handle_len, 0.0),
                to,
            ],
            false,
            fill,
            egui::Stroke::new(width, color),
        ));
}

fn inline_slider_int(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u8,
    range: std::ops::RangeInclusive<u8>,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(10.0)
                .color(egui::Color32::LIGHT_GRAY),
        );
        ui.add(egui::Slider::new(value, range)).changed()
    })
    .inner
}

/// A stock/custom wavetable picker combo; returns true when the selection changed.
fn table_picker(
    ui: &mut egui::Ui,
    id: &str,
    table: &mut TableSel,
    lib: &wavetable::CustomLibrary,
) -> bool {
    let mut sel_table = table.clone();
    egui::ComboBox::from_id_salt(id)
        .selected_text(table_label(&sel_table))
        .width(120.0)
        .show_ui(ui, |ui| {
            for t in lib.tables.iter() {
                ui.selectable_value(
                    &mut sel_table,
                    TableSel::Custom(t.name.clone()),
                    t.name.clone(),
                );
            }
            for s in StockTable::ALL {
                ui.selectable_value(&mut sel_table, TableSel::Stock(s), stock_label(&s));
            }
        });
    if sel_table != *table {
        *table = sel_table;
        true
    } else {
        false
    }
}
