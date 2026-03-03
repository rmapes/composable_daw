use crate::models::instuments::InstrumentActions;
use crate::models::sequences::{MidiNote, Tick};
use crate::models::shared::{PatternNoteIdentifier, RegionIdentifier, RegionType, TrackIdentifier};

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
    MoveRegion(RegionIdentifier, TrackIdentifier, Tick),
    DeleteRegion(RegionIdentifier),
    // Pattern Actions
    PatternClickNote(PatternNoteIdentifier),
    PreviewMidiNote(TrackIdentifier, MidiNote),
    // Midi Editor
    CreateMidiNote(RegionIdentifier, Tick, MidiNote),
    DeleteMultipleMidiNotes(RegionIdentifier, Vec<(Tick, usize)>), // region, vec of (start_tick, note_index) pairs
    UpdateMidiNote(RegionIdentifier, Tick, usize, Tick, MidiNote), // region, old_start_tick, note_index, new_start_tick, updated_note
    // Instrument Actions
    Instrument(TrackIdentifier, InstrumentActions),
    // Actions local to engine itself
    Internal(SystemActions),
}

#[derive(Debug, Clone)]
pub enum SystemActions {
    SamplesPlayed(usize),
    SetSampleRate(u32),
}
