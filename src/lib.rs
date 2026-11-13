pub mod engine;
pub mod params;
pub mod presets;
pub mod ui;
pub mod voice;

use engine::graph::{CompiledGraph, Patch, PlayMode};
use params::LittleMommyParams;
use voice::Voice;

use nice_plug::prelude::*;
use nice_plug_egui::{EguiEditor, EguiEditorState, RepaintNotifier};
use std::sync::{Arc, Mutex};

const NUM_VOICES: usize = 16;

pub struct LittleMommy {
    params: Arc<LittleMommyParams>,
    patch: Arc<Mutex<Patch>>,
    compiled: CompiledGraph,
    compiled_version: u64,
    play_mode: PlayMode,
    held_notes: Vec<u8>,
    voices: [Option<Voice>; NUM_VOICES],
    next_internal_voice_id: u64,
    editor_state: Arc<EguiEditorState>,
    repaint_notifier: RepaintNotifier,
    initial_editor: Option<ui::LittleMommyEditor>,
}

impl Default for LittleMommy {
    fn default() -> Self {
        let params = Arc::new(LittleMommyParams::default());
        let patch = Arc::new(Mutex::new(presets::default_patch()));
        let library = crate::engine::osc::wavetable::read_guard();
        let compiled = CompiledGraph::compile(&patch.lock().unwrap().grid.to_graph(), &library);
        let compiled_version = patch.lock().unwrap().version;

        Self {
            params: params.clone(),
            patch: patch.clone(),
            compiled,
            compiled_version,
            play_mode: PlayMode::default(),
            held_notes: Vec::new(),
            voices: std::array::from_fn(|_| None),
            next_internal_voice_id: 0,
            editor_state: EguiEditorState::from_size(ui::MIN_WINDOW_SIZE, ui::INITIAL_SCALE_FACTOR),
            repaint_notifier: RepaintNotifier::new(),
            initial_editor: Some(ui::LittleMommyEditor::new(params, patch)),
        }
    }
}

impl LittleMommy {
    fn note_on(&mut self, note: u8, velocity: f32) {
        self.held_notes.retain(|n| *n != note);
        self.held_notes.push(note);

        match self.play_mode {
            PlayMode::Poly => self.spawn_voice(note, velocity),
            PlayMode::MonoLegato => {
                let reused = match self.active_mono_voice() {
                    Some(voice) => {
                        voice.note_number = note;
                        true
                    }
                    None => false,
                };
                if !reused {
                    self.spawn_voice_mono(note, velocity);
                }
            }
            PlayMode::MonoRetrig => {
                let reused = match self.active_mono_voice() {
                    Some(voice) => {
                        voice.note_number = note;
                        voice.retrigger_envs();
                        true
                    }
                    None => false,
                };
                if !reused {
                    self.spawn_voice_mono(note, velocity);
                }
            }
        }
    }

    fn note_off(&mut self, note: u8) {
        if let Some(index) = self.held_notes.iter().rposition(|n| *n == note) {
            self.held_notes.remove(index);
        }
        match self.play_mode {
            PlayMode::Poly => self.release_note(note),
            PlayMode::MonoLegato | PlayMode::MonoRetrig => match self.held_notes.last().copied() {
                Some(top) => {
                    if let Some(voice) = self.active_mono_voice() {
                        voice.note_number = top;
                    }
                }
                None => {
                    for slot in self.voices.iter_mut() {
                        if let Some(voice) = slot
                            && !voice.stealing
                        {
                            voice.releasing = true;
                        }
                    }
                }
            },
        }
    }

    fn active_mono_voice(&mut self) -> Option<&mut Voice> {
        self.voices
            .iter_mut()
            .filter_map(|v| v.as_mut())
            .find(|v| !v.stealing && !v.releasing)
    }

    fn spawn_voice(&mut self, note: u8, velocity: f32) {
        self.allocate_voice(note, velocity);
    }

    fn spawn_voice_mono(&mut self, note: u8, velocity: f32) {
        for slot in self.voices.iter_mut() {
            if let Some(voice) = slot
                && !voice.stealing
            {
                voice.begin_steal();
            }
        }
        self.allocate_voice(note, velocity);
    }

    fn allocate_voice(&mut self, note: u8, velocity: f32) {
        let internal_voice_id = self.next_internal_voice_id;
        self.next_internal_voice_id = internal_voice_id.wrapping_add(1);
        let new_voice = Voice::new(internal_voice_id, note, velocity, self.compiled.nodes.len());
        if let Some(free_idx) = self.voices.iter().position(|v| v.is_none()) {
            self.voices[free_idx] = Some(new_voice);
            return;
        }
        let oldest_idx = self
            .voices
            .iter()
            .enumerate()
            .filter_map(|(idx, v)| v.as_ref().map(|v| (idx, v.internal_voice_id)))
            .min_by_key(|(_, id)| *id)
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        self.voices[oldest_idx] = Some(new_voice);
    }

    fn release_note(&mut self, note: u8) {
        for slot in self.voices.iter_mut() {
            if let Some(voice) = slot
                && voice.note_number == note
            {
                voice.releasing = true;
            }
        }
    }

    fn choke_note(&mut self, note: u8) {
        self.held_notes.retain(|n| *n != note);
        match self.play_mode {
            PlayMode::Poly => {
                for slot in self.voices.iter_mut() {
                    let matches = slot
                        .as_ref()
                        .map(|v| v.note_number == note)
                        .unwrap_or(false);
                    if matches {
                        *slot = None;
                    }
                }
            }
            PlayMode::MonoLegato | PlayMode::MonoRetrig => {
                if self.held_notes.is_empty() {
                    for slot in self.voices.iter_mut() {
                        *slot = None;
                    }
                } else {
                    let top = *self.held_notes.last().unwrap();
                    let sounding_choked = self.voices.iter().any(|v| {
                        v.as_ref()
                            .map(|v| !v.stealing && v.note_number == note)
                            .unwrap_or(false)
                    });
                    if sounding_choked && let Some(voice) = self.active_mono_voice() {
                        voice.note_number = top;
                    }
                }
            }
        }
    }

    fn terminate_finished_voices(&mut self) {
        for slot in self.voices.iter_mut() {
            let done = slot.as_ref().map(|v| v.is_finished()).unwrap_or(false);
            if done {
                *slot = None;
            }
        }
    }
}

impl Plugin for LittleMommy {
    const NAME: &'static str = "Little Mommy III";
    const VENDOR: &'static str = "Tom Pizza Labs";
    const URL: &'static str = "https://pornhub.com";
    const EMAIL: &'static str = "iant@givinmyemail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type Editor = EguiEditor<ui::LittleMommyEditor>;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Self::Editor> {
        ui::build_editor(
            self.editor_state.clone(),
            self.repaint_notifier.clone(),
            self.initial_editor.take().unwrap(),
        )
    }

    fn reset(&mut self) {
        self.voices.fill(None);
        self.held_notes.clear();
        self.next_internal_voice_id = 0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if let Ok(patch) = self.patch.try_lock() {
            if patch.play_mode != self.play_mode {
                self.play_mode = patch.play_mode;
                self.held_notes.clear();
                for slot in self.voices.iter_mut() {
                    if let Some(voice) = slot
                        && !voice.stealing
                    {
                        voice.releasing = true;
                    }
                }
            }
            if patch.version != self.compiled_version
                && let Ok(library) = crate::engine::osc::wavetable::library().try_read()
            {
                self.compiled = CompiledGraph::compile(&patch.grid.to_graph(), &library);
                self.compiled_version = patch.version;
                self.held_notes.clear();
                self.voices.fill(None);
            }
        }

        let sample_rate = context.transport().sample_rate;
        let mut next_event = context.next_event();

        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            while let Some(event) = next_event {
                if event.timing() > sample_id as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { key, velocity, .. } => {
                        let note = key.number().unwrap_or(0);
                        if velocity <= 0.0 {
                            self.note_off(note);
                        } else {
                            self.note_on(note, velocity);
                        }
                    }
                    NoteEvent::NoteOff { key, .. } => {
                        let note = key.number().unwrap_or(0);
                        self.note_off(note);
                    }
                    NoteEvent::Choke { key, .. } => {
                        let note = key.number().unwrap_or(0);
                        self.choke_note(note);
                    }
                    _ => (),
                }
                next_event = context.next_event();
            }

            let master_gain = self.params.master_gain.smoothed.next();
            let mut left = 0.0;
            let mut right = 0.0;
            for voice in self.voices.iter_mut().filter_map(|v| v.as_mut()) {
                let (l, r) = voice.render(&self.compiled, sample_rate);
                left += l;
                right += r;
            }
            let out_l = (left * master_gain).clamp(-1.0, 1.0);
            let out_r = (right * master_gain).clamp(-1.0, 1.0);
            let mut channels = channel_samples.into_iter();
            match (channels.next(), channels.next()) {
                (Some(l), Some(r)) => {
                    *l = out_l;
                    *r = out_r;
                }
                (Some(l), None) => {
                    *l = (out_l + out_r) * 0.5;
                }
                _ => {}
            }
            self.terminate_finished_voices();
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for LittleMommy {
    const CLAP_ID: &'static str = "com.TomPizzaLabs.LittleMommyIII";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Modular node-based synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for LittleMommy {
    const VST3_CLASS_ID: [u8; 16] = *b"LittleMommyIII00";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

nice_export_clap!(LittleMommy);
nice_export_vst3!(LittleMommy);
