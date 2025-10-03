/*
Defines the shared data structure btween all threads
*/
use super::sequences::{PatternSeq, Sequence, EventStream};
use std::option::{Option};

pub struct SongData {
    pub patterns: Vec<PatternSeq>,

}
impl SongData {
    pub fn new() -> SongData {
        SongData {
            patterns: Vec::new(),
        }
    }
}

impl Sequence for SongData {
    fn to_event_stream(&self) -> Option<Box<dyn EventStream>> {
        // For the moment, let's just return the first pattern
        if self.patterns.is_empty() {
            return None
        }
        self.patterns[0].to_event_stream()
    }
}