use crate::models::sequences::{Sequence, Tick};

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
#[derive(Debug, Clone, Copy)]
pub struct TrackIdentifier {
    pub track_id: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct PatternIdentifier {
    pub track_id: TrackIdentifier,
    pub pattern_id: Tick,
}
#[derive(Debug, Clone, Copy)]
pub struct PatternNoteIdentifier {
    pub pattern_id: PatternIdentifier,
    pub note_num: u8,
    pub beat_num: u8,
}

////////////
/// Data that will be stored to file
pub struct ProjectData {
    // Components
    pub tracks: Vec<Track>,
    // Regionss
    pub regions: Vec<Sequence>,

}

impl ProjectData {
    pub fn new() -> Self {
        let mut this = Self {
            tracks: Vec::new(),
            regions: Vec::new(),
        };
        // Always start with one track
        this.new_track();
        this
    }

    pub fn new_track(&mut self) {
        // Add a new track, defaulting to name Track # where # is current position
        let new_track_num = self.tracks.len() + 1;
        let mut new_track = Track::new(TrackIdentifier {track_id: new_track_num - 1}, format!("Track {new_track_num}"));
        // Temporary until we can add regions via UI. Add pattern at start
        new_track.add_pattern_at(0).expect("Unexpected collision inserting into empty sequence");
        self.tracks.push(new_track);
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