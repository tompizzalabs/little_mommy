//! Reusable widgets and label/color formatting helpers.

use crate::engine::node::{NodeKind, NodeParams};

pub const KIND_LIST: [NodeKind; 10] = [
    NodeKind::Analog,
    NodeKind::Noise,
    NodeKind::Fm,
    NodeKind::Pm,
    NodeKind::Mix,
    NodeKind::Filter,
    NodeKind::Env,
    NodeKind::Gain,
    NodeKind::Lfo,
    NodeKind::Out,
];

pub fn kind_color(params: Option<&NodeParams>) -> egui::Color32 {
    match params.map(|p| p.kind()) {
        None => egui::Color32::from_rgb(70, 70, 78),
        Some(NodeKind::Analog) => egui::Color32::from_rgb(70, 110, 170),
        Some(NodeKind::Noise) => egui::Color32::from_rgb(110, 110, 118),
        Some(NodeKind::Fm) => egui::Color32::from_rgb(180, 100, 130),
        Some(NodeKind::Pm) => egui::Color32::from_rgb(160, 110, 190),
        Some(NodeKind::Mix) => egui::Color32::from_rgb(80, 140, 95),
        Some(NodeKind::Filter) => egui::Color32::from_rgb(190, 130, 60),
        Some(NodeKind::Env) => egui::Color32::from_rgb(140, 80, 150),
        Some(NodeKind::Gain) => egui::Color32::from_rgb(90, 150, 150),
        Some(NodeKind::Out) => egui::Color32::from_rgb(180, 70, 70),
        Some(NodeKind::Lfo) => egui::Color32::from_rgb(70, 160, 120),
    }
}

pub fn kind_label(kind: Option<NodeKind>) -> String {
    match kind {
        None => "--".to_string(),
        Some(k) => match k {
            NodeKind::Analog => "ANALOG",
            NodeKind::Noise => "NOISE",
            NodeKind::Fm => "FM",
            NodeKind::Pm => "PM",
            NodeKind::Mix => "MIX",
            NodeKind::Filter => "FILTER",
            NodeKind::Env => "ENV",
            NodeKind::Gain => "GAIN",
            NodeKind::Out => "OUT",
            NodeKind::Lfo => "LFO",
        }
        .to_string(),
    }
}
