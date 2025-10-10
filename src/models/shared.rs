/*
Defines the shared data structure btween all threads
*/
use super::sequences::{PatternSeq};
// use super::super::engine::buss::Output;

// pub trait Structure {
//     fn get_output(&self) -> Output;
// }
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

// impl Structure for SongData {
//     fn get_output(&self) -> Box<dyn Output> {
//         // For the moment, let's just return the first pattern
//         if self.patterns.is_empty() {
//             return None
//         }
//         self.patterns[0].to_event_stream()
//     }
// }