use crate::engine::graph::Patch;
use crate::presets;
use nice_plug::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Params)]
pub struct LittleMommyParams {
    #[id = "master_gain"]
    pub master_gain: FloatParam,
    #[persist = "patch"]
    pub patch: Arc<Mutex<Patch>>,
}

impl Default for LittleMommyParams {
    fn default() -> Self {
        Self {
            master_gain: FloatParam::new(
                "Master Gain",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(5.0)),
            patch: Arc::new(Mutex::new(presets::default_patch())),
        }
    }
}
