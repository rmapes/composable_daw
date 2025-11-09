use std::path::PathBuf;

use iced::window;

use crate::models::shared::{PatternNoteIdentifier, TrackIdentifier};


#[derive(Debug, Clone)]
pub enum Message {
    // Window event messages...
    WindowEvent(window::Event),
    PatternClickNote(PatternNoteIdentifier),
    Play,
    PlayStopped,
    AddTrack,
    SelectTrack(TrackIdentifier),
    Synth(SynthMessage),
}

#[derive(Debug, Clone)]
pub enum SynthMessage {
    SelectSoundFont(TrackIdentifier),
    SetSoundFont(TrackIdentifier, Option<PathBuf>),
    SetBank(TrackIdentifier, u32),
    SetProgram(TrackIdentifier, u8),
}