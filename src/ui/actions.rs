use std::path::PathBuf;

use iced::window;

use crate::models::{sequences::Tick, shared::{RegionIdentifier, PatternNoteIdentifier, RegionType, TrackIdentifier}};


#[derive(Debug, Clone)]
pub enum Message {
    // Window event messages...
    WindowEvent(window::Event),
    PatternClickNote(PatternNoteIdentifier),
    Play,
    Stop,
    GoToStart,
    AddTrack,
    SelectTrack(TrackIdentifier),
    SelectRegion(RegionIdentifier, bool), 
    DeselectAllRegions(),
    AddRegionAtPlayhead(RegionType),
    AddRegionAt(TrackIdentifier, Tick, RegionType),
    DeleteSelectedRegion(),
    Synth(SynthMessage),
    // Playhead
    SetPlayhead(Tick),
    Tick,
    // Menu
    NewFile,
    OpenFile,
    // For abandoning task chains
    Ignore,
}

#[derive(Debug, Clone)]
pub enum SynthMessage {
    SelectSoundFont(TrackIdentifier),
    SetSoundFont(TrackIdentifier, Option<PathBuf>),
    SetBank(TrackIdentifier, u32),
    SetProgram(TrackIdentifier, u8),
}