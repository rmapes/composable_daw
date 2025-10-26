use iced::{widget::{ row, Row }, Length};
use super::Message;

pub struct Component {
    height: f32,

}

impl Component {
    pub fn new(height: f32) -> Self {
        Self {height}
    } 

    pub fn update(&mut self, _msg: Message) {

    }
    pub fn view(&self) -> Row<'_, Message> {
        row![
        ]
        .width(Length::Fill)
        .height(Length::Fixed(self.height))
    }
}