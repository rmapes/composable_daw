use iced::widget::{ row, column, Column, text };
use iced::{Length, Element};
use crate::models::components::Track;

use super::components;

use super::Message;

pub struct Component {
    width: f32,
    height: f32,
}

impl Component {
    pub fn new(width: f32, height: f32) -> Self {
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
            .width(Length::Fill)//Length::Fixed(self.width))
            .height(Length::Fill)//Length::Fixed(self.height))
            .into()
        ).into()
    }

    fn channel_strip(&self, track: &Track) -> Column<'_, Message> {
        column![
            components::control(text(track.name.clone()).into()),
        ]
    }
}