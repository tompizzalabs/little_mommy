use super::theme::{HEADER_H, NODE_W};
use crate::engine::graph::Patch;
use nice_plug::context::gui::GuiContext;

pub const MAX_MOD_PARAMS: usize = 24;

const LAYOUT_COL_W: f32 = 300.0;
const LAYOUT_ROW_H: f32 = 230.0;
const LAYOUT_X0: f32 = 80.0;
const LAYOUT_Y0: f32 = 90.0;
const DEFAULT_VIEW_W: f32 = 1720.0;
const DEFAULT_VIEW_H: f32 = 1080.0;
const ESTIMATED_NODE_SIZE: egui::Vec2 = egui::vec2(NODE_W, 210.0);

#[derive(Clone, Copy, PartialEq)]
pub enum DragState {
    None,
    Cable { from: usize, current: egui::Pos2 },
    ModCable { from: usize, current: egui::Pos2 },
}

#[derive(Clone, Copy)]
pub struct NodeGeom {
    pub rect: egui::Rect,
    pub mod_inputs: [egui::Pos2; MAX_MOD_PARAMS],
    pub mod_input_count: usize,
    pub valid: bool,
}

impl Default for NodeGeom {
    fn default() -> Self {
        Self {
            rect: egui::Rect::from_min_size(egui::Pos2::ZERO, ESTIMATED_NODE_SIZE),
            mod_inputs: [egui::Pos2::ZERO; MAX_MOD_PARAMS],
            mod_input_count: 0,
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
        for position in &mut self.mod_inputs[..self.mod_input_count] {
            *position += delta;
        }
        self
    }
}

pub fn ensure_scratch(patch: &Patch, positions: &mut Vec<egui::Pos2>, geom: &mut Vec<NodeGeom>) {
    let cell_count = patch.grid.cells.len();
    if positions.len() < cell_count {
        positions.resize(cell_count, egui::Pos2::ZERO);
    }
    if geom.len() < cell_count {
        geom.resize(cell_count, NodeGeom::default());
    }
}

pub fn apply_patch(
    patch: &mut Patch,
    positions: &mut Vec<egui::Pos2>,
    selected: &mut Option<usize>,
    scene_rect: &mut egui::Rect,
    replacement: Patch,
) {
    let next_version = patch.version.wrapping_add(1);
    let cell_count = replacement.grid.cells.len();
    *patch = replacement;
    patch.version = next_version;
    *selected = None;
    if positions.len() < cell_count {
        positions.resize(cell_count, egui::Pos2::ZERO);
    }
    auto_layout(patch, positions);
    *scene_rect = content_bbox(patch, positions);
}

pub fn content_bbox(patch: &Patch, positions: &[egui::Pos2]) -> egui::Rect {
    let mut union_bounds = egui::Rect::NOTHING;
    let mut has_content = false;
    for (slot, cell) in patch.grid.cells.iter().enumerate() {
        if cell.params.is_some()
            && let Some(&position) = positions.get(slot)
        {
            union_bounds =
                union_bounds.union(egui::Rect::from_min_size(position, ESTIMATED_NODE_SIZE));
            has_content = true;
        }
    }
    let content = if has_content {
        union_bounds
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
    let cell_count = patch.grid.cells.len().min(positions.len());
    if cell_count == 0 {
        return;
    }
    let max_depth = cell_count.saturating_sub(1);
    let mut depth = vec![0usize; cell_count];

    for _ in 0..cell_count {
        let mut changed = false;
        for slot in 0..cell_count {
            let cell = &patch.grid.cells[slot];
            let Some(params) = cell.params.as_ref() else {
                continue;
            };
            let inputs = params.input_count();
            let mut deepest_input = 0usize;
            for (port, selection) in [(0usize, cell.in0), (1usize, cell.in1)] {
                if port >= inputs {
                    continue;
                }
                if let Some(source) = patch.grid.resolve(slot, selection) {
                    let source = source as usize;
                    if source < cell_count && source != slot {
                        deepest_input = deepest_input.max(depth[source] + 1);
                    }
                }
            }
            let new_depth = deepest_input.min(max_depth);
            if new_depth > depth[slot] {
                depth[slot] = new_depth;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut column_counts = vec![0usize; cell_count];
    let mut column_rows = vec![0usize; cell_count];
    for slot in 0..cell_count {
        if patch.grid.cells[slot].params.is_some() {
            column_counts[depth[slot]] += 1;
        }
    }
    for slot in 0..cell_count {
        if patch.grid.cells[slot].params.is_none() {
            continue;
        }
        let column = depth[slot];
        let total_in_column = column_counts[column] as f32;
        let row_index = column_rows[column] as f32;
        column_rows[column] += 1;
        let y = LAYOUT_Y0 + row_index * LAYOUT_ROW_H - (total_in_column - 1.0) * 0.5 * LAYOUT_ROW_H;
        positions[slot] = egui::pos2(LAYOUT_X0 + column as f32 * LAYOUT_COL_W, y.max(40.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::Grid;

    #[test]
    fn apply_patch_keeps_version_increasing_regardless_of_replacement() {
        let mut patch = Patch::new(Grid::new());
        let mut positions = Vec::new();
        let mut selected = None;
        let mut scene_rect = egui::Rect::NOTHING;

        for _ in 0..3 {
            let previous = patch.version;
            apply_patch(
                &mut patch,
                &mut positions,
                &mut selected,
                &mut scene_rect,
                Patch::new(Grid::new()),
            );
            assert!(patch.version > previous);
            assert_eq!(patch.version, previous + 1);
        }
    }
}
