use std::sync::LazyLock;

use iced::Length;
use iced::widget::{Button, button, image};

use super::super::actions::Message;

// This file predefines the systems application items, and allows them to be embedded in the application

// 1. Define the icons to be embedded in the application
// The path is relative to this file.
const PLAY_ICON_BYTES: &[u8] = include_bytes!("../../../../icons/play.png");
const STOP_ICON_BYTES: &[u8] = include_bytes!("../../../../icons/stop.png");
const REWIND_TO_START_ICON_BYTES: &[u8] = include_bytes!("../../../../icons/rewind_to_start.png");

// 2. Create the Handle from the bytes
struct IconHandles {
    pub play: image::Handle,
    pub stop: image::Handle,
    pub rewind_to_start: image::Handle,
}

pub enum Icon {
    Play,
    Stop,
    RewindToStart,
}

impl IconHandles {
    pub fn new() -> Self {
        IconHandles {
            // Handle::from_bytes expects the raw, encoded image data (PNG/JPG/etc.)
            play: image::Handle::from_bytes(PLAY_ICON_BYTES.to_vec()),
            stop: image::Handle::from_bytes(STOP_ICON_BYTES.to_vec()),
            rewind_to_start: image::Handle::from_bytes(REWIND_TO_START_ICON_BYTES.to_vec()),
        }
    }
}

static ICON_HANDLES_MAP: LazyLock<IconHandles> = LazyLock::new(IconHandles::new);

fn image_button<'a>(handle: image::Handle) -> Button<'a, Message> {
    button(
        image(handle)
            .width(Length::Fixed(20.0)) // Adjust size as needed
            .height(Length::Fixed(20.0)),
    )
}

pub fn icon_button<'a>(icon: Icon) -> Button<'a, Message> {
    let handle = match icon {
        Icon::Play => ICON_HANDLES_MAP.play.clone(),
        Icon::Stop => ICON_HANDLES_MAP.stop.clone(),
        Icon::RewindToStart => ICON_HANDLES_MAP.rewind_to_start.clone(),
    };
    image_button(handle)
}
