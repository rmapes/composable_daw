use std::path::PathBuf;

use iced::window;

use crate::models::{sequences::Tick, shared::{PatternIdentifier, PatternNoteIdentifier, TrackIdentifier}};


#[derive(Debug, Clone)]
pub enum Message {
    // Window event messages...
    WindowEvent(window::Event),
    PatternClickNote(PatternNoteIdentifier),
    Play,
    PlayStopped,
    AddTrack,
    SelectTrack(TrackIdentifier),
    SelectPattern(PatternIdentifier, bool), 
    DeselectAllPatterns(),
    DeleteSelectedPattern(),
    Synth(SynthMessage),
    // Playhead
    SetPlayhead(Tick),
    // Menu
    NewFile,
    OpenFile,
    ShowHelp,
}

#[derive(Debug, Clone)]
pub enum SynthMessage {
    SelectSoundFont(TrackIdentifier),
    SetSoundFont(TrackIdentifier, Option<PathBuf>),
    SetBank(TrackIdentifier, u32),
    SetProgram(TrackIdentifier, u8),
}