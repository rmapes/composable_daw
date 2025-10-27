use iced::widget::{ row };
use iced::{Length, Element};
use super::actions::Message;

use super::components;

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
    pub fn view(&self) -> Element<'_, Message> {
        components::module(
            row![
            ]
            .width(Length::Fixed(self.width))
            .height(Length::Fixed(self.height)).into()
        ).into()
    }
}