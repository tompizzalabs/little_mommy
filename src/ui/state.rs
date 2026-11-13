use super::theme::{HEADER_H, NODE_W};
use crate::engine::graph::Patch;
use nice_plug::context::gui::GuiContext;

pub const GRID_CELLS: usize = 16;
pub const MAX_MOD_PARAMS: usize = 24;

const LAYOUT_COL_W: f32 = 300.0;
const LAYOUT_ROW_H: f32 = 230.0;
const LAYOUT_X0: f32 = 80.0;
const LAYOUT_Y0: f32 = 90.0;
const DEFAULT_VIEW_W: f32 = 1720.0;
const DEFAULT_VIEW_H: f32 = 1080.0;
const EST_SIZE: egui::Vec2 = egui::vec2(NODE_W, 210.0);
const EST_FALLBACK: egui::Vec2 = egui::vec2(NODE_W, 210.0);

#[derive(Clone, Copy, PartialEq)]
pub enum DragState {
    None,
    Cable { from: usize, current: egui::Pos2 },
    ModCable { from: usize, current: egui::Pos2 },
}

#[derive(Clone, Copy)]
pub struct NodeGeom {
    pub rect: egui::Rect,
    pub mod_in: [egui::Pos2; MAX_MOD_PARAMS],
    pub mod_in_n: usize,
    pub valid: bool,
}

impl Default for NodeGeom {
    fn default() -> Self {
        Self {
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, EST_FALLBACK),
            mod_in: [egui::Pos2::ZERO; MAX_MOD_PARAMS],
            mod_in_n: 0,
            valid: false,
        }
    }
}

impl NodeGeom {
    #[inline(always)]
    pub fn audio_in(self, port: usize) -> egui::Pos2 {
        egui::pos2(
            self.rect.left(),
            self.rect.top() + HEADER_H + 18.0 + port as f32 * 22.0,
        )
    }

    #[inline(always)]
    pub fn audio_out(self) -> egui::Pos2 {
        egui::pos2(self.rect.right(), self.rect.top() + HEADER_H + 18.0)
    }

    #[inline(always)]
    pub fn mod_out(self) -> egui::Pos2 {
        egui::pos2(self.rect.right(), self.rect.top() + HEADER_H * 0.5)
    }

    #[inline]
    pub fn translated(mut self, delta: egui::Vec2) -> Self {
        self.rect = self.rect.translate(delta);
        for p in &mut self.mod_in[..self.mod_in_n] {
            *p += delta;
        }
        self
    }
}

pub fn content_bbox(patch: &Patch, positions: &[egui::Pos2]) -> egui::Rect {
    let mut bbox = egui::Rect::NOTHING;
    let mut any = false;
    for (slot, cell) in patch.grid.cells.iter().enumerate() {
        if cell.params.is_some() {
            bbox = bbox.union(egui::Rect::from_min_size(positions[slot], EST_SIZE));
            any = true;
        }
    }
    let content = if any {
        bbox
    } else {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0))
    };
    egui::Rect::from_center_size(content.center(), egui::vec2(DEFAULT_VIEW_W, DEFAULT_VIEW_H))
}

pub struct OpenEditorState {
    pub nice_gui_ctx: GuiContext,
}

impl OpenEditorState {
    pub fn new(nice_gui_ctx: GuiContext) -> Self {
        Self { nice_gui_ctx }
    }
}

pub fn auto_layout(patch: &Patch, positions: &mut [egui::Pos2]) {
    let n = patch.grid.cells.len().min(positions.len()).min(GRID_CELLS);
    let cap = n.saturating_sub(1);
    let mut depth = [0usize; GRID_CELLS];

    for _ in 0..n {
        let mut changed = false;
        for slot in 0..n {
            let cell = &patch.grid.cells[slot];
            let Some(params) = cell.params.as_ref() else {
                continue;
            };
            let inputs = params.input_count();
            let mut d = 0usize;
            for (port, sel) in [(0usize, cell.in0), (1usize, cell.in1)] {
                if port >= inputs {
                    continue;
                }
                if let Some(src) = patch.grid.resolve(slot, sel) {
                    let src = src as usize;
                    if src < n && src != slot {
                        d = d.max(depth[src] + 1);
                    }
                }
            }
            let d = d.min(cap);
            if d > depth[slot] {
                depth[slot] = d;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut counts = [0usize; GRID_CELLS];
    let mut rows = [0usize; GRID_CELLS];
    for slot in 0..n {
        if patch.grid.cells[slot].params.is_some() {
            counts[depth[slot]] += 1;
        }
    }
    for slot in 0..n {
        if patch.grid.cells[slot].params.is_none() {
            continue;
        }
        let col = depth[slot];
        let total = counts[col] as f32;
        let idx = rows[col] as f32;
        rows[col] += 1;
        let y = LAYOUT_Y0 + idx * LAYOUT_ROW_H - (total - 1.0) * 0.5 * LAYOUT_ROW_H;
        positions[slot] = egui::pos2(LAYOUT_X0 + col as f32 * LAYOUT_COL_W, y.max(40.0));
    }
}
