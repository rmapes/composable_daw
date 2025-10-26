use iced::widget::{ container, Container};
use iced::Element;
use super::style;

pub fn rack<Message> (content: Element<'_, Message>) -> Container<'_, Message>{
    container(content)
    .style(style::rack)
}


pub fn module<Message> (content: Element<'_, Message>) -> Container<'_, Message>{
    container(content)
    .style(style::module)
}

pub fn module_slot<Message> (content: Element<'_, Message>) -> Container<'_, Message>{
    container(content)
    .padding(3)
    .style(style::module_slot)
}

pub fn display<Message> (content: Element<'_, Message>) -> Container<'_, Message>{
    container(content)
    .style(style::display)
}

pub fn control<Message> (content: Element<'_, Message>) -> Container<'_, Message>{
    container(content)
    .style(style::control)
}
