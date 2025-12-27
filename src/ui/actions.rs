use std::path::PathBuf;

use iced::window;

use crate::models::sequences::Tick;
use crate::models:: shared::{RegionIdentifier, RegionType, TrackIdentifier};
use crate::engine::actions::Actions;

#[derive(Debug, Clone)]
pub enum Message {
    // Window event messages...
    WindowEvent(window::Event),
    // Engine Messages
    Engine(Actions),
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
    // For abandoning task chains
    Ignore,
}

#[derive(Debug, Clone)]
pub enum SynthMessage {
    SelectSoundFont(TrackIdentifier),
    SetSoundFont(TrackIdentifier, Option<PathBuf>),
}