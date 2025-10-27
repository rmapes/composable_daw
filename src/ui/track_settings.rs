use iced::widget::{ row, column, Column, text };
use iced::{Length, Element};
use crate::models::components::Track;

use super::components;
use super::actions::Message;

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

    pub fn update(&mut self, _msg: Message) {

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
            components::control(text(track.name.clone()).into()),
        ]
    }
}