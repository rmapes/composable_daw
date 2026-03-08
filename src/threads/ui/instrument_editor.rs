use iced::{Element, Length, Task};

use crate::models::components::Track;
use crate::threads::audio::sources::synth::config::Instrument;
use crate::models::shared::TrackIdentifier;

// TODO: Decouple via instrument registry
use crate::threads::audio::sources::synth::editor::{handle_event, synth_editor_ui};
use crate::threads::engine::actions::Actions;

use super::actions::Message;
use super::components;
use super::instrument_editor_event::Event;

#[derive(Debug, Clone)]
pub struct Component {
    width: Length,
    height: Length,
}

impl Component {
    pub fn new(width: Length, height: Length) -> Self {
        Self { width, height }
    }

    /// Handle an instrument-editor event by dispatching to the active instrument's editor. Returns a task and an optional engine action.
    pub fn update(evt: Event) -> (Task<Message>, Option<Actions>) {
        if let Some(result) = handle_event(evt) {
            result
        } else {
            (Task::none(), None)
        }
    }

    pub fn view(&self, track: &Track) -> Element<'_, Message> {
        let TrackIdentifier { track_id: _ } = track.id;
        let instrument = &track.instrument.kind;

        let content = match instrument {
            Instrument::Synth(synth) => synth_editor_ui(track, synth),
        };

        components::module(content)
            .width(self.width)
            .height(self.height)
            .into()
    }
}

