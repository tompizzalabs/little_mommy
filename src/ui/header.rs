//! Top header bar: global controls, presets, and master parameters.

use crate::engine::graph::{Patch, PlayMode};
use crate::engine::osc::wavetable;
use crate::params::LittleMommyParams;
use crate::presets;
use crate::ui::state::{auto_layout, content_bbox};
use nice_plug::context::gui::ParamSetter;
use nice_plug::prelude::Param;
use std::sync::Arc;

// UI state is passed through explicitly; splitting into a struct adds indirection without benefit.
#[allow(clippy::too_many_arguments)]
pub fn top_bar(
    ui: &mut egui::Ui,
    patch: &mut Patch,
    positions: &mut [egui::Pos2],
    scene_rect: &mut egui::Rect,
    preset_index: &mut usize,
    selected: &mut Option<usize>,
    params: &Arc<LittleMommyParams>,
    setter: &ParamSetter,
) {
    ui.horizontal_wrapped(|ui| {
        ui.heading("Little Mommy III");
        ui.separator();

        // Preset
        let names: Vec<&'static str> = presets::factory_presets().iter().map(|p| p.name).collect();
        let mut preset_changed = false;
        egui::ComboBox::from_label("Preset")
            .selected_text(names[*preset_index])
            .show_ui(ui, |ui| {
                for (i, name) in names.iter().enumerate() {
                    preset_changed |= ui.selectable_value(preset_index, i, *name).changed();
                }
            });
        if preset_changed {
            let preset = &presets::factory_presets()[*preset_index];
            patch.grid = crate::engine::graph::Grid::new();
            for (slot, params, in0, in1) in preset.cells.iter() {
                patch.grid.set(*slot, Some(params.clone()), *in0, *in1);
            }
            patch.play_mode = preset.play_mode;
            patch.version = patch.version.wrapping_add(1);
            *selected = None;
            auto_layout(patch, positions);
            *scene_rect = content_bbox(patch, positions);
        }

        ui.separator();

        // Play Mode
        let mut mode = patch.play_mode;
        egui::ComboBox::from_label("Play Mode")
            .selected_text(mode.to_string())
            .show_ui(ui, |ui| {
                for m in PlayMode::ALL {
                    ui.selectable_value(&mut mode, m, m.to_string());
                }
            });
        if mode != patch.play_mode {
            patch.play_mode = mode;
            patch.version = patch.version.wrapping_add(1);
        }

        ui.separator();

        // Master Gain
        let mut gain = params.master_gain.modulated_normalized_value();
        let resp = ui.add(egui::Slider::new(&mut gain, 0.0..=1.0).text("Master"));
        if resp.drag_started() {
            setter.begin_set_parameter(&params.master_gain);
        }
        if resp.changed() {
            setter.set_parameter_normalized(&params.master_gain, gain);
        }
        if resp.drag_stopped() {
            setter.end_set_parameter(&params.master_gain);
        }

        ui.separator();

        if ui.button("Rescan Wavetables").clicked() {
            wavetable::rescan();
        }
    });
}
