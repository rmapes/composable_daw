mod config;
mod editor;
#[allow(clippy::module_inception)]
mod synth;

use std::collections::HashMap;

use crate::models::components::Track;
use crate::models::instrument::{InstrumentAction, InstrumentConfig};
use crate::models::sequences::EventStreamSource;
use crate::models::shared::TrackIdentifier;
use crate::threads::audio::controllers::MidiInputMessage;
use crate::threads::ui::actions::Message;
use crate::threads::ui::instrument_editor_event::Event as InstrumentEditorEvent;

use iced::{Element, Task};

use super::super::super::engine::actions::Actions;

type CreateTrackSynthFn = Box<
    dyn Fn(
            TrackIdentifier,
            &dyn EventStreamSource,
            u32,
            &dyn InstrumentConfig,
            flume::Receiver<MidiInputMessage>,
        ) -> Result<TrackSynth, Box<dyn std::error::Error + Send + Sync>>
        + Send
        + Sync,
>;
type ApplyActionToConfigFn =
    Box<dyn Fn(&mut dyn InstrumentConfig, &dyn std::any::Any) -> bool + Send + Sync>;
type ViewEditorFn =
    Box<dyn Fn(&Track, &dyn InstrumentConfig) -> Option<Element<'static, Message>> + Send + Sync>;
type HandleEditorEventFn =
    Box<dyn Fn(InstrumentEditorEvent) -> Option<(Task<Message>, Option<Actions>)> + Send + Sync>;

/// Re-export so the engine can hold `Rc<RefCell<TrackSynth>>`; no other synth types are public.
pub use synth::TrackSynth;

/// Re-export for instrument editor event (Event::Synth(SynthMessage)).
pub use config::SynthMessage;

/// One instrument kind's handlers. All closures are invoked from engine or UI thread.
/// All boxed Fn traits are Send + Sync so that Arc<InstrumentRegistry> can be shared with the engine thread.
struct RegistryEntry {
    default_config: Box<dyn Fn() -> Box<dyn InstrumentConfig> + Send + Sync>,
    create_track_synth: CreateTrackSynthFn,
    apply_action_to_config: ApplyActionToConfigFn,
    view_editor: ViewEditorFn,
    handle_editor_event: Option<HandleEditorEventFn>,
}

/// Maps instrument kind strings (e.g. "simple_synth") to constructors and handlers.
/// The only public entry point into the synth implementation.
pub struct InstrumentRegistry {
    entries: HashMap<String, RegistryEntry>,
}

impl std::fmt::Debug for InstrumentRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InstrumentRegistry")
    }
}

impl InstrumentRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Default config for the given kind. Caller should assign to `VirtualInstrument.config` when None.
    pub fn default_config(&self, kind: &str) -> Option<Box<dyn InstrumentConfig>> {
        self.entries
            .get(kind)
            .map(|e| (e.default_config)())
    }

    /// Create the audio source for a track. Returns Ok(TrackSynth) for the engine to hold.
    pub fn create_track_synth(
        &self,
        kind: &str,
        track_id: TrackIdentifier,
        seq: &dyn EventStreamSource,
        sample_rate: u32,
        config: &dyn InstrumentConfig,
        midi_rx: flume::Receiver<MidiInputMessage>,
    ) -> Result<TrackSynth, Box<dyn std::error::Error + Send + Sync>> {
        self.entries
            .get(kind)
            .ok_or_else(|| "unknown instrument kind".into())
            .and_then(|e| (e.create_track_synth)(track_id, seq, sample_rate, config, midi_rx))
    }

    /// Apply an action to the stored config (e.g. after receiving Actions::Instrument). Returns true if config changed.
    pub fn apply_action_to_config(
        &self,
        kind: &str,
        config: &mut dyn InstrumentConfig,
        action: &InstrumentAction,
    ) -> bool {
        self.entries
            .get(kind)
            .map(|e| (e.apply_action_to_config)(config, action.as_ref()))
            .unwrap_or(false)
    }

    /// Apply an instrument action to both the live synth and the stored config. Call this from the
    /// engine so that the TrackSynth (audio) and project config stay in sync. `live_synth_updater`
    /// should call into AudioSources to update the TrackSynth for this track.
    pub fn apply_instrument_action(
        &self,
        kind: &str,
        track_id: TrackIdentifier,
        action: &InstrumentAction,
        config: &mut dyn InstrumentConfig,
        live_synth_updater: impl FnOnce(
            TrackIdentifier,
            &InstrumentAction,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        live_synth_updater(track_id, action)?;
        Ok(self.apply_action_to_config(kind, config, action))
    }

    /// Build the instrument editor UI for this kind. Returns None if no editor.
    pub fn view_editor(
        &self,
        kind: &str,
        track: &Track,
        config: &dyn InstrumentConfig,
    ) -> Option<Element<'static, Message>> {
        self.entries
            .get(kind)
            .and_then(|e| (e.view_editor)(track, config))
    }

    /// True if this kind has an editor (e.g. for showing "Instrument…" button).
    pub fn has_editor(&self, kind: &str) -> bool {
        self.entries.contains_key(kind)
    }

    /// Dispatch an instrument editor event (e.g. file picker, set soundfont). Returns None if no handler.
    pub fn handle_editor_event(
        &self,
        evt: InstrumentEditorEvent,
    ) -> Option<(Task<Message>, Option<Actions>)> {
        for entry in self.entries.values() {
            if let Some(handler) = &entry.handle_editor_event
                && let Some(result) = handler(evt.clone())
            {
                return Some(result);
            }
        }
        None
    }

    fn register(
        &mut self,
        kind: &str,
        entry: RegistryEntry,
    ) {
        self.entries.insert(kind.to_string(), entry);
    }
}

/// Register the simple_synth instrument. Call at startup before passing the registry to engine/UI.
pub fn register_simple_synth(registry: &mut InstrumentRegistry) {
    registry.register(
        "simple_synth",
        RegistryEntry {
            default_config: Box::new(|| Box::new(config::SimpleSynth::default())),
            create_track_synth: Box::new(
                |track_id, seq, sample_rate, config, midi_rx| {
                    let synth = config
                        .as_any()
                        .downcast_ref::<config::SimpleSynth>()
                        .ok_or("simple_synth config downcast failed")?;
                    Ok(synth::TrackSynth::new(
                        track_id,
                        seq,
                        sample_rate,
                        &synth.get_soundfont_path(),
                        synth.bank,
                        synth.program,
                        midi_rx,
                    ))
                },
            ),
            apply_action_to_config: Box::new(|config, action| config.apply_action(action)),
            view_editor: Box::new(|track, config| {
                config
                    .as_any()
                    .downcast_ref::<config::SimpleSynth>()
                    .map(|synth| editor::synth_editor_ui(track, synth))
            }),
            handle_editor_event: Some(Box::new(editor::handle_event)),
        },
    );
}
