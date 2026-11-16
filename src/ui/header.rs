use super::knob::Knob;
use super::state::{auto_layout, content_bbox};
use super::theme as th;
use crate::engine::graph::{Grid, Patch, PlayMode};
use crate::params::LittleMommyParams;
use crate::presets;
use nice_plug::context::gui::ParamSetter;
use nice_plug::prelude::Param;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn top_bar(
    ui: &mut egui::Ui,
    patch: &mut Patch,
    positions: &mut Vec<egui::Pos2>,
    scene_rect: &mut egui::Rect,
    preset_index: &mut usize,
    selected: &mut Option<usize>,
    params: &Arc<LittleMommyParams>,
    setter: &ParamSetter,
) {
    let full = ui.available_rect_before_wrap();
    let band = egui::Rect::from_min_size(full.min, egui::vec2(full.width(), 64.0));
    ui.allocate_rect(band, egui::Sense::hover());
    ui.painter().rect_filled(band, 0.0, th::SURFACE);
    ui.painter().line_segment(
        [
            egui::pos2(band.left(), band.bottom()),
            egui::pos2(band.right(), band.bottom()),
        ],
        egui::Stroke::new(1.0, th::INSET_EDGE),
    );

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(band.shrink2(egui::vec2(12.0, 7.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);

    child.add_space(14.0);
    child.label(
        egui::RichText::new("LITTLE MOMMY III")
            .size(13.0)
            .strong()
            .color(th::TEXT_BRIGHT),
    );
    child.add_space(6.0);
    child.painter().vline(
        child.cursor().left(),
        band.y_range().shrink(14.0),
        egui::Stroke::new(1.0, th::INSET_EDGE),
    );
    child.add_space(6.0);

    let presets = presets::factory_presets();
    let mut preset_changed = false;
    egui::ComboBox::from_id_salt("preset_pick")
        .selected_text(presets[*preset_index].name)
        .width(160.0)
        .height(300.0)
        .show_ui(&mut child, |ui| {
            for (candidate_index, preset) in presets.iter().enumerate() {
                preset_changed |= ui
                    .selectable_value(preset_index, candidate_index, preset.name)
                    .changed();
            }
        });
    if preset_changed {
        let preset = &presets[*preset_index];
        patch.grid = Grid::new();
        for (slot, params, in0, in1) in preset.cells.iter() {
            patch.grid.set(*slot, Some(params.clone()), *in0, *in1);
        }
        patch.play_mode = preset.play_mode;
        patch.version = patch.version.wrapping_add(1);
        *selected = None;
        if positions.len() < patch.grid.cells.len() {
            positions.resize(patch.grid.cells.len(), egui::Pos2::ZERO);
        }
        auto_layout(patch, positions);
        *scene_rect = content_bbox(patch, positions);
    }

    let mut selected_mode = patch.play_mode;
    egui::ComboBox::from_id_salt("play_mode_pick")
        .selected_text(selected_mode.to_string())
        .width(120.0)
        .show_ui(&mut child, |ui| {
            for mode in PlayMode::ALL {
                ui.selectable_value(&mut selected_mode, mode, mode.to_string());
            }
        });
    if selected_mode != patch.play_mode {
        patch.play_mode = selected_mode;
        patch.version = patch.version.wrapping_add(1);
    }

    child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let active_node_count = patch
            .grid
            .cells
            .iter()
            .filter(|cell| cell.params.is_some())
            .count();
        let mut gain = params.master_gain.modulated_normalized_value();
        let response = Knob::new(&mut gain, 0.0, 1.0)
            .accent(th::CV)
            .size(26.0)
            .label("MASTER")
            .default(0.8)
            .ui(ui);
        if response.response.drag_started() {
            setter.begin_set_parameter(&params.master_gain);
        }
        if response.response.changed() {
            setter.set_parameter_normalized(&params.master_gain, gain);
        }
        if response.response.drag_stopped() {
            setter.end_set_parameter(&params.master_gain);
        }
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(format!(
                "{} NODE{}",
                active_node_count,
                if active_node_count == 1 { "" } else { "S" }
            ))
            .size(10.0)
            .color(th::TEXT_FAINT),
        );
        ui.add_space(8.0);
        ui.painter().vline(
            ui.cursor().left(),
            band.y_range().shrink(14.0),
            egui::Stroke::new(1.0, th::INSET_EDGE),
        );
    });
}
