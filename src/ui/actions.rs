use iced::window;

use crate::models::shared::PatternNoteIdentifier;


#[derive(Debug, Clone)]
pub enum Message {
    // Window event messages...
    WindowEvent(window::Event),
    PatternClickNote(PatternNoteIdentifier),
    Play,
    PlayStopped,
    AddTrack,
}