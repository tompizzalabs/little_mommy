//! Unison engine.
//!
//! Voice layouts (detune shaping, pan positions, gain compensation, stack
//! intervals) are precomputed and cached; they are only rebuilt when a
//! layout-affecting parameter actually changes. Per sample the engine only
//! advances the swarm oscillators (rotation trick, periodically renormalized)
//! and reads gains from a small pan LUT — no trig or `powf` in the layout
//! path. Randomness comes from a deterministic seeded RNG so rebuilds are
//! stable and every note gets its own swarm phase.

use std::f32::consts::{PI, TAU};

pub const MAX_UNISON_VOICES: usize = 16;
/// Full-scale detune spread at `detune = 1.0`, in cents.
pub const MAX_DETUNE_CENTS: f32 = 50.0;

/// How detune is spread across voices. The continuous `detune_power`
/// parameter covers the analytic family (Linear = 0, Exponential = 1,
/// InvertedExponential = -1, Super = -0.5); these modes add character that
/// the power curve cannot reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DetuneMode {
    /// Shaped by the `detune_power` curve.
    Power,
    /// Random symmetric jitter.
    Random,
    /// Analog-style mirrored jitter with a dead-center voice.
    Analog,
}

impl DetuneMode {
    pub const ALL: [DetuneMode; 3] = [
        DetuneMode::Power,
        DetuneMode::Random,
        DetuneMode::Analog,
    ];
}

impl std::fmt::Display for DetuneMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            DetuneMode::Power => "Curve",
            DetuneMode::Random => "Random",
            DetuneMode::Analog => "Analog",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PanMode {
    /// Even spread across the field.
    Linear,
    /// Linear, pulled toward the center.
    Center,
    /// Pushed toward the edges.
    Outer,
    /// Hard left / hard right alternation.
    Alternating,
    /// Random symmetric spread.
    Random,
}

impl PanMode {
    pub const ALL: [PanMode; 5] = [
        PanMode::Linear,
        PanMode::Center,
        PanMode::Outer,
        PanMode::Alternating,
        PanMode::Random,
    ];
}

impl std::fmt::Display for PanMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            PanMode::Linear => "Linear",
            PanMode::Center => "Center",
            PanMode::Outer => "Outer",
            PanMode::Alternating => "Alt",
            PanMode::Random => "Random",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StackMode {
    Unison,
    Octaves,
    Fifths,
    OctavesAndFifths,
}

impl StackMode {
    pub const ALL: [StackMode; 4] = [
        StackMode::Unison,
        StackMode::Octaves,
        StackMode::Fifths,
        StackMode::OctavesAndFifths,
    ];
}

impl std::fmt::Display for StackMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            StackMode::Unison => "Unison",
            StackMode::Octaves => "Octaves",
            StackMode::Fifths => "Fifths",
            StackMode::OctavesAndFifths => "Oct+Fifth",
        };
        write!(f, "{}", name)
    }
}

/// All inputs the unison engine consumes. `Copy + PartialEq` so the engine
/// can detect real changes and skip layout rebuilds otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnisonConfig {
    pub voices: u8,
    /// Detune amount in [0, 1] (scaled to `MAX_DETUNE_CENTS`).
    pub detune: f32,
    /// Continuous shaping exponent in [-5, 5]; ignored unless
    /// `detune_mode` is `Power`.
    pub detune_power: f32,
    pub detune_mode: DetuneMode,
    pub pan_mode: PanMode,
    pub stack_mode: StackMode,
    /// Stereo spread in [0, 1].
    pub width: f32,
    /// Gain weighting: +1 favors edge voices, -1 favors center voices.
    pub blend: f32,
    /// Slow per-voice detune/pan movement in [0, 1].
    pub swarm: f32,
    pub swarm_rate_hz: f32,
    /// Fraction of per-voice phase re-randomization applied on retrigger.
    pub phase_randomization: f32,
}

impl Default for UnisonConfig {
    fn default() -> Self {
        Self {
            voices: 1,
            detune: 0.0,
            detune_power: 1.0,
            detune_mode: DetuneMode::Power,
            pan_mode: PanMode::Linear,
            stack_mode: StackMode::Unison,
            width: 1.0,
            blend: 0.0,
            swarm: 0.0,
            swarm_rate_hz: 0.15,
            phase_randomization: 0.0,
        }
    }
}

impl UnisonConfig {
    fn normalized(mut self) -> Self {
        self.voices = self.voices.max(1).min(MAX_UNISON_VOICES as u8);
        self.detune = finite(self.detune).clamp(0.0, 1.0);
        self.detune_power = finite(self.detune_power).clamp(-5.0, 5.0);
        self.width = finite(self.width).clamp(0.0, 1.0);
        self.blend = finite(self.blend).clamp(-1.0, 1.0);
        self.swarm = finite(self.swarm).clamp(0.0, 1.0);
        self.swarm_rate_hz = finite(self.swarm_rate_hz).clamp(0.0, 20.0);
        self.phase_randomization = finite(self.phase_randomization).clamp(0.0, 1.0);
        self
    }

    /// Fields that define the voice layout; when only these change the
    /// engine rebuilds. Detune amount and swarm can move per sample without
    /// invalidating anything.
    fn layout_key(&self) -> Self {
        UnisonConfig {
            voices: self.voices,
            detune: 0.0,
            detune_power: self.detune_power,
            detune_mode: self.detune_mode,
            pan_mode: self.pan_mode,
            stack_mode: self.stack_mode,
            width: self.width,
            blend: self.blend,
            swarm: 0.0,
            swarm_rate_hz: self.swarm_rate_hz,
            phase_randomization: 0.0,
        }
    }
}

/// Per-voice output for one audio sample. Gains include the unison gain
/// compensation — callers just multiply their oscillator output.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnisonFrame {
    /// Frequency multiplier including detune and stack interval.
    pub pitch_multiplier: f32,
    pub gain_left: f32,
    pub gain_right: f32,
}

#[derive(Debug, Clone, Copy)]
struct Voice {
    /// Shaped detune position in [-1, 1] (before the detune amount scale).
    detune_position: f32,
    /// Stack interval in semitones for this slot.
    stack_semitones: f32,
    /// Pan position in [-1, 1] (before width scaling).
    base_pan: f32,
    /// Normalized voice gain.
    gain: f32,
    /// Swarm rotation state (unit phasor).
    swarm_sin: f32,
    swarm_cos: f32,
    swarm_step_sin: f32,
    swarm_step_cos: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            detune_position: 0.0,
            stack_semitones: 0.0,
            base_pan: 0.0,
            gain: 0.0,
            swarm_sin: 0.0,
            swarm_cos: 1.0,
            swarm_step_sin: 0.0,
            swarm_step_cos: 1.0,
        }
    }
}

const PAN_LUT_SIZE: usize = 256;
const PAN_LUT_MASK: usize = PAN_LUT_SIZE - 1;
const PAN_LUT_SCALE: f32 = PAN_LUT_SIZE as f32 * 0.5;
const RECIPROCAL_PAN_LUT_SCALE: f32 = 1.0 / PAN_LUT_SCALE;
/// Maximum swarm detune depth at full swarm amount, in cents.
pub const SWARM_MAX_CENTS: f32 = 50.0;
/// Renormalize the swarm phasors every N samples so the rotation trick's
/// rounding error can never accumulate into a gain drift.
const RENORMALIZE_INTERVAL: u64 = 4096;

#[derive(Debug, Clone)]
pub struct UnisonEngine {
    voices: [Voice; MAX_UNISON_VOICES],
    pan_lut: [(f32, f32); PAN_LUT_SIZE + 1],
    layout_key: Option<UnisonConfig>,
    voice_count: usize,
    swarm_enabled: bool,
    swarm_detune_cents: f32,
    swarm_pan: f32,
    swarm_rate_hz: f32,
    phase_randomization: f32,
    sample_counter: u64,
    random: Random,
}

impl Default for UnisonEngine {
    fn default() -> Self {
        Self::new(0)
    }
}

impl UnisonEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            voices: [Voice::default(); MAX_UNISON_VOICES],
            pan_lut: build_pan_lut(),
            layout_key: None,
            voice_count: 1,
            swarm_enabled: false,
            swarm_detune_cents: 0.0,
            swarm_pan: 0.0,
            swarm_rate_hz: 0.15,
            phase_randomization: 0.0,
            sample_counter: 0,
            random: Random::new(seed),
        }
    }

    /// Re-seed the RNG (per voice/node instance). Invalidates the layout so
    /// the next `sync` re-rolls randomized layouts and swarm phases.
    pub fn seed(&mut self, seed: u64) {
        self.random = Random::new(seed);
        self.layout_key = None;
    }

    /// Stored phase randomization amount, applied by the oscillator states
    /// when the note is retriggered.
    #[inline(always)]
    pub fn phase_randomization(&self) -> f32 {
        self.phase_randomization
    }

    /// Number of active voices in the current layout.
    #[inline(always)]
    pub fn voice_count(&self) -> usize {
        self.voice_count
    }

    /// Bring the cached layout in line with `config`. Cheap when nothing
    /// changed (the common case): one struct compare. Swarm phases survive
    /// rebuilds so the movement never audibly resets.
    pub fn sync(&mut self, config: UnisonConfig, sample_rate: f32) {
        let config = config.normalized();
        let key = config.layout_key();

        if self.layout_key != Some(key) {
            self.rebuild(config, sample_rate);
            self.layout_key = Some(key);
        }

        // Fast-moving values: applied even when the layout is cached.
        self.swarm_detune_cents = config.swarm * SWARM_MAX_CENTS;
        self.swarm_pan = config.swarm * config.width;
        self.swarm_enabled = config.swarm > 0.0 && self.voice_count > 1;
        self.phase_randomization = finite(config.phase_randomization);

        // Swarm rate lives outside the layout: re-derive only the rotation
        // steps when it changes, so rate modulation never rebuilds anything.
        if self.swarm_rate_hz != config.swarm_rate_hz {
            let step_angle =
                TAU * config.swarm_rate_hz / finite(sample_rate).max(1.0);
            let (step_sin, step_cos) = step_angle.sin_cos();

            for voice in self.voices.iter_mut() {
                voice.swarm_step_sin = step_sin;
                voice.swarm_step_cos = step_cos;
            }

            self.swarm_rate_hz = config.swarm_rate_hz;
        }
    }

    /// Per-voice frame for this sample. Call exactly once per voice per
    /// sample; advances that voice's swarm oscillator.
    #[inline(always)]
    pub fn frame(&mut self, index: usize, detune: f32, max_detune_cents: f32) -> UnisonFrame {
        let voice = &mut self.voices[index];

        let (pan, cents) = if self.swarm_enabled {
            let movement = voice.swarm_sin;
            let next_sin =
                voice.swarm_sin * voice.swarm_step_cos + voice.swarm_cos * voice.swarm_step_sin;
            let next_cos =
                voice.swarm_cos * voice.swarm_step_cos - voice.swarm_sin * voice.swarm_step_sin;
            voice.swarm_sin = next_sin;
            voice.swarm_cos = next_cos;

            (
                (voice.base_pan + movement * self.swarm_pan).clamp(-1.0, 1.0),
                voice.detune_position * detune * max_detune_cents
                    + movement * self.swarm_detune_cents,
            )
        } else {
            (voice.base_pan, voice.detune_position * detune * max_detune_cents)
        };

        let pitch_multiplier = cents_to_ratio(cents + voice.stack_semitones * 100.0);

        let (gain_left, gain_right) = pan_lut_lookup(&self.pan_lut, pan);

        UnisonFrame {
            pitch_multiplier,
            gain_left: voice.gain * gain_left,
            gain_right: voice.gain * gain_right,
        }
    }

    /// Renormalize the swarm phasors.
    pub fn tick(&mut self) {
        self.sample_counter = self.sample_counter.wrapping_add(1);

        if self.swarm_enabled && self.sample_counter & (RENORMALIZE_INTERVAL - 1) == 0 {
            for voice in self.voices.iter_mut().take(self.voice_count) {
                let magnitude_squared =
                    voice.swarm_sin * voice.swarm_sin + voice.swarm_cos * voice.swarm_cos;
                let scale = 1.0 / magnitude_squared.sqrt();
                voice.swarm_sin *= scale;
                voice.swarm_cos *= scale;
            }
        }
    }

    /// Re-roll per-voice phase offsets in [0, 1] scaled by the stored phase
    /// randomization amount. Called on note retrigger; oscillator states
    /// apply the returned offsets to their own phases.
    pub fn reroll_phase_offsets(&mut self) -> [f32; MAX_UNISON_VOICES] {
        let amount = self.phase_randomization;
        let mut offsets = [0.0; MAX_UNISON_VOICES];

        for offset in offsets.iter_mut().take(self.voice_count) {
            *offset = self.random.next_f32() * amount;
        }

        offsets
    }

    fn rebuild(&mut self, config: UnisonConfig, sample_rate: f32) {
        let count = config.voices as usize;
        let sample_rate = finite(sample_rate).max(1.0);
        let previous_count = self.voice_count;
        self.voice_count = count;

        let mut detune_positions = [0.0f32; MAX_UNISON_VOICES];
        let mut pan_positions = [0.0f32; MAX_UNISON_VOICES];

        build_detune_positions(
            &mut detune_positions,
            count,
            config.detune_mode,
            config.detune_power,
            &mut self.random,
        );
        build_pan_positions(&mut pan_positions, count, config.pan_mode, &mut self.random);

        let step_angle = TAU * config.swarm_rate_hz / sample_rate;
        let (step_sin, step_cos) = step_angle.sin_cos();

        let mut gains = [0.0f32; MAX_UNISON_VOICES];
        for index in 0..count {
            gains[index] = blend_gain(detune_positions[index].abs(), config.blend);
        }
        normalize_gains(&mut gains, count);

        let first_build = self.layout_key.is_none();
        for index in 0..count {
            let voice = &mut self.voices[index];
            voice.detune_position = detune_positions[index];
            voice.stack_semitones = stack_interval(index, config.stack_mode);
            voice.base_pan = pan_positions[index] * config.width;
            voice.gain = gains[index];

            // Fresh swarm phase for newly added voices; existing voices keep
            // running so parameter tweaks don't restart the movement.
            if first_build || index >= previous_count {
                let swarm_phase = self.random.next_f32() * TAU;
                let (sin, cos) = swarm_phase.sin_cos();
                voice.swarm_sin = sin;
                voice.swarm_cos = cos;
            }
            voice.swarm_step_sin = step_sin;
            voice.swarm_step_cos = step_cos;
        }
    }
}

#[inline(always)]
fn pan_lut_lookup(table: &[(f32, f32); PAN_LUT_SIZE + 1], position: f32) -> (f32, f32) {
    let coordinate = (position + 1.0) * PAN_LUT_SCALE;
    let index = (coordinate as usize).min(PAN_LUT_MASK);
    let fraction = coordinate - index as f32;

    let (a_left, a_right) = table[index];
    let (b_left, b_right) = table[index + 1];

    (
        a_left + (b_left - a_left) * fraction,
        a_right + (b_right - a_right) * fraction,
    )
}

fn build_pan_lut() -> [(f32, f32); PAN_LUT_SIZE + 1] {
    let mut lut = [(0.0f32, 0.0f32); PAN_LUT_SIZE + 1];

    for (index, entry) in lut.iter_mut().enumerate() {
        // Position -1 → angle 0 (full left), +1 → PI/2 (full right),
        // center → PI/4 (equal-power).
        let angle = (index as f32 * RECIPROCAL_PAN_LUT_SCALE) * PI * 0.25;
        *entry = (angle.cos(), angle.sin());
    }

    lut
}

#[inline(always)]
pub fn cents_to_ratio(cents: f32) -> f32 {
    2.0_f32.powf(cents / 1200.0)
}

fn build_detune_positions(
    positions: &mut [f32; MAX_UNISON_VOICES],
    count: usize,
    mode: DetuneMode,
    power: f32,
    random: &mut Random,
) {
    if count <= 1 {
        return;
    }

    if mode == DetuneMode::Random {
        random_symmetric_positions(positions, count, random, 0.15, 1.0);
        return;
    }

    if mode == DetuneMode::Analog {
        // Analog: jittered mirror of the linear layout — voices come in
        // slightly-off symmetric pairs, with the center voice pinned.
        for (index, position) in positions.iter_mut().enumerate().take(count) {
            let centered = index as f32 / (count - 1) as f32 * 2.0 - 1.0;
            let jitter = 0.92 + random.next_f32() * 0.16;
            *position = centered * jitter;
        }

        for index in 0..count / 2 {
            let mirrored = count - 1 - index;
            let magnitude = positions[index].abs().max(positions[mirrored].abs());

            positions[index] = -magnitude;
            positions[mirrored] = magnitude;
        }

        if count % 2 == 1 {
            positions[count / 2] = 0.0;
        }
        return;
    }

    // Power mode: the continuous detune power curve.
    for (index, position) in positions.iter_mut().enumerate().take(count) {
        let centered = index as f32 / (count - 1) as f32 * 2.0 - 1.0;
        let absolute = centered.abs();

        let shaped = if power >= 0.0 {
            absolute.powf(1.0 + power)
        } else {
            1.0 - (1.0 - absolute).powf(1.0 - power)
        };

        *position = shaped * centered.signum();
    }
}

fn build_pan_positions(
    positions: &mut [f32; MAX_UNISON_VOICES],
    count: usize,
    mode: PanMode,
    random: &mut Random,
) {
    if count <= 1 {
        return;
    }

    match mode {
        PanMode::Linear => {
            for (index, position) in positions.iter_mut().enumerate().take(count) {
                *position = index as f32 / (count - 1) as f32 * 2.0 - 1.0;
            }
        }
        PanMode::Center => {
            for (index, position) in positions.iter_mut().enumerate().take(count) {
                *position = (index as f32 / (count - 1) as f32 * 2.0 - 1.0) * 0.5;
            }
        }
        PanMode::Outer => {
            for (index, position) in positions.iter_mut().enumerate().take(count) {
                let centered = index as f32 / (count - 1) as f32 * 2.0 - 1.0;
                *position = centered.signum() * centered.abs().sqrt();
            }
        }
        PanMode::Alternating => {
            for (index, position) in positions.iter_mut().enumerate().take(count) {
                *position = if index % 2 == 0 { -1.0 } else { 1.0 };
            }
        }
        PanMode::Random => {
            random_symmetric_positions(positions, count, random, 0.2, 1.0);
        }
    }
}

/// Symmetric random positions: mirrored pairs get opposite signs so the
/// stack stays center-anchored.
fn random_symmetric_positions(
    positions: &mut [f32; MAX_UNISON_VOICES],
    count: usize,
    random: &mut Random,
    minimum: f32,
    maximum: f32,
) {
    for index in 0..count / 2 {
        let magnitude = minimum + random.next_f32() * (maximum - minimum);
        let mirrored = count - 1 - index;

        positions[index] = -magnitude;
        positions[mirrored] = magnitude;
    }

    if count % 2 == 1 {
        positions[count / 2] = 0.0;
    }
}

/// Gain weighting across the stack: positive blend emphasizes edge voices,
/// negative emphasizes the center.
#[inline(always)]
fn blend_gain(position: f32, blend: f32) -> f32 {
    if blend.abs() < 1e-4 {
        return 1.0;
    }

    let center = 1.0 - position;

    if blend >= 0.0 {
        1.0 + blend * (2.0 * center * center - 1.0)
    } else {
        1.0 + blend * (2.0 * position * position - 1.0)
    }
}

/// Normalize so perceived loudness stays constant across voice counts and
/// blend settings (equal-power over the actual shaped gains).
fn normalize_gains(gains: &mut [f32; MAX_UNISON_VOICES], count: usize) {
    let normalization = gains[..count]
        .iter()
        .map(|gain| gain * gain)
        .sum::<f32>()
        .sqrt();

    if normalization > f32::EPSILON {
        let inverse = normalization.recip();

        for gain in gains.iter_mut().take(count) {
            *gain *= inverse;
        }
    }
}

/// Stack interval in semitones for a voice slot.
#[inline(always)]
fn stack_interval(index: usize, mode: StackMode) -> f32 {
    match mode {
        StackMode::Unison => 0.0,
        StackMode::Octaves => match index % 4 {
            0 => -12.0,
            3 => 12.0,
            _ => 0.0,
        },
        StackMode::Fifths => match index % 4 {
            0 => -7.0,
            3 => 7.0,
            _ => 0.0,
        },
        StackMode::OctavesAndFifths => match index % 5 {
            0 => -12.0,
            1 => -7.0,
            3 => 7.0,
            4 => 12.0,
            _ => 0.0,
        },
    }
}

#[inline(always)]
fn finite(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

/// Small xorshift RNG — deterministic, dependency-free, seeded per node.
#[derive(Debug, Clone)]
struct Random {
    state: u64,
}

impl Random {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E3779B97F4A7C15
            } else {
                seed
            },
        }
    }

    #[inline(always)]
    fn next_f32(&mut self) -> f32 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state = self.state.wrapping_mul(0x2545F4914F6CDD1D);

        (self.state >> 40) as f32 * (1.0 / 16_777_216.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_voice_is_centered_and_unity() {
        let mut engine = UnisonEngine::new(1);
        engine.sync(
            UnisonConfig {
                voices: 1,
                detune: 0.5,
                ..UnisonConfig::default()
            },
            48_000.0,
        );

        let frame = engine.frame(0, 0.5, 50.0);
        assert_eq!(frame.gain_left, frame.gain_right);
        assert!((frame.pitch_multiplier - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sync_is_free_when_unchanged() {
        let mut engine = UnisonEngine::new(1);
        let config = UnisonConfig {
            voices: 7,
            detune: 0.3,
            swarm: 0.4,
            ..UnisonConfig::default()
        };
        engine.sync(config, 48_000.0);
        let first = engine.frame(0, 0.3, 50.0);

        // Same config again: layout is cached, frame matches.
        engine.sync(config, 48_000.0);
        let second = engine.frame(0, 0.3, 50.0);
        assert_eq!(first.pitch_multiplier, second.pitch_multiplier);
    }

    #[test]
    fn swarm_moves_pitch_and_stays_finite() {
        let mut engine = UnisonEngine::new(1);
        engine.sync(
            UnisonConfig {
                voices: 4,
                detune: 1.0,
                swarm: 1.0,
                swarm_rate_hz: 2.0,
                ..UnisonConfig::default()
            },
            48_000.0,
        );

        let start = engine.frame(1, 1.0, 50.0);
        let mut moved = false;
        for _ in 0..2_000 {
            let frame = engine.frame(1, 1.0, 50.0);
            assert!(frame.pitch_multiplier.is_finite());
            assert!(frame.gain_left.is_finite() && frame.gain_right.is_finite());
            if (frame.pitch_multiplier - start.pitch_multiplier).abs() > 1e-4 {
                moved = true;
            }
            engine.tick();
        }
        assert!(moved, "swarm should modulate pitch");
    }

    #[test]
    fn gain_compensation_holds_loudness_across_voice_counts() {
        let loudness = |voices: u8| {
            let mut engine = UnisonEngine::new(1);
            engine.sync(
                UnisonConfig {
                    voices,
                    detune: 0.5,
                    width: 1.0,
                    ..UnisonConfig::default()
                },
                48_000.0,
            );
            let mut power = 0.0;
            for i in 0..voices as usize {
                let frame = engine.frame(i, 0.5, 50.0);
                power += frame.gain_left * frame.gain_left + frame.gain_right * frame.gain_right;
            }
            power.sqrt()
        };

        let one = loudness(1);
        let eight = loudness(8);
        assert!(
            (one - eight).abs() < 0.05,
            "mono loudness should match: {} vs {}",
            one,
            eight
        );
    }

    #[test]
    fn stack_mode_shifts_pitch() {
        let mut engine = UnisonEngine::new(1);
        engine.sync(
            UnisonConfig {
                voices: 4,
                stack_mode: StackMode::Octaves,
                ..UnisonConfig::default()
            },
            48_000.0,
        );

        let frame = engine.frame(3, 0.0, 50.0);
        assert!((frame.pitch_multiplier - 2.0).abs() < 1e-4);
    }
}
