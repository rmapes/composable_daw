use iced::{Element, Length};

use crate::models::components::Track;
use crate::models::instuments::Instrument;
use crate::models::shared::TrackIdentifier;

//TODO: Decouple via instrument registry
use crate::threads::audio::sources::synth::editor::synth_editor_ui;

use super::actions::Message;
use super::components;

#[derive(Debug, Clone)]
pub struct Component {
    width: Length,
    height: Length,
}

impl Component {
    pub fn new(width: Length, height: Length) -> Self {
        Self { width, height }
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

