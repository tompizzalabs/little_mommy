use super::wavetable::{TableSource, WtVoice};

pub const VECTOR_EDGES: usize = 4;

const VECTOR_SMOOTH: f32 = 0.001;

#[derive(Debug, Clone)]
pub struct VectorState {
    pub voice: WtVoice,
    pub x_smooth: f32,
    pub y_smooth: f32,
}

impl Default for VectorState {
    fn default() -> Self {
        Self {
            voice: WtVoice::default(),
            x_smooth: 0.0,
            y_smooth: 0.0,
        }
    }
}

impl VectorState {
    pub fn snap_xy(&mut self, x: f32, y: f32) {
        self.x_smooth = x;
        self.y_smooth = y;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn next(
        &mut self,
        s0: &TableSource,
        s1: &TableSource,
        s2: &TableSource,
        s3: &TableSource,
        p0: f32,
        p1: f32,
        p2: f32,
        p3: f32,
        freq: f32,
        sample_rate: f32,
        x: f32,
        y: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }

        self.x_smooth += (x - self.x_smooth) * VECTOR_SMOOTH;
        self.y_smooth += (y - self.y_smooth) * VECTOR_SMOOTH;
        let x_modded = self.x_smooth.clamp(0.0, 1.0);
        let y_modded = self.y_smooth.clamp(0.0, 1.0);

        let oversampled_rate = sample_rate * 3.0;
        let mut up = [0.0f32; 3];
        for slot in up.iter_mut() {
            let e0 = s0.sample(self.voice.phase, p0);
            let e1 = s1.sample(self.voice.phase, p1);
            let e2 = s2.sample(self.voice.phase, p2);
            let e3 = s3.sample(self.voice.phase, p3);

            let lower = (1.0 - x_modded) * e0 + x_modded * e3;
            let upper = (1.0 - x_modded) * e1 + x_modded * e2;

            *slot = (1.0 - y_modded) * lower + y_modded * upper;

            self.voice.phase += freq / oversampled_rate;
            while self.voice.phase >= 1.0 {
                self.voice.phase -= 1.0;
            }
        }

        self.voice.aa.process(up)
    }
}
