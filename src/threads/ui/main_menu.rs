use iced::widget::{button, text};
use iced::Element;
use iced_aw::menu::Item;
use iced_aw::{Menu, menu_bar, menu_items};

use crate::models::shared::RegionType;

use super::actions::Message;

pub fn top_menu_view() -> Element<'static, Message>
{
    // Define menu levels
    let menu_tpl_1 = |items:Vec<Item<'static, Message, iced::Theme, iced::Renderer>>| Menu::new(items).width(180.0).offset(15.0).spacing(5.0);
    // let menu_tpl_2 = |items: Vec<Item<'static, Message, iced::Theme, iced::Renderer>>| Menu::new(items).width(180.0).offset(0.0).spacing(5.0);

    // Define your dropdown items
    // let file_menu = (
    //     text("File"),
    //     menu_tpl_1(menu_items!(
    //         (text("New"))
    //         (text("Open"))
    //         // (button(text("New")).on_press(Message::NewFile)),
    //         // (button(text("Open")).on_press(Message::OpenFile)),
    //     ))
    // );

    // let help_menu = Item::new(button(text("Help")).on_press(Message::ShowHelp));

    // Create the MenuBar
    menu_bar!(
        (
            text("File"),
            menu_tpl_1(menu_items!(
                // (text("New"))
                // (text("Open"))
                (button(text("New")).on_press(Message::NewFile)),
                (button(text("Open")).on_press(Message::OpenFile))
            ))
        ),
        (
            text("Edit"),
            menu_tpl_1(menu_items!(
                // (text("New"))
                // (text("Open"))
                (button(text("Add Pattern")).on_press(Message::AddRegionAtPlayhead(RegionType::Pattern))),
                (button(text("Add Midi")).on_press(Message::AddRegionAtPlayhead(RegionType::Midi))),
                (button(text("Delete Region")).on_press(Message::DeleteSelectedRegion))
            ))
        )
    ).into()
}
