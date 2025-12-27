use iced::widget::{ button, column, pick_list, row, text, Column };
use iced::{Length, Element};
use crate::engine::actions::{Actions, SynthActions};
use crate::models::components::Track;
use crate::models::instuments::Instrument;
use crate::models::shared::TrackIdentifier;

use super::components;
use super::actions::Message;
use super::actions::SynthMessage;

pub struct Component {
    width: Length,
    height: Length,
}

impl Component {
    pub fn new(width: Length, height: Length) -> Self {
        Self {
            width,
            height,
        }
    } 

    pub fn view(&self, track: &Track) -> Element<'_, Message> {
        components::module(
            row![
                self.channel_strip(track),
            ]
            .width(self.width)//Length::Fixed(self.width))
            .height(self.height)//Length::Fixed(self.height))
            .into()
        ).into()
    }

    fn channel_strip(&self, track: &Track) -> Column<'_, Message> {
        column![
            components::control(
                column![
                    text(track.name.clone()),
                    self.instrument_config(&track.instrument.kind, track.id),
                ].into(),
            ),
        ]
    }

    fn instrument_config(&self, instrument: &Instrument, track_id: TrackIdentifier) -> Column<'_, Message> {
        match instrument {
            Instrument::Synth(synth) => {
                column![
                    button(text(synth.soundfont.clone()).size(12)).on_press(Message::Synth(SynthMessage::SelectSoundFont(track_id))),
                    components::label(text("Bank").into()),
                    self.number_selector(0, 127, synth.bank as u8, move |val: u8| { Message::Engine(Actions::Synth(SynthActions::SetBank(track_id, val as u32))) }),
                    components::label(text("Program").into()),
                    self.number_selector(0, 127, synth.program, move |val: u8| { Message::Engine(Actions::Synth(SynthActions::SetProgram(track_id, val))) }),
                ]
            }
        }
    }

    fn number_selector<F>(&self, min: u8, max: u8, current: u8, on_set: F ) -> Element<'_, Message> 
    where F: Fn(u8) -> Message + 'static {
        let options: Vec<u8> = (min..=max).collect();
        // let options = vec![min..max];
        pick_list(options, Some(current), on_set).into()
    }
}