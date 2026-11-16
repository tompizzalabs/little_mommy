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

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
