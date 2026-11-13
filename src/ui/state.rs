use crate::engine::graph::Patch;
use nice_plug::context::gui::GuiContext;

pub const GRID_CELLS: usize = 16;

/// Spacing for the automatic node layout.
const LAYOUT_COL_W: f32 = 240.0;
const LAYOUT_ROW_H: f32 = 180.0;
const LAYOUT_X0: f32 = 60.0;
const LAYOUT_Y0: f32 = 80.0;

#[derive(Clone, Copy, PartialEq)]
pub enum DragState {
    None,
    Cable { from: usize, current: egui::Pos2 },
    ModCable { from: usize, current: egui::Pos2 },
}

/// Bounding rectangle of all active nodes (in world coordinates), plus a small
/// margin. Used to initialize/reset the `Scene` view so the patch fits on screen.
pub fn content_bbox(patch: &Patch, positions: &[egui::Pos2]) -> egui::Rect {
    // The default view is a fixed-size world rect centered on the patch. It is
    // deliberately larger than the window, so the initial zoom is zoomed-out
    // (roughly 60%) instead of blown up to fill the screen.
    const DEFAULT_VIEW_W: f32 = 1600.0;
    const DEFAULT_VIEW_H: f32 = 1000.0;

    let mut bbox: Option<egui::Rect> = None;
    for (slot, cell) in patch.grid.cells.iter().enumerate() {
        if cell.params.is_some() {
            let r = egui::Rect::from_min_size(positions[slot], egui::vec2(200.0, 140.0));
            bbox = Some(bbox.map_or(r, |b| b.union(r)));
        }
    }
    let content = bbox
        .unwrap_or_else(|| egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0)));
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

/// Lay out all active cells left-to-right by audio-chain depth so freshly
/// loaded patches (default patch, presets) don't spawn stacked on one spot.
pub fn auto_layout(patch: &Patch, positions: &mut [egui::Pos2]) {
    let n = patch.grid.cells.len().min(positions.len());

    // Longest-path depth from audio sources, converged iteratively.
    // Bounded by `n` iterations, so feedback cycles can't loop forever.
    let mut depth = vec![0usize; n];
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
            if d > depth[slot] {
                depth[slot] = d;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Vertical placement: one row per node, columns centered around LAYOUT_Y0.
    let max_col = depth.iter().copied().max().unwrap_or(0);
    let mut col_counts = vec![0usize; max_col + 1];
    let mut col_rows = vec![0usize; max_col + 1];
    for slot in 0..n {
        if patch.grid.cells[slot].params.is_some() {
            col_counts[depth[slot]] += 1;
        }
    }
    for slot in 0..n {
        if patch.grid.cells[slot].params.is_none() {
            continue;
        }
        let col = depth[slot];
        let rows = col_counts[col] as f32;
        let idx = col_rows[col] as f32;
        col_rows[col] += 1;
        let y = LAYOUT_Y0 + (rows - 1.0) * 0.5 * LAYOUT_ROW_H + idx * LAYOUT_ROW_H;
        positions[slot] = egui::pos2(LAYOUT_X0 + col as f32 * LAYOUT_COL_W, y);
    }
}
