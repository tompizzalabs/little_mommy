pub const MAX_UNISON_VOICES: usize = 16;

pub fn detune_multipliers(voices: u8, power: f32) -> [f32; MAX_UNISON_VOICES] {
    let mut result = [0.0f32; MAX_UNISON_VOICES];
    let n = voices.max(1).min(MAX_UNISON_VOICES as u8) as usize;

    for (i, slot) in result.iter_mut().enumerate().take(n) {
        let centered = if n == 1 {
            0.0
        } else {
            (i as f32 / (n - 1) as f32) * 2.0 - 1.0
        };

        let abs_c = centered.abs();
        let sign = if centered >= 0.0 { 1.0 } else { -1.0 };

        let shaped = if power >= 0.0 {
            abs_c.powf(1.0 + power)
        } else {
            1.0 - (1.0 - abs_c).powf(1.0 - power)
        };

        *slot = shaped * sign;
    }

    result
}

/// Pan position in [-1, 1] for each unison voice, distributed symmetrically.
pub fn pan_positions(voices: u8) -> [f32; MAX_UNISON_VOICES] {
    let mut positions = [0.0f32; MAX_UNISON_VOICES];
    let n = voices.max(1).min(MAX_UNISON_VOICES as u8) as usize;
    if n == 1 {
        positions[0] = 0.0;
    } else {
        for (i, pos) in positions.iter_mut().enumerate().take(n) {
            *pos = (i as f32 / (n - 1) as f32) * 2.0 - 1.0;
        }
    }
    positions
}

/// Equal-power gain compensation to normalize perceived loudness across voice counts.
pub fn gain_compensation(voices: u8) -> f32 {
    let n = voices.max(1).min(MAX_UNISON_VOICES as u8) as f32;
    1.0 / n.sqrt()
}

/// Equal-power pan gains (left, right) for a position in [-1, 1].
pub fn pan_gains(position: f32) -> (f32, f32) {
    let theta = (position + 1.0) * 0.25 * std::f32::consts::PI;
    (theta.cos(), theta.sin())
}
