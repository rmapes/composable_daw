use iced::widget::{ row };
use iced::{Length, Element};
use super::actions::Message;

use super::components;

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
    pub fn view(&self) -> Element<'_, Message> {
        components::module(
            row![
            ]
            .width(self.width)
            .height(self.height).into()
        ).into()
    }
}