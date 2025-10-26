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

pub struct SongData {
    // Components
    pub tracks: Vec<Track>,

}

impl SongData {
    pub fn new() -> Self {
        let mut this = Self {
            tracks: Vec::new(),
        };
        // Always start with one track
        this.new_track();
        this
    }

    pub fn new_track(&mut self) {
        // Add a new track, defaulting to name Track # where # is current position
        let new_track_num = self.tracks.len() + 1;
        let new_track = Track::new(format!("Track {new_track_num}"));
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