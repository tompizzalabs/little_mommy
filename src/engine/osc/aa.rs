//! Anti-aliasing chain ported from Odin 2 `WavetableOsc1D::doOscillateWithSync`:
//! 3x oversampling, fixed-coefficient IIR decimation, DC blocking.

#[derive(Debug, Clone, Copy)]
pub struct DecimationFilter {
    xv: [f64; 10],
    yv: [f64; 10],
}

impl Default for DecimationFilter {
    fn default() -> Self {
        Self {
            xv: [0.0; 10],
            yv: [0.0; 10],
        }
    }
}

impl DecimationFilter {
    fn push(&mut self, input: f64) {
        self.xv.copy_within(1..10, 0);
        self.xv[9] = input * 0.019966841051093;

        self.yv.copy_within(1..10, 0);
        self.yv[9] = (self.xv[0] + self.xv[9])
            + 9.0 * (self.xv[1] + self.xv[8])
            + 36.0 * (self.xv[2] + self.xv[7])
            + 84.0 * (self.xv[3] + self.xv[6])
            + 126.0 * (self.xv[4] + self.xv[5])
            + (-0.0003977153 * self.yv[0])
            + (-0.0064474617 * self.yv[1])
            + (-0.0476997403 * self.yv[2])
            + (-0.2185829743 * self.yv[3])
            + (-0.6649234123 * self.yv[4])
            + (-1.4773657709 * self.yv[5])
            + (-2.2721421641 * self.yv[6])
            + (-2.6598673212 * self.yv[7])
            + (-1.8755960587 * self.yv[8]);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DcBlocker {
    x1: f32,
    y1: f32,
}

impl DcBlocker {
    pub fn process(&mut self, input: f32) -> f32 {
        let output = input - self.x1 + 0.999 * self.y1;
        self.x1 = input;
        self.y1 = output;
        output
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AntiAlias {
    decimation: DecimationFilter,
    dc: DcBlocker,
}

impl AntiAlias {
    /// Consumes three oversampled values, returns one decimated sample.
    pub fn process(&mut self, up: [f32; 3]) -> f32 {
        self.decimation.push(up[0] as f64);
        self.decimation.push(up[1] as f64);
        self.decimation.push(up[2] as f64);
        self.dc.process(self.decimation.yv[9] as f32)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

