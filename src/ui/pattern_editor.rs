use iced::widget::{ row, Row };
use iced::Length;
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
    pub fn view(&self) -> Row<'_, Message> {
        row![
        ]
        .width(Length::Fixed(self.width))
        .height(Length::Fixed(self.height))
    }
}