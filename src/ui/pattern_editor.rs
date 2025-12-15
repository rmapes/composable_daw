use iced::widget::{ row, text, Column, Container, Row, button };
use iced::{Length, Element};
use crate::models::sequences::PatternSeq;
use crate::models::shared::{ RegionIdentifier, PatternNoteIdentifier};

use super::actions::Message;

use super::components;
use super::style;

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

    pub fn view(&self, maybe_pattern: Option<&PatternSeq>) -> Element<'_, Message> {
        let content: Vec<Element<Message>> = if let Some(pattern) = maybe_pattern {
            (0..pattern.num_notes)
                .map(|note_num| pattern_editor_row(pattern, note_num).into())
                .collect() // Collect into a temporary Vec
        } else {
            vec![] // An empty vec if no pattern
        };
        let content = Column::with_children(content);
        components::module(
            content
            .width(self.width)
            .height(self.height).into()
        ).into()
    }
}

fn pattern_editor_row(pattern: &PatternSeq, note_num: u8) -> Row<'static, Message> {
    // 1. Create a Vector of dynamic elements
    let beat_toggles: Vec<Element<'static, Message>> = (0..pattern.num_beats)
    .map(|beat_num| note_toggle_button(pattern.id, note_num, beat_num, pattern.is_on(beat_num, note_num)).into())
    .collect();

    // 2. Create the label element
    let label = note_label(
    pattern.note_values.get(note_num as usize).expect("Unexpected index into note_values")
    );

    // 3. Combine the label and the collected vector into the final Row
    let toggles = Row::with_children(beat_toggles);
    // Return the element
    row![
        label,
        toggles
    ]
}

fn note_label(midi_pitch: &u8) -> Element<'static, Message> {
    components::label(
       text(format!("{midi_pitch}")).width(Length::Fixed(20.0)).height(Length::Fixed(20.0)).into(),
    ).into()
}

fn note_toggle_button(pattern_id: RegionIdentifier, note_num: u8, beat_num: u8, is_on: &bool) -> Container<'static, Message> {   
    let style = if *is_on { style::note_button_on } else { style::note_button_off };
    components::control(
       button(
        Container::new(row![]).width(Length::Fixed(20.0)).height(Length::Fixed(20.0)).style(style)
       ).on_press(Message::PatternClickNote(PatternNoteIdentifier {region_id: pattern_id, note_num, beat_num} )).into(),
    )
}
