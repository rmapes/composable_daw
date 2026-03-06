use iced::{Element, Length, Task};

use crate::models::components::Track;
use crate::models::instuments::{Instrument, InstrumentActions, SynthMessage};
use crate::models::shared::TrackIdentifier;

use crate::threads::engine::actions::Actions;
// TODO: Decouple via instrument registry
use crate::threads::audio::sources::synth::editor::synth_editor_ui;

use super::actions::Message;
use super::components;
use super::file_picker;
use super::instrument_editor_event::Event;

const SOUNDFONTS_DIR: &str = "./soundfonts/";

#[derive(Debug, Clone)]
pub struct Component {
    width: Length,
    height: Length,
}

impl Component {
    pub fn new(width: Length, height: Length) -> Self {
        Self { width, height }
    }

    /// Handle an instrument-editor event. Returns a task (e.g. file picker) and an optional engine action.
    pub fn update(evt: Event) -> (Task<Message>, Option<Actions>) {
        match evt {
            Event::Synth(SynthMessage::SelectSoundFont(track_id)) => (
                Task::perform(
                    file_picker::pick_file(track_id, SOUNDFONTS_DIR),
                    |(track_id, path)| Message::InstrumentEditor(Event::Synth(SynthMessage::SetSoundFont(track_id, path))),
                ),
                None,
            ),
            Event::Synth(SynthMessage::SetSoundFont(track_id, path)) => (
                Task::none(),
                Some(Actions::Instrument(track_id, InstrumentActions::SetSoundFont(path))),
            ),
        }
    }

    pub fn view(&self, track: &Track) -> Element<'_, Message> {
        let TrackIdentifier { track_id: _ } = track.id;
        let instrument = &track.instrument.kind;

        let content = match instrument {
            Instrument::Synth(synth) => synth_editor_ui(track, synth, |sm| Message::InstrumentEditor(Event::Synth(sm))),
        };

        components::module(content)
            .width(self.width)
            .height(self.height)
            .into()
    }
}

