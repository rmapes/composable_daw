use std::sync::Arc;

use crate::models::components::Track;
use crate::models::shared::TrackIdentifier;
use crate::threads::audio::sources::synth::InstrumentRegistry;
use iced::widget::{Column, button, column, row, text};
use iced::{Element, Length};

use super::actions::Message;
use super::components;

#[derive(Clone)]
pub struct Component {
    width: Length,
    height: Length,
    registry: Arc<InstrumentRegistry>,
}

impl Component {
    pub fn new(width: Length, height: Length, registry: Arc<InstrumentRegistry>) -> Self {
        Self {
            width,
            height,
            registry,
        }
    }

    pub fn view(&self, track: &Track) -> Element<'_, Message> {
        components::module(
            row![self.channel_strip(track),]
                .width(self.width)
                .height(self.height)
                .into(),
        )
        .into()
    }

    fn channel_strip(&self, track: &Track) -> Column<'_, Message> {
        column![components::control(
            column![
                text(track.name.clone()),
                self.instrument_config(&track.instrument.kind, track.id),
            ]
            .into(),
        ),]
    }

    fn instrument_config(&self, kind: &str, track_id: TrackIdentifier) -> Column<'_, Message> {
        if self.registry.has_editor(kind) {
            column![
                button(text("Instrument…").size(12))
                    .on_press(Message::OpenInstrumentEditor(track_id)),
            ]
        } else {
            column![]
        }
    }
}
