mod canvas;
mod header;
mod preview;
mod state;
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
use state::{DragState, GRID_CELLS, OpenEditorState};
use std::sync::{Arc, Mutex};

pub const MIN_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(960.0, 600.0);
pub const INITIAL_SCALE_FACTOR: f32 = 1.0;
const RESIZE_HINT: ResizeHint = ResizeHint::resizable().with_min_logical_size(MIN_WINDOW_SIZE);

/// Custom UI font shipped with the plugin, embedded at compile time.
static CUSTOM_FONT: &[u8] = include_bytes!("../../assets/fonts/MPLUS2-VariableFont_wght.ttf");

/// Install the bundled font as the primary proportional/monospace font so all
/// UI text renders with it.
fn install_fonts(ctx: &egui::Context) {
    const NAME: &str = "MPLUS2";
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        NAME.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(CUSTOM_FONT)),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, NAME.to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(NAME.to_owned());
    ctx.set_fonts(fonts);
}

pub struct LittleMommyEditor {
    open_state: Option<OpenEditorState>,
    params: Arc<LittleMommyParams>,
    patch: Arc<Mutex<Patch>>,
    positions: Vec<egui::Pos2>,
    selected: Option<usize>,
    drag: DragState,
    preset_index: usize,
    scene_rect: egui::Rect,
    context_menu_pos: Option<egui::Pos2>,
    context_menu_world: Option<egui::Pos2>,
}

impl LittleMommyEditor {
    pub fn new(params: Arc<LittleMommyParams>, patch: Arc<Mutex<Patch>>) -> Self {
        let mut positions = vec![egui::Pos2::ZERO; GRID_CELLS];
        let (positions, scene_rect) = {
            let guard = patch.lock().unwrap();
            state::auto_layout(&guard, &mut positions);
            let scene_rect = state::content_bbox(&guard, &positions);
            (positions, scene_rect)
        };

        Self {
            open_state: None,
            params,
            patch,
            positions,
            selected: None,
            drag: DragState::None,
            preset_index: 0,
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
        let mut patch = self.patch.lock().unwrap();

        let params = &self.params;

        ResizableWindow::new("Shade-window")
            .min_size(egui::vec2(MIN_WINDOW_SIZE.width, MIN_WINDOW_SIZE.height))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Top Header Bar
                    header::top_bar(
                        ui,
                        &mut patch,
                        &mut self.positions,
                        &mut self.scene_rect,
                        &mut self.preset_index,
                        &mut self.selected,
                        params,
                        &setter,
                    );

                    ui.separator();

                    // Canvas
                    canvas::canvas(
                        ui,
                        &mut patch,
                        &mut self.positions,
                        &mut self.selected,
                        &mut self.drag,
                        &mut self.scene_rect,
                        &mut self.context_menu_pos,
                        &mut self.context_menu_world,
                    );
                });
            });
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
