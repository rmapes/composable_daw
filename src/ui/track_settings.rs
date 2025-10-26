use iced::widget::{ row, Row, column, Column, text };
use iced::Length;
use crate::models::components::Track;

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
    pub fn view(&self, track: &Track) -> Row<'_, Message> {
        row![
            self.channel_strip(track),
        ]
        .width(Length::Fixed(self.width))
        .height(Length::Fixed(self.height))
    }

    fn channel_strip(&self, track: &Track) -> Column<'_, Message> {
        column![
            text(track.name.clone()),
        ]
    }
}