use iced::widget::{button, column, pick_list, row, text};
use iced::{Element, Length};

use crate::models::components::Track;
use crate::models::instuments::{Instrument, InstrumentActions};
use crate::models::shared::TrackIdentifier;

use super::super::engine::actions::Actions;
use super::actions::{Message, SynthMessage};
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
            Instrument::Synth(synth) => {
                column![
                    text("Instrument Settings"),
                    row![
                        text("Soundfont:").size(12),
                        button(text(synth.soundfont.clone()).size(12)).on_press(
                            Message::Synth(SynthMessage::SelectSoundFont(track.id))
                        )
                    ]
                    .spacing(8),
                    components::label(text("Bank").into()),
                    self.number_selector(0, 127, synth.bank as u8, {
                        let track_id = track.id;
                        move |val: u8| {
                            Message::Engine(Actions::Instrument(
                                track_id,
                                InstrumentActions::SetBank(val as u32),
                            ))
                        }
                    }),
                    components::label(text("Program").into()),
                    self.number_selector(0, 127, synth.program, {
                        let track_id = track.id;
                        move |val: u8| {
                            Message::Engine(Actions::Instrument(
                                track_id,
                                InstrumentActions::SetProgram(val),
                            ))
                        }
                    }),
                    row![button(text("Done")).on_press(Message::CloseInstrumentEditor)]
                        .spacing(8),
                ]
                .spacing(4)
                .into()
            }
        };

        components::module(content)
            .width(self.width)
            .height(self.height)
            .into()
    }

    fn number_selector<F>(&self, min: u8, max: u8, current: u8, on_set: F) -> Element<'_, Message>
    where
        F: Fn(u8) -> Message + 'static,
    {
        let options: Vec<u8> = (min..=max).collect();
        pick_list(options, Some(current), on_set).into()
    }
}

