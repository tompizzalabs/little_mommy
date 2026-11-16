use crate::engine::node::{Node, NodeParams};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const GRID_COLS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub from: u32,
    pub to: u32,
    pub port: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModLink {
    pub param: u8,
    pub source: u32,
    pub depth: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModEdge {
    pub from: u32,
    pub to: u32,
    pub param: u8,
    pub depth: f32,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub mod_edges: Vec<ModEdge>,
    next_id: u32,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            mod_edges: Vec::new(),
            next_id: 0,
        }
    }

    pub fn add(&mut self, params: NodeParams) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.nodes.push(Node { id, params });
        id
    }

    pub fn upsert(&mut self, id: u32, params: NodeParams) {
        if let Some(node) = self.nodes.iter_mut().find(|node| node.id == id) {
            node.params = params;
            return;
        }
        self.nodes.push(Node { id, params });
        self.next_id = self.next_id.max(id + 1);
    }

    pub fn connect(&mut self, from: u32, to: u32, port: u8) {
        if from == to {
            return;
        }
        self.edges
            .retain(|edge| !(edge.to == to && edge.port == port));
        self.edges.push(Edge { from, to, port });
    }

    pub fn connect_mod(&mut self, from: u32, to: u32, param: u8, depth: f32) {
        if from == to {
            return;
        }
        self.mod_edges
            .retain(|edge| !(edge.to == to && edge.param == param));
        self.mod_edges.push(ModEdge {
            from,
            to,
            param,
            depth,
        });
    }

    pub fn remove(&mut self, id: u32) {
        self.nodes.retain(|node| node.id != id);
        self.edges.retain(|edge| edge.from != id && edge.to != id);
        self.mod_edges
            .retain(|edge| edge.from != id && edge.to != id);
    }

    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InputSel {
    None,
    Prev,
    Slot(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub params: Option<NodeParams>,
    pub in0: InputSel,
    pub in1: InputSel,
    pub mods: Vec<ModLink>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            params: None,
            in0: InputSel::Prev,
            in1: InputSel::None,
            mods: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub cells: Vec<Cell>,
}

impl Grid {
    pub fn new() -> Self {
        Self { cells: Vec::new() }
    }

    fn ensure(&mut self, slot: usize) {
        if self.cells.len() <= slot {
            self.cells.resize(slot + 1, Cell::default());
        }
    }

    pub fn insert(&mut self, params: NodeParams, in0: InputSel, in1: InputSel) -> usize {
        let slot = self
            .cells
            .iter()
            .position(|cell| cell.params.is_none())
            .unwrap_or(self.cells.len());
        self.ensure(slot);
        let cell = &mut self.cells[slot];
        cell.params = Some(params);
        cell.in0 = in0;
        cell.in1 = in1;
        cell.mods.clear();
        slot
    }

    pub fn set(&mut self, slot: usize, params: Option<NodeParams>, in0: InputSel, in1: InputSel) {
        self.ensure(slot);
        let cell = &mut self.cells[slot];
        cell.params = params;
        cell.in0 = in0;
        cell.in1 = in1;
    }

    pub fn resolve(&self, slot: usize, selection: InputSel) -> Option<u32> {
        match selection {
            InputSel::None => None,
            InputSel::Slot(source) => {
                let source = source as usize;
                if self
                    .cells
                    .get(source)
                    .and_then(|cell| cell.params.as_ref())
                    .is_some()
                {
                    Some(source as u32)
                } else {
                    None
                }
            }
            InputSel::Prev => {
                let row_start = (slot / GRID_COLS) * GRID_COLS;
                let mut candidate = slot;
                while candidate > row_start {
                    candidate -= 1;
                    if self.cells[candidate].params.is_some() {
                        return Some(candidate as u32);
                    }
                }
                None
            }
        }
    }

    pub fn to_graph(&self) -> Graph {
        let mut graph = Graph::new();

        for (slot, cell) in self.cells.iter().enumerate() {
            if let Some(params) = &cell.params {
                graph.upsert(slot as u32, params.clone());
            }
        }

        for (slot, cell) in self.cells.iter().enumerate() {
            if cell.params.is_none() {
                continue;
            }
            if let Some(source) = self.resolve(slot, cell.in0) {
                graph.connect(source, slot as u32, 0);
            }
            if let Some(source) = self.resolve(slot, cell.in1) {
                graph.connect(source, slot as u32, 1);
            }
            for link in cell.mods.iter() {
                let source = link.source as usize;
                if self
                    .cells
                    .get(source)
                    .and_then(|cell| cell.params.as_ref())
                    .is_some()
                {
                    graph.connect_mod(source as u32, slot as u32, link.param, link.depth);
                }
            }
        }

        graph
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CompiledNode {
    pub params: NodeParams,
    pub inputs: [Option<u32>; 2],
    pub mod_links: Vec<ModLink>,
}

#[derive(Debug, Clone)]
pub struct CompiledGraph {
    pub nodes: Vec<CompiledNode>,
    pub order: Vec<usize>,
}

impl CompiledGraph {
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            order: Vec::new(),
        }
    }

    pub fn compile(graph: &Graph) -> Self {
        let mut memo: HashMap<u32, u32> = HashMap::new();
        let mut stack: Vec<u32> = Vec::new();

        let mut depths: Vec<(u32, u32)> = graph
            .nodes
            .iter()
            .map(|node| (node.id, depth_of(graph, node.id, &mut stack, &mut memo)))
            .collect();

        depths.sort_by_key(|&(id, depth)| (depth, id));

        let mut slot_by_node_id: HashMap<u32, usize> = HashMap::new();
        for (slot, (id, _)) in depths.iter().enumerate() {
            slot_by_node_id.insert(*id, slot);
        }

        let mut nodes = Vec::with_capacity(depths.len());
        for (id, _) in depths.iter() {
            let node = match graph.node(*id) {
                Some(node) => node,
                None => continue,
            };

            let mut inputs: [Option<u32>; 2] = [None, None];
            for edge in graph.edges.iter().filter(|edge| edge.to == *id) {
                let port = edge.port as usize;
                if port < 2
                    && let Some(&slot) = slot_by_node_id.get(&edge.from)
                {
                    inputs[port] = Some(slot as u32);
                }
            }

            let mut mod_links = Vec::new();
            for edge in graph.mod_edges.iter().filter(|edge| edge.to == *id) {
                if let Some(&source_slot) = slot_by_node_id.get(&edge.from) {
                    mod_links.push(ModLink {
                        param: edge.param,
                        source: source_slot as u32,
                        depth: edge.depth,
                    });
                }
            }

            nodes.push(CompiledNode {
                params: node.params.clone(),
                inputs,
                mod_links,
            });
        }

        let order: Vec<usize> = (0..nodes.len()).collect();

        Self { nodes, order }
    }
}

fn depth_of(graph: &Graph, id: u32, stack: &mut Vec<u32>, memo: &mut HashMap<u32, u32>) -> u32 {
    if let Some(&depth) = memo.get(&id) {
        return depth;
    }
    if stack.contains(&id) {
        return 0;
    }
    stack.push(id);
    let mut depth = 0;
    for edge in graph.edges.iter().filter(|edge| edge.to == id) {
        depth = depth.max(depth_of(graph, edge.from, stack, memo) + 1);
    }
    for edge in graph.mod_edges.iter().filter(|edge| edge.to == id) {
        depth = depth.max(depth_of(graph, edge.from, stack, memo) + 1);
    }
    stack.pop();
    memo.insert(id, depth);
    depth
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlayMode {
    #[default]
    Poly,
    MonoLegato,
    MonoRetrig,
}

impl PlayMode {
    pub const ALL: [PlayMode; 3] = [PlayMode::Poly, PlayMode::MonoLegato, PlayMode::MonoRetrig];
}

impl std::fmt::Display for PlayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            PlayMode::Poly => "Poly",
            PlayMode::MonoLegato => "Mono Legato",
            PlayMode::MonoRetrig => "Mono Retrig",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub grid: Grid,
    #[serde(default)]
    pub play_mode: PlayMode,
    #[serde(default)]
    pub version: u64,
}

impl Patch {
    pub fn new(grid: Grid) -> Self {
        Self {
            grid,
            play_mode: PlayMode::default(),
            version: 1,
        }
    }
}
