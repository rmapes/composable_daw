use iced::{widget::{ row, Row }, Length};
use super::actions::Message;
use super::components::icons::{icon_button, Icon};
use super::super::engine::actions::Actions::{Play, Pause};

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
            icon_button( Icon::RewindToStart).on_press(Message::GoToStart),
            icon_button( Icon::Stop ).on_press(Message::Engine(Pause)),
            icon_button( Icon::Play ).on_press(Message::Engine(Play))
        ]
        .width(self.width)
        .height(self.height)
    }
}