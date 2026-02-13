use std::path::PathBuf;

use iced::window;

use crate::models::sequences::Tick;
use crate::models::shared::{ProjectData, RegionIdentifier, RegionType, TrackIdentifier};
use super::super::engine::actions::Actions;

#[derive(Debug, Clone)]
pub enum Message {
    // Window event messages...
    WindowEvent(window::Event),
    // Engine Callback Message
    ProjectDataChanged(ProjectData),
    // Send To Engine Messages
    Engine(Actions),
    // Local Messages
    GoToStart,
    SelectTrack(TrackIdentifier),
    SelectRegion(RegionIdentifier, bool), 
    DeleteSelectedRegion,
    DeselectAllRegions(),
    // Playhead
    SetPlayhead(Tick),
    AddRegionAtPlayhead(RegionType),
    Tick,
    // Synth
    Synth(SynthMessage),
    // Menu
    NewFile,
    OpenFile,
    // Midi Editor
    MidiEditor(super::midi_editor::MidiEditorMessage),
}

#[derive(Debug, Clone)]
pub enum SynthMessage {
    SelectSoundFont(TrackIdentifier),
    SetSoundFont(TrackIdentifier, Option<PathBuf>),
}