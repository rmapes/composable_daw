use std::path::PathBuf;

use iced::window;

use crate::models::{sequences::{MidiNote, Tick}, shared::{PatternNoteIdentifier, RegionIdentifier, RegionType, TrackIdentifier}};
use crate::engine::actions::Actions;

#[derive(Debug, Clone)]
pub enum Message {
    // Window event messages...
    WindowEvent(window::Event),
    // Engine Messages
    Engine(Actions),
    PatternClickNote(PatternNoteIdentifier),
    GoToStart,
    AddTrack,
    SelectTrack(TrackIdentifier),
    SelectRegion(RegionIdentifier, bool), 
    DeselectAllRegions(),
    AddRegionAtPlayhead(RegionType),
    AddRegionAt(TrackIdentifier, Tick, RegionType),
    DeleteSelectedRegion(),
    Synth(SynthMessage),
    // Midi Editor
    CreateMidiNote(RegionIdentifier, Tick, MidiNote),
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
