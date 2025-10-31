use iced::{widget::{ button, text, row, Row }, Length};
use super::actions::Message;

pub struct Component {
    width: Length,
    height: Length,

}

impl Component {
    pub fn new(width: Length, height: Length) -> Self {
        Self {width, height}
    } 

    pub fn view(&self) -> Row<'_, Message> {
        row![
            button( text("Play")).on_press(Message::Play)
        ]
        .width(self.width)
        .height(self.height)
    }
}