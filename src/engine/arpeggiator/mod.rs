const STEP_LENGTH: f32 = 1.0;
pub const ARP_STEPS: usize = 3;

#[inline]
pub fn pitch_shift_multiplier(semitones: f32) -> f32 {
    (0.057_762_265 * semitones).exp()
}

#[derive(Debug, Clone)]
pub struct ChiptuneArpeggiator {
    /// Base step rate in Hz (Odin default 20 Hz).
    pub frequency: f32,
    pub step_semitones: [i32; ARP_STEPS],
    pub step_three_active: bool,
    pub arp_active: bool,
    position_in_step: f32,
    current_step: usize,
}

impl Default for ChiptuneArpeggiator {
    fn default() -> Self {
        Self {
            frequency: 20.0,
            step_semitones: [0, 12, 7],
            step_three_active: false,
            arp_active: false,
            position_in_step: 0.0,
            current_step: 0,
        }
    }
}

impl ChiptuneArpeggiator {
    pub fn reset(&mut self) {
        self.current_step = 0;
        self.position_in_step = 0.0;
    }

    pub fn do_arpeggiator(&mut self, freq_mod: f32, sample_rate: f32) -> i32 {
        if !self.arp_active {
            self.current_step = 0;
            self.position_in_step = 0.0;
            return 0;
        }
        if sample_rate <= 0.0 {
            return self.step_semitones[self.current_step];
        }

        let one_over_samplerate = 1.0 / sample_rate;
        // Odin: `m_inc = m_frequency * m_one_over_samplerate * pitchShiftMultiplier(*m_freq_mod * 24)`
        let inc = self.frequency
            * one_over_samplerate
            * if freq_mod != 0.0 {
                pitch_shift_multiplier(freq_mod * 24.0)
            } else {
                1.0
            };

        self.position_in_step += inc;
        // Odin: wrap position inside step, then advance/wrap step.
        let mut wrapped = false;
        while self.position_in_step > STEP_LENGTH {
            self.position_in_step -= STEP_LENGTH;
            wrapped = true;
        }
        if wrapped {
            self.current_step += 1;
            if self.current_step > 2 || (!self.step_three_active && self.current_step > 1) {
                self.current_step = 0;
            }
        }

        self.step_semitones[self.current_step.min(2)]
    }

    pub fn set_step_semitone(&mut self, step: usize, semitone: i32) {
        if step < ARP_STEPS {
            self.step_semitones[step] = semitone;
        }
    }
}
