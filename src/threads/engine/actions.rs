use std::path::PathBuf;

use crate::models::shared::{PatternNoteIdentifier, RegionIdentifier, RegionType, TrackIdentifier};
use crate::models::sequences::{MidiNote, Tick};

#[derive(Debug, Clone)]
pub enum Actions {
    Play,
    Pause,
    Quit,
    // Project Actions
    NewFile,
    // Track Actions
    AddTrack,
    AddRegionAt(TrackIdentifier, Tick, RegionType),
    DeleteRegion(RegionIdentifier),
    // Pattern Actions
    PatternClickNote(PatternNoteIdentifier),
    // Midi Editor
    CreateMidiNote(RegionIdentifier, Tick, MidiNote),
    DeleteMidiNote(RegionIdentifier, Tick, usize), // region, start_tick, note_index
    DeleteMultipleMidiNotes(RegionIdentifier, Vec<(Tick, usize)>), // region, vec of (start_tick, note_index) pairs
    UpdateMidiNote(RegionIdentifier, Tick, usize, Tick, MidiNote), // region, old_start_tick, note_index, new_start_tick, updated_note
    // Synthesizer Actions
    Synth(SynthActions),
    // Actions local to engine itself
    Internal(SystemActions),
}

#[derive(Debug, Clone)]
pub enum SystemActions {
    SamplesPlayed(usize),
    SetSampleRate(u32),

}

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)] // Set is not part of enum name
pub enum SynthActions {
    SetSoundFont(TrackIdentifier, Option<PathBuf>),
    SetBank(TrackIdentifier, u32),
    SetProgram(TrackIdentifier, u8),
}

