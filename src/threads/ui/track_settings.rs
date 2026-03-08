use crate::models::components::Track;
use crate::models::shared::TrackIdentifier;
use crate::threads::audio::sources::synth::config::Instrument;
use iced::widget::{Column, button, column, row, text};
use iced::{Element, Length};

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
        components::module(
            row![self.channel_strip(track),]
                .width(self.width) //Length::Fixed(self.width))
                .height(self.height) //Length::Fixed(self.height))
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

    fn instrument_config(
        &self,
        instrument: &Instrument,
        track_id: TrackIdentifier,
    ) -> Column<'_, Message> {
        match instrument {
            Instrument::Synth(_) => column![
                button(text("Instrument…").size(12))
                    .on_press(Message::OpenInstrumentEditor(track_id)),
            ],
        }
    }
}
