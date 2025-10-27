use iced::{widget::{ row, Row }, Length};
use super::actions::Message;

pub struct Component {
    width: Length,
    height: Length,

}

impl Component {
    pub fn new(width: Length, height: Length) -> Self {
        Self {width, height}
    } 

    pub fn update(&mut self, _msg: Message) {

    }
    pub fn view(&self) -> Row<'_, Message> {
        row![
        ]
        .width(self.width)
        .height(self.height)
    }
}