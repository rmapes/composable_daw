use iced::widget::row;
use iced::{Element, Length};

use crate::models::sequences::Sequence;


use super::actions::Message;
use super::midi_editor;
use super::pattern_editor;

#[derive(Debug, Clone)]
pub struct Component {
    pattern_editor: pattern_editor::Component,
    midi_editor: midi_editor::Component,
    snap_to_grid: midi_editor::SnapToGrid,
}

impl Component {
    pub fn new(width: Length, height: Length) -> Self {
        Self {
            pattern_editor: pattern_editor::Component::new(width, height),
            midi_editor: midi_editor::Component::new(width, height),
            snap_to_grid: midi_editor::SnapToGrid::None,
        }
    }


    pub fn view(&self, maybe_region: Option<&Sequence>, snap_to_grid: midi_editor::SnapToGrid, midi_offset: u8) -> Element<'_, Message> {
        if let Some(region) = maybe_region {
            match region {
                Sequence::Pattern(pattern) =>  self.pattern_editor.view(pattern),
                Sequence::Midi(midi) => self.midi_editor.view(midi, snap_to_grid, midi_offset),
                Sequence::SequenceContainer(_sequence_container) => row![].into(),
            }
        } else {
            row![].into()
        }
    }


}
