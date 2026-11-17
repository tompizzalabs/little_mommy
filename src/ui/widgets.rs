use crate::engine::node::{NodeKind, NodeParams};

pub const KIND_LIST: [NodeKind; 20] = [
    NodeKind::Analog,
    NodeKind::Noise,
    NodeKind::Fm,
    NodeKind::Pm,
    NodeKind::Vector,
    NodeKind::Mix,
    NodeKind::Filter,
    NodeKind::Env,
    NodeKind::Gain,
    NodeKind::Lfo,
    NodeKind::Delay,
    NodeKind::Chorus,
    NodeKind::Flanger,
    NodeKind::Phaser,
    NodeKind::Bitcrusher,
    NodeKind::RingMod,
    NodeKind::Distortion,
    NodeKind::Equalizer,
    NodeKind::Reverb,
    NodeKind::Out,
];

pub fn kind_color(params: Option<&NodeParams>) -> egui::Color32 {
    match params.map(|params| params.kind()) {
        None => egui::Color32::from_rgb(70, 70, 78),
        Some(NodeKind::Analog) => egui::Color32::from_rgb(70, 110, 170),
        Some(NodeKind::Noise) => egui::Color32::from_rgb(110, 110, 118),
        Some(NodeKind::Fm) => egui::Color32::from_rgb(180, 100, 130),
        Some(NodeKind::Pm) => egui::Color32::from_rgb(160, 110, 190),
        Some(NodeKind::Vector) => egui::Color32::from_rgb(110, 120, 200),
        Some(NodeKind::Mix) => egui::Color32::from_rgb(80, 140, 95),
        Some(NodeKind::Filter) => egui::Color32::from_rgb(190, 130, 60),
        Some(NodeKind::Env) => egui::Color32::from_rgb(140, 80, 150),
        Some(NodeKind::Gain) => egui::Color32::from_rgb(90, 150, 150),
        Some(NodeKind::Out) => egui::Color32::from_rgb(180, 70, 70),
        Some(NodeKind::Lfo) => egui::Color32::from_rgb(70, 160, 120),
        Some(NodeKind::Delay) => egui::Color32::from_rgb(150, 140, 60),
        Some(NodeKind::Chorus) => egui::Color32::from_rgb(60, 150, 160),
        Some(NodeKind::Flanger) => egui::Color32::from_rgb(60, 130, 180),
        Some(NodeKind::Phaser) => egui::Color32::from_rgb(120, 100, 170),
        Some(NodeKind::Bitcrusher) => egui::Color32::from_rgb(160, 90, 70),
        Some(NodeKind::RingMod) => egui::Color32::from_rgb(170, 80, 110),
        Some(NodeKind::Distortion) => egui::Color32::from_rgb(170, 70, 60),
        Some(NodeKind::Equalizer) => egui::Color32::from_rgb(100, 150, 80),
        Some(NodeKind::Reverb) => egui::Color32::from_rgb(80, 120, 170),
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
            NodeKind::Vector => "VECTOR",
            NodeKind::Mix => "MIX",
            NodeKind::Filter => "FILTER",
            NodeKind::Env => "ENV",
            NodeKind::Gain => "GAIN",
            NodeKind::Out => "OUT",
            NodeKind::Lfo => "LFO",
            NodeKind::Delay => "DELAY",
            NodeKind::Chorus => "CHORUS",
            NodeKind::Flanger => "FLANGER",
            NodeKind::Phaser => "PHASER",
            NodeKind::Bitcrusher => "CRUSH",
            NodeKind::RingMod => "RINGMOD",
            NodeKind::Distortion => "DISTORT",
            NodeKind::Equalizer => "EQ",
            NodeKind::Reverb => "VERB",
        }
        .to_string(),
    }
}
