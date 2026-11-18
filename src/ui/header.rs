use super::knob::Knob;
use super::state::apply_patch;
use super::theme as th;
use crate::engine::graph::{Patch, PlayMode};
use crate::params::SozvezdieParams;
use crate::preset_bank::{self, PresetEntry, PresetOrigin};
use crate::presets;
use nice_plug::context::gui::ParamSetter;
use nice_plug::prelude::Param;
use std::sync::Arc;

pub struct HeaderState {
    pub listing: Vec<PresetEntry>,
    pub listing_valid: bool,
    pub current: Option<PresetEntry>,
    pub browser_open: bool,
    pub browser_category: Option<(PresetOrigin, String)>,
    pub save_open: bool,
    pub save_name: String,
    pub save_category: String,
    pub save_author: String,
    pub save_status: Option<String>,
}

impl Default for HeaderState {
    fn default() -> Self {
        Self {
            listing: Vec::new(),
            listing_valid: false,
            current: None,
            browser_open: false,
            browser_category: None,
            save_open: false,
            save_name: String::new(),
            save_category: String::new(),
            save_author: String::new(),
            save_status: None,
        }
    }
}

impl HeaderState {
    fn rescan(&mut self) {
        self.listing = preset_bank::scan();
        self.listing_valid = true;
    }

    fn ensure_listing(&mut self) {
        if !self.listing_valid {
            self.rescan();
        }
    }

    fn current_index(&self) -> Option<usize> {
        let current = self.current.as_ref()?;
        self.listing
            .iter()
            .position(|entry| entry.path == current.path)
    }

    fn load_entry(
        &mut self,
        entry: &PresetEntry,
        patch: &mut Patch,
        positions: &mut Vec<egui::Pos2>,
        selected: &mut Option<usize>,
        scene_rect: &mut egui::Rect,
    ) {
        match preset_bank::load_preset(&entry.path) {
            Ok(file) => {
                apply_patch(patch, positions, selected, scene_rect, file.patch);
                self.current = Some(entry.clone());
                self.save_status = None;
            }
            Err(error) => {
                self.save_status = Some(format!("Load failed: {error}"));
            }
        }
    }

    fn step(
        &mut self,
        direction: i32,
        patch: &mut Patch,
        positions: &mut Vec<egui::Pos2>,
        selected: &mut Option<usize>,
        scene_rect: &mut egui::Rect,
    ) {
        self.ensure_listing();
        if self.listing.is_empty() {
            return;
        }
        let count = self.listing.len() as i32;
        let index = self
            .current_index()
            .map_or(if direction < 0 { count - 1 } else { 0 }, |index| {
                (index as i32 + direction).rem_euclid(count)
            });
        let entry = self.listing[index as usize].clone();
        self.load_entry(&entry, patch, positions, selected, scene_rect);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn top_bar(
    ui: &mut egui::Ui,
    patch: &mut Patch,
    positions: &mut Vec<egui::Pos2>,
    scene_rect: &mut egui::Rect,
    header_state: &mut HeaderState,
    selected: &mut Option<usize>,
    params: &Arc<SozvezdieParams>,
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
        egui::RichText::new("SOZVEZDIE")
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

    if child.button("<").clicked() {
        header_state.step(-1, patch, positions, selected, scene_rect);
    }
    let current_name = header_state
        .current
        .as_ref()
        .map(|entry| entry.name.clone())
        .unwrap_or_else(|| "Init".to_owned());
    child.add_sized(
        egui::vec2(150.0, 18.0),
        egui::Label::new(
            egui::RichText::new(current_name)
                .size(12.0)
                .strong()
                .color(th::TEXT_BRIGHT),
        ),
    );
    if child.button(">").clicked() {
        header_state.step(1, patch, positions, selected, scene_rect);
    }
    child.add_space(4.0);
    if child.button("INIT").clicked() {
        apply_patch(
            patch,
            positions,
            selected,
            scene_rect,
            presets::default_patch(),
        );
        header_state.current = None;
        header_state.save_status = None;
    }
    if child.button("SAVE").clicked() {
        header_state.browser_open = false;
        if header_state.save_name.is_empty() {
            header_state.save_name = header_state
                .current
                .as_ref()
                .map_or_else(String::new, |entry| entry.name.clone());
        }
        if header_state.save_category.is_empty() {
            header_state.save_category = header_state
                .current
                .as_ref()
                .map_or_else(|| "Misc".to_owned(), |entry| entry.category.clone());
        }
        header_state.save_status = None;
        header_state.save_open = true;
    }
    if child.button("BROWSE").clicked() {
        header_state.save_open = false;
        header_state.ensure_listing();
        header_state.browser_open = true;
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

pub fn browser_window(
    ui: &mut egui::Ui,
    patch: &mut Patch,
    positions: &mut Vec<egui::Pos2>,
    selected: &mut Option<usize>,
    scene_rect: &mut egui::Rect,
    state: &mut HeaderState,
) {
    if !state.browser_open {
        return;
    }
    let mut open = state.browser_open;
    egui::Window::new("PRESET BROWSER")
        .open(&mut open)
        .default_width(480.0)
        .default_height(360.0)
        .show(ui, |ui| {
            state.ensure_listing();
            ui.horizontal(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("preset_categories")
                    .max_width(160.0)
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            for origin in [PresetOrigin::Community, PresetOrigin::User] {
                                ui.label(
                                    egui::RichText::new(origin.label().to_uppercase())
                                        .size(10.0)
                                        .strong()
                                        .color(th::CV),
                                );
                                for category in categories_of(state, origin) {
                                    let is_selected =
                                        state.browser_category.as_ref().is_some_and(|selected| {
                                            *selected == (origin, category.clone())
                                        });
                                    if ui.selectable_label(is_selected, &category).clicked() {
                                        state.browser_category = Some((origin, category));
                                    }
                                }
                                ui.add_space(6.0);
                            }
                            if ui.button("REFRESH").clicked() {
                                state.rescan();
                            }
                        });
                    });
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("preset_list")
                    .show(ui, |ui| {
                        let entries: Vec<PresetEntry> =
                            entries_in(state).into_iter().cloned().collect();
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            for entry in entries {
                                let author = if entry.author.is_empty() {
                                    String::from("-")
                                } else {
                                    entry.author.clone()
                                };
                                let is_current = state
                                    .current
                                    .as_ref()
                                    .is_some_and(|current| current.path == entry.path);
                                if ui
                                    .selectable_label(
                                        is_current,
                                        format!("{}  -  {}", entry.name, author),
                                    )
                                    .clicked()
                                {
                                    state
                                        .load_entry(&entry, patch, positions, selected, scene_rect);
                                }
                            }
                        });
                    });
            });
        });
    state.browser_open = open;
}

fn categories_of(state: &HeaderState, origin: PresetOrigin) -> Vec<String> {
    let mut categories: Vec<String> = Vec::new();
    for entry in &state.listing {
        if entry.origin == origin && !categories.contains(&entry.category) {
            categories.push(entry.category.clone());
        }
    }
    categories
}

fn entries_in(state: &HeaderState) -> Vec<&PresetEntry> {
    state
        .listing
        .iter()
        .filter(|entry| match &state.browser_category {
            Some((origin, category)) => entry.origin == *origin && &entry.category == category,
            None => true,
        })
        .collect()
}

pub fn save_window(ui: &mut egui::Ui, patch: &mut Patch, state: &mut HeaderState) {
    if !state.save_open {
        return;
    }
    let mut open = state.save_open;
    egui::Window::new("SAVE PRESET")
        .open(&mut open)
        .default_width(340.0)
        .show(ui, |ui| {
            egui::Grid::new("save_fields")
                .num_columns(2)
                .spacing(egui::vec2(8.0, 6.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("NAME").size(10.0).color(th::TEXT_DIM));
                    ui.text_edit_singleline(&mut state.save_name);
                    ui.end_row();
                    ui.label(
                        egui::RichText::new("CATEGORY")
                            .size(10.0)
                            .color(th::TEXT_DIM),
                    );
                    ui.text_edit_singleline(&mut state.save_category);
                    ui.end_row();
                    ui.label(egui::RichText::new("AUTHOR").size(10.0).color(th::TEXT_DIM));
                    ui.text_edit_singleline(&mut state.save_author);
                    ui.end_row();
                });
            ui.add_space(4.0);
            if ui.button("SAVE TO USER PRESETS").clicked() {
                perform_save(patch, state);
            }
            if let Some(status) = &state.save_status {
                ui.label(egui::RichText::new(status).size(10.0).color(th::CV));
            }
        });
    state.save_open = open;
}

fn perform_save(patch: &mut Patch, state: &mut HeaderState) {
    match preset_bank::preset_root() {
        Some(root) => {
            match preset_bank::save_preset(
                &root,
                &state.save_category,
                &state.save_name,
                &state.save_author,
                patch,
            ) {
                Ok(path) => {
                    state.rescan();
                    state.current = state
                        .listing
                        .iter()
                        .find(|entry| entry.path == path)
                        .cloned();
                    state.save_status = Some("Saved".to_owned());
                }
                Err(error) => state.save_status = Some(format!("Save failed: {error}")),
            }
        }
        None => {
            state.save_status = Some("Preset folder unavailable on this platform".to_owned());
        }
    }
}
