use crate::models::sequences::Tick;

/*
Defines the shared data structure btween all threads
*/
use super::components::*;

// Everything in here represents configuration.
// This will be used to generate actual generators, processors, busses and buffers in the audio layer.
// The only exception to this is that each track will expose a pair of atomic integers representing the instantaneous
// left and right audio buffer values.
// These metric values allow display of vu meters, spectrograms and other visualisations.

// For the moment, tracks will own all routing, fx, virtual instruments etc.
// A more complete system might allow routing to be external to the tracks, and for tracks to share fx and instruments
// However, we want to minimise complexity at this point, and we can simulate external routing with busses and sends.

// the only problems this presents are:
//    - do we need explici

////////
/// Value objects to identify structures stored within Project Data
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct TrackIdentifier {
    pub track_id: usize,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct RegionIdentifier {
    pub track_id: TrackIdentifier,
    pub region_id: Tick,
}
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PatternNoteIdentifier {
    pub region_id: RegionIdentifier,
    pub note_num: u8,
    pub beat_num: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum RegionType {
    Pattern,
    Midi,
    //Audio
}

////////////
/// Data that will be stored to file
const DEFAULT_PPQ: u32 = 960;
#[derive(Debug, Clone)]
pub struct ProjectData {
    // Components
    pub tracks: Vec<Track>,
    // Tempo and Measures
    pub bpm: u8,
    pub ppq: u32,
}

impl ProjectData {
    pub fn new() -> Self {
        let mut this = Self {
            tracks: Vec::new(),
            bpm: 120,
            ppq: DEFAULT_PPQ,
        };
        // Always start with one track
        this.new_track();
        this
    }

    pub fn reset(&mut self) {
        // should mirror new
        self.tracks = Vec::new();
        self.new_track();
    }

    pub fn ticks_per_second(&self) -> u32 {
        self.ppq * self.bpm as u32 / 60
    }

    pub fn new_track(&mut self) -> TrackIdentifier {
        // Add a new track, defaulting to name Track # where # is current position
        let new_track_num = self.tracks.len() + 1;
        let id = TrackIdentifier {
            track_id: new_track_num - 1,
        };
        let mut new_track = Track::new(id, format!("Track {new_track_num}"), self.ppq);
        // Temporary until we can add regions via UI. Add pattern at start
        new_track
            .add_midi_region_at(0)
            .expect("Unexpected collision inserting into empty sequence");
        self.tracks.push(new_track);
        id
    }

    pub fn get_track_by_id(&mut self, id: &TrackIdentifier) -> &mut Track {
        &mut self.tracks[id.track_id]
    }
}

// impl Structure for SongData {
//     fn get_output(&self) -> Box<dyn Output> {
//         // For the moment, let's just return the first pattern
//         if self.patterns.is_empty() {
//             return None
//         }
//         self.patterns[0].to_event_stream()
//     }
// }
