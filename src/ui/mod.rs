mod canvas;
mod header;
mod knob;
mod preview;
mod shape;
mod state;
mod theme;
mod widgets;

use crate::engine::graph::Patch;
use crate::params::LittleMommyParams;
use nice_plug::context::gui::GuiContext;
use nice_plug::editor::dpi::LogicalSize;
use nice_plug::prelude::*;
use nice_plug_egui::{
    EguiEditor, EguiEditorState, EguiNiceSettings, NiceEguiApp, RepaintNotifier,
    create_egui_editor, resizable_window::ResizableWindow,
};
use state::{DragState, NodeGeom, OpenEditorState, ensure_scratch};
use std::sync::{Arc, Mutex};

pub const MIN_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(1040.0, 660.0);
pub const INITIAL_SCALE_FACTOR: f32 = 1.0;
const RESIZE_HINT: ResizeHint = ResizeHint::resizable().with_min_logical_size(MIN_WINDOW_SIZE);
const MIN_SIZE: egui::Vec2 = egui::vec2(MIN_WINDOW_SIZE.width, MIN_WINDOW_SIZE.height);
const FONT_NAME: &str = "MPLUS2";

static CUSTOM_FONT: &[u8] = include_bytes!("../../assets/fonts/MPLUS2-VariableFont_wght.ttf");

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        FONT_NAME.to_owned(),
        Arc::new(egui::FontData::from_static(CUSTOM_FONT)),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, FONT_NAME.to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(FONT_NAME.to_owned());
    ctx.set_fonts(fonts);
}

fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    let visuals = &mut style.visuals;
    *visuals = egui::Theme::Dark.default_visuals();
    visuals.override_text_color = Some(theme::TEXT_BRIGHT);
    visuals.panel_fill = theme::CANVAS_BG;
    visuals.window_fill = theme::SURFACE_HI;
    visuals.window_stroke = egui::Stroke::new(1.0, theme::NODE_EDGE);
    visuals.window_corner_radius = 8.into();
    visuals.menu_corner_radius = 8.into();
    visuals.extreme_bg_color = theme::INSET_BG;
    visuals.faint_bg_color = theme::SURFACE;
    visuals.code_bg_color = theme::INSET_BG;
    visuals.hyperlink_color = theme::CV;
    visuals.selection.bg_fill = theme::alpha(theme::CV, 70);
    visuals.selection.stroke = egui::Stroke::new(1.0, theme::CV);
    visuals.warn_fg_color = theme::WARN;
    visuals.error_fg_color = theme::AUDIO;

    let widgets = &mut visuals.widgets;

    widgets.noninteractive.bg_fill = theme::SURFACE;
    widgets.noninteractive.weak_bg_fill = theme::SURFACE;
    widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, theme::INSET_EDGE);
    widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, theme::TEXT_DIM);
    widgets.noninteractive.corner_radius = 4.into();

    widgets.inactive.bg_fill = theme::WIDGET;
    widgets.inactive.weak_bg_fill = theme::WIDGET;
    widgets.inactive.bg_stroke = egui::Stroke::new(1.0, theme::INSET_EDGE);
    widgets.inactive.fg_stroke = egui::Stroke::new(1.0, theme::TEXT_DIM);
    widgets.inactive.corner_radius = 5.into();

    widgets.hovered.bg_fill = theme::WIDGET_HOT;
    widgets.hovered.weak_bg_fill = theme::WIDGET_HOT;
    widgets.hovered.bg_stroke = egui::Stroke::new(1.0, theme::CV);
    widgets.hovered.fg_stroke = egui::Stroke::new(1.4, theme::TEXT_BRIGHT);
    widgets.hovered.corner_radius = 5.into();

    widgets.active.bg_fill = theme::WIDGET_ON;
    widgets.active.weak_bg_fill = theme::WIDGET_ON;
    widgets.active.bg_stroke = egui::Stroke::new(1.0, theme::CV);
    widgets.active.fg_stroke = egui::Stroke::new(1.4, theme::TEXT_BRIGHT);
    widgets.active.corner_radius = 5.into();

    widgets.open.bg_fill = theme::SURFACE_HI;
    widgets.open.weak_bg_fill = theme::SURFACE_HI;
    widgets.open.bg_stroke = egui::Stroke::new(1.0, theme::NODE_EDGE);
    widgets.open.fg_stroke = egui::Stroke::new(1.0, theme::TEXT_BRIGHT);

    style.spacing.item_spacing = egui::vec2(6.0, 6.0);
    style.spacing.button_padding = egui::vec2(9.0, 5.0);
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.window_margin = egui::Margin::same(8);
    style.spacing.scroll.bar_width = 7.0;
    style.spacing.scroll.foreground_color = false;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.spacing.scroll.bar_outer_margin = 2.0;
    ctx.set_style_of(egui::Theme::Dark, style);
    ctx.set_theme(egui::Theme::Dark);
}

pub struct LittleMommyEditor {
    open_state: Option<OpenEditorState>,
    params: Arc<LittleMommyParams>,
    patch: Arc<Mutex<Patch>>,
    positions: Vec<egui::Pos2>,
    geom: Vec<NodeGeom>,
    selected: Option<usize>,
    drag: DragState,
    header_state: header::HeaderState,
    scene_rect: egui::Rect,
    context_menu_pos: Option<egui::Pos2>,
    context_menu_world: Option<egui::Pos2>,
}

impl LittleMommyEditor {
    pub fn new(params: Arc<LittleMommyParams>, patch: Arc<Mutex<Patch>>) -> Self {
        let mut positions = Vec::new();
        let mut geom = Vec::new();
        let scene_rect = {
            let guard = patch.lock().unwrap();
            ensure_scratch(&guard, &mut positions, &mut geom);
            state::auto_layout(&guard, &mut positions);
            state::content_bbox(&guard, &positions)
        };
        Self {
            open_state: None,
            params,
            patch,
            positions,
            geom,
            selected: None,
            drag: DragState::None,
            header_state: header::HeaderState::default(),
            scene_rect,
            context_menu_pos: None,
            context_menu_world: None,
        }
    }
}

impl NiceEguiApp for LittleMommyEditor {
    fn build(
        &mut self,
        egui_ctx: egui::Context,
        nice_gui_ctx: GuiContext,
        _frame: &mut nice_plug_egui::Frame,
    ) -> Result<(), nice_plug_egui::baseview::HandlerError> {
        install_fonts(&egui_ctx);
        apply_theme(&egui_ctx);
        let _ = crate::preset_bank::install_community_presets();
        self.open_state = Some(OpenEditorState::new(nice_gui_ctx));
        Ok(())
    }

    fn editor_closed(&mut self) {
        self.open_state = None;
    }

    fn track_info_changed(&mut self, _info: TrackInfo) {}

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut nice_plug_egui::Frame) {
        let Some(open_state) = self.open_state.as_mut() else {
            return;
        };
        let setter = open_state.nice_gui_ctx.param_setter();
        let Ok(mut patch) = self.patch.lock() else {
            return;
        };
        let params = &self.params;
        let scratch_grew = self.positions.len() < patch.grid.cells.len();
        ensure_scratch(&patch, &mut self.positions, &mut self.geom);
        if scratch_grew {
            state::auto_layout(&patch, &mut self.positions);
        }

        ResizableWindow::new("Shade-window")
            .min_size(MIN_SIZE)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                ui.vertical(|ui| {
                    header::top_bar(
                        ui,
                        &mut patch,
                        &mut self.positions,
                        &mut self.scene_rect,
                        &mut self.header_state,
                        &mut self.selected,
                        params,
                        &setter,
                    );
                    canvas::canvas(
                        ui,
                        &mut patch,
                        &mut self.positions,
                        &mut self.selected,
                        &mut self.drag,
                        &mut self.scene_rect,
                        &mut self.geom,
                        &mut self.context_menu_pos,
                        &mut self.context_menu_world,
                    );
                });
            });
        header::browser_window(
            ui,
            &mut patch,
            &mut self.positions,
            &mut self.selected,
            &mut self.scene_rect,
            &mut self.header_state,
        );
        header::save_window(ui, &mut patch, &mut self.header_state);
    }
}

pub fn build_editor(
    editor_state: Arc<EguiEditorState>,
    repaint: RepaintNotifier,
    editor: LittleMommyEditor,
) -> Option<EguiEditor<LittleMommyEditor>> {
    create_egui_editor(
        editor_state,
        repaint,
        EguiNiceSettings::new().with_resize_hint(RESIZE_HINT),
        editor,
    )
}
