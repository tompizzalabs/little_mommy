use super::wavetable::{TableSource, WtVoice};
use crate::engine::arpeggiator::{ChiptuneArpeggiator, pitch_shift_multiplier};
use fastrand::Rng;

const WAVETABLE_LENGTH: f32 = 512.0;
const NOISE_SEGMENT_LENGTH_3X: f32 = 90.0;

#[derive(Debug, Clone)]
pub struct ChiptuneState {
    pub voice: WtVoice,
    pub arp: ChiptuneArpeggiator,
    pub generate_noise: bool,
    pub noise_index: f32,
    pub last_noise_value: f32,
    pub xv: [f32; 10],
    pub yv: [f32; 10],
    rng: Rng,
}

impl Default for ChiptuneState {
    fn default() -> Self {
        Self {
            voice: WtVoice::default(),
            arp: ChiptuneArpeggiator::default(),
            generate_noise: false,
            noise_index: 0.0,
            last_noise_value: 0.0,
            xv: [0.0; 10],
            yv: [0.0; 10],
            rng: Rng::with_seed(0x51_7C_C1_B7),
        }
    }
}

impl ChiptuneState {
    pub fn seed(&mut self, rng: Rng) {
        self.rng = rng;
    }

    pub fn set_noise_mode(&mut self, noise: bool) {
        if noise != self.generate_noise {
            self.generate_noise = noise;
            self.noise_index = 0.0;
            self.xv = [0.0; 10];
            self.yv = [0.0; 10];
        }
    }

    pub fn next(
        &mut self,
        source: &TableSource,
        freq: f32,
        sample_rate: f32,
        position: f32,
    ) -> f32 {
        if sample_rate <= 0.0 {
            return 0.0;
        }
        let arp_semitones = self.arp.do_arpeggiator(0.0, sample_rate) as f32;
        let freq = freq * pitch_shift_multiplier(arp_semitones);

        if self.generate_noise {
            self.generate_chip_noise(freq, sample_rate)
        } else {
            self.voice.next(source, position, freq, sample_rate)
        }
    }

    fn generate_chip_noise(&mut self, freq: f32, sample_rate: f32) -> f32 {
        let inc = freq / sample_rate * WAVETABLE_LENGTH;
        self.noise_index += inc;
        if self.noise_index > NOISE_SEGMENT_LENGTH_3X {
            self.noise_index = 0.0;
            // Odin: `std::rand() % 16 - 8` then `* 0.125f` -> multiples of 0.125 in [-1, 7/8].
            let raw = self.rng.u32(0..16) as f32 - 8.0;
            self.last_noise_value = raw * 0.125;
        }

        for _ in 0..3 {
            self.push_last_noise();
        }
        self.yv[9]
    }

    fn push_last_noise(&mut self) {
        self.xv.copy_within(1..10, 0);
        self.xv[9] = self.last_noise_value * 0.019966841051093;
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
