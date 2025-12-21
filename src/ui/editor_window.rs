use iced::widget::row;
use iced::{Element, Length};

use crate::models::sequences::Sequence;


use super::actions::Message;
use super::midi_editor;
use super::pattern_editor;

pub struct Component {
    pattern_editor: pattern_editor::Component,
    midi_editor: midi_editor::Component,
}

impl Component {
    pub fn new(width: Length, height: Length) -> Self {
        Self {
            pattern_editor: pattern_editor::Component::new(width, height),
            midi_editor: midi_editor::Component::new(width, height),
        }
    } 

    pub fn view(&self, maybe_region: Option<&Sequence>) -> Element<'_, Message> {
        if let Some(region) = maybe_region {
            match region {
                Sequence::Pattern(pattern) =>  self.pattern_editor.view(pattern),
                Sequence::Midi(midi) => self.midi_editor.view(midi),
                Sequence::SequenceContainer(_sequence_container) => row![].into(),
            }
        } else {
            row![].into()
        }
    }

}
