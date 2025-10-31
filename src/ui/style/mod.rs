use iced::{border, color, Background, Color, Theme};
use iced::widget::container::Style; 

// These colours use a slightly red hue to simulate an iron based metal casing
const LIGHT_GRAY: Color = color!(0x968B86);
const DARK_GRAY: Color = color!(0x7D7570);

const CASING_BLACK: Color = color!(0x1F1C1B);
const DISPLAY_BLACK: Color = color!(0x040414);//Color::from_rgb8(4, 4, 20);
const CONTROL_BLACK: Color = color!(0x131404);//Color::from_rgb8(19, 20, 4);


const WHITE_TEXT: Color = color!(0xD8EBE9);//Color::from_rgb8(216, 235, 233);
const GREEN_TEXT: Color = color!(0x39CC54);//Color::from_rgb8(57, 204, 84);
const BLUE_TEXT: Color = color!(0x63D6CD);//Color::from_rgb8(99, 214, 205);



pub fn rack(_: &Theme) -> Style {
    Style {
        // Set the background color
        background: Some(Background::Color(LIGHT_GRAY)),
        text_color: Some(WHITE_TEXT),
        ..Style::default()
    }
}

pub fn module_slot(_: &Theme) -> Style {
    Style {
        // Set the background color
        background: Some(Background::Color(LIGHT_GRAY)),
        text_color: Some(WHITE_TEXT),
        border: border::rounded(5).width(2).color(CASING_BLACK),
        ..Style::default()
    }
}

pub fn module(_: &Theme) -> Style {
    Style {
        // Set the background color
        background: Some(Background::Color(DARK_GRAY)),
        text_color: Some(WHITE_TEXT),
        border: border::rounded(5).width(2).color(LIGHT_GRAY),
        ..Style::default()
    }
}

pub fn display(_: &Theme) -> Style {
    Style {
        // Set the background color
        background: Some(Background::Color(DISPLAY_BLACK)),
        text_color: Some(BLUE_TEXT),
        ..Style::default()
    }
}

pub fn control(_: &Theme) -> Style {
    Style {
        // Set the background color
        background: Some(CONTROL_BLACK.into()),
        text_color: Some(GREEN_TEXT),
        border: border::rounded(2),
        ..Style::default()
    }
}

pub fn label(_: &Theme) -> Style {
    Style {
        // Set the background color
        background: Some(CONTROL_BLACK.into()),
        text_color: Some(BLUE_TEXT),
        ..Style::default()
    }
}

pub fn border(_: &Theme) -> Style {
    Style {
        // Set the background color
        background: Some(CONTROL_BLACK.into()),
        border: border::width(2),
        ..Style::default()
    }
}

// Specific styles
pub fn note_button_on(_: &Theme) -> Style {
    Style {
        // Set the background color
        background: Some(Background::Color(color!(0xff0050))),
        border: border::width(1).color(DARK_GRAY),
        ..Style::default()
    }
}

pub fn note_button_off(_: &Theme) -> Style {
    Style {
        // Set the background color
        background: Some(Background::Color(color!(0x00ff00))),
        border: border::width(1).color(DARK_GRAY),
        ..Style::default()
    }
}

/* 
Palette:
Rack colour: colour of components simulating raised metal framework, that panels are inset into
Module colour: colour of a pluggable module (e.g. channel strip with embedded controls and display)
Display colour: colour simulating a display screen (e.g. meters, or led displays)
Control colour

Rack

- Top level container included Top command ribbon and transport control. Simulates a modular audio device that other modules can be mounted into.
- Should look like a stamped metal case, with holes through which the modules can be seen. This can be achieved by setting the background colour of
- the container to RACK and implementing all module slots to use RACK_RECESS to simulate an inset bevelled edge.
- Note the illusion will be slightly broken by having resizable module slots. However, this is an acceptable trade off for the configurability.

Module Slot:

- A container for a module.
- In the first instance, will be fixed to contain specific modules, but in the future new slots can be created, moved and resized, and different modules allocated to different slots.

Module (channel strips, composer window, editor window)

- Should look like a slightly recessed audio unit with casing. To help with the inset effect, the MODULE case colour should be slightly darker than RACK, although Module Slot can take care of the recessing itelf.
- Modules can contain Controls, Labels and Displays
*/