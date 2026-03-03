use std::path::PathBuf;

use iced::window;

use super::super::engine::actions::Actions;
use crate::models::sequences::Tick;
use crate::models::shared::{ProjectData, RegionIdentifier, RegionType, TrackIdentifier};

#[derive(Debug, Clone)]
#[allow(dead_code)] // SelectRegion and CancelRegionDrag are constructed from composer_window and main_window
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
    /// Click on a region without dragging (press + release within threshold).
    RegionClick(RegionIdentifier),
    // Region drag. When threshold passed: (region_id, initial_mouse_x, current_mouse_x, current_mouse_y).
    StartRegionDrag(RegionIdentifier, f32, f32, f32),
    UpdateRegionDrag(f32, f32),
    EndRegionDrag,
    CancelRegionDrag,
    DeleteSelectedRegion,
    DeselectAllRegions(),
    // Playhead
    SetPlayhead(Tick),
    AddRegionAtPlayhead(RegionType),
    Tick,
    // Synth
    Synth(SynthMessage),
    // Instrument editor
    OpenInstrumentEditor(TrackIdentifier),
    CloseInstrumentEditor,
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
