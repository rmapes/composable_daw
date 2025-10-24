use std::rc::Rc;

use slint::{VecModel, ModelRc, Model, SharedString };

use crate::models::sequences::PatternSeq;
use crate::models::shared::SongData;

slint::include_modules!();

/**********
 *  Utility functions
 */
fn vec_to_model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    let the_model : Rc<VecModel<T>> =
        Rc::new(VecModel::from(v));
    // Convert it to a ModelRc.
    ModelRc::from(the_model.clone())
}

/*********
 * Slint model adaptors
 */

impl Song {
    pub fn new() -> Song {
        Song {
            tracks: vec_to_model(vec![Track::new_midi()])
        }
    }

    pub fn sync_from(&mut self, source: &SongData) {
        self.tracks = vec_to_model(source.tracks.iter().map(|track| {Track::from(track)}).collect());
    }
}

impl TrackSettings {
    pub fn new(name: &str) -> TrackSettings {
        TrackSettings {
            name: SharedString::from(name)
        }
    }
}

impl Track {
    pub fn new_midi() -> Track {
        Track {
            trackType: TrackType::Midi,
            midi_content: MidiContent::new(),
            audio_content: AudioContent::new(),
            settings: TrackSettings::new("Track 1"),
        }
    }

    pub fn from(track: &crate::models::components::Track) -> Self {
        // For the moment only support midi tracks
        let mut midi_content = MidiContent::new();
        let patterns: Vec<(Pattern, i32)>;
        if let Some(midi) = &track.midi {
            patterns = midi.sequences.iter()
                .filter(|&(_,sequence)| {
                    if let crate::models::sequences::Sequence::Pattern(_) = sequence {
                        true
                    } else {
                        false
                    }
                })
                .map(|(tick, sequence)| {
                    let pattern = match sequence  {
                        crate::models::sequences::Sequence::Pattern(pattern) => Pattern::from_pattern_seq(&pattern),
                        _ => Pattern::new() // Shouldn't get here. Consider throwing exception
                    };
                    (pattern, *tick as i32)
                })
                .collect();
        } else {
            patterns = Vec::new();
        }
        midi_content.patterns = vec_to_model(patterns);
        Self {
            trackType: TrackType::Midi,
            midi_content: midi_content,
            audio_content: AudioContent::new(),
            settings: TrackSettings::new(&track.name),           
        }
    }
}

impl AudioContent {
    pub fn new() -> AudioContent {
        AudioContent {  }
    }
}

impl MidiContent {
    pub fn new() -> MidiContent {
        MidiContent {
            sequences: vec_to_model(vec![]),
            patterns: vec_to_model(vec![(Pattern::new(), 0)]),
        }
    }    
}

impl Pattern {
    pub fn new() -> Self {
        let pattern_notes = vec![72,71,69,67,65,64,62,60];
        let num_notes: i32 = pattern_notes.len() as i32;
        let num_beats = 16;
        let pattern = vec_to_model((0..num_beats).map(|_| { vec_to_model((0..num_notes).map(|_| {false}).collect())}).collect());
        Self { r#beats: num_beats, r#note_values: vec_to_model(pattern_notes), r#notes: num_notes, pattern }
    }
    pub fn from_pattern_seq(pattern: &PatternSeq) -> Pattern {
        Pattern {
            note_values: vec_to_model(pattern.note_values.iter().map(|&n| {n as i32}).collect()),
            pattern: vec_to_model(pattern.pattern.iter().map(|row: &Vec<bool>| {vec_to_model(row.to_owned())}).collect()),
            notes: pattern.num_notes as i32,
            beats: pattern.num_beats as i32,
        }
    }
    pub fn to_pattern_seq(&self) -> PatternSeq {
        let contained_pattern: Vec<Vec<bool>> = self.pattern.iter().map(|row| {
            row.iter().collect()
        }).collect();
        let local_note_values: Vec<u8> = self.note_values.iter().map(|n| n as u8).collect();
         PatternSeq { 
                note_values: local_note_values,
                pattern: contained_pattern,
                num_notes: self.notes as u8,
                num_beats: self.beats as u8,
                bpm: 120,
                sample_rate: 960,
        }
    }
}

