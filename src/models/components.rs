/*
All of the components that make up the structure of a 'song'

- Tracks
- Regions (can audio and midi be mixed?)
  - Midi
    - Sequence
    - Pattern
  - Audio
    - Sequence
    - Pattern
- Busses
- FX
- Routing
  - midi outputs to audio generators
  - audio generators to busses
  - audio generators to FX
  - buss to stereo out
  - (inputs to generators / recorders)
- General / Settings
*/


/*** Generators ****/

use std::{error::Error, time::Duration};
use std::fmt;

use crate::models::sequences::{self, PatternSeq, Sequence, SequenceContainer, TSequence, Tick};
use crate::models::shared::{PatternIdentifier, TrackIdentifier};
pub trait AudioGenerator {
}

pub trait MidiGenerator {

}

pub trait MidiConsumer {

}

pub trait AudioConsumer {

}

pub struct VirtualInstrument {

}

/*** Conduits ****/

// Midi Connector
pub struct MidiConnector {
    // Connects a single input to a single output
    // Used to allow configurable routing, and to hide implementation details of e.g. tracks
}

impl MidiConnector {

}

impl MidiConsumer for MidiConnector {

}

impl MidiGenerator for MidiConnector {
    
}

// Audio Connector
pub struct AudioConnector {
    // Connects a single input to a single output
    // Used to allow configurable routing, and to hide implementation details of e.g. tracks
}

impl AudioConnector {

}

impl AudioConsumer for AudioConnector {

}

impl AudioGenerator for AudioConnector {
    
}

// Audo Buss (many to one)
pub struct AudioBuss {
    // Merges multiple inputs to a single output
    // Used to allow configurable routing, and merging to a final stereo out
}

impl AudioBuss {

}

impl AudioConsumer for AudioBuss {

}

impl AudioGenerator for AudioBuss {
    
}

// Audo Splitter (many to one)
pub struct AudioSplitter {
    // Splits a single input to multiple outputs
    // Used to allow configurable routing
}

impl AudioSplitter {

}

impl AudioConsumer for AudioSplitter {

}

impl AudioGenerator for AudioSplitter {
    
}

/*** Containers ****/

pub struct Track {
    /////////////////
    /// Fixed structures
    /// 
    /// 
    // Inputs
    audio_input: AudioBuss, 
    // Input needs to be a buss to support Buss tracks. 
    // Alernative is to allow busses to act as independent entities and each track accept only a single input.
    // This allows routing of busses to multiple outputs via
    // a wiring diagram, but doesn't really give much more flexibility over using sends



    // Generator pipelines
    pub midi: Option<SequenceContainer>,

    // Outputs
    /// Sends + Returns
    /// Volume/Fader
    /// Pan
    /// Output connector
    pub audio_output: AudioConnector,

    // Track Metadata
    pub id: TrackIdentifier,
    pub name: String,

}

#[derive(Debug, Clone)]
pub struct CollisionError;

impl Error for CollisionError {
   fn description(&self) -> &str {
        "overlapping regions not allowed"
    }
}

impl fmt::Display for CollisionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "overlapping regions not allowed")
    }
}


impl Track {
    pub fn new(id: TrackIdentifier, name: String) -> Self {
        Self {
            id,
            name,
            midi: Some(SequenceContainer::new()), // If track type == midi
            audio_input: AudioBuss {  },
            audio_output:  AudioConnector { },
        }
    }

    pub fn duration(&self, ticks_per_second: u32) -> Duration {
        // Find last sequence
        // 15000 * pattern.num_beats as u64 / pattern.bpm as u64;
        let length_in_ticks = self.midi.as_ref().map(|s| s.length_in_ticks()).unwrap_or(0) as f32;
        Duration::from_secs_f32(length_in_ticks / ticks_per_second as f32)
    }

    pub fn add_pattern_at(&mut self, tick: Tick) -> Result<(), CollisionError> {
        let sequence = self.midi.get_or_insert(SequenceContainer::new());
        let pattern = PatternSeq::new(PatternIdentifier{ track_id: self.id, pattern_id: tick });
        // Check for collisions
        if sequence.region_collides_with_existing(tick, pattern.length_in_ticks()) {
            return Err(CollisionError {});
        }
        let region = Sequence::Pattern(pattern);
        sequence.sequences.insert(tick, region);
        Ok(())
    }

    pub fn get_pattern_by_id(&mut self, id: &PatternIdentifier) -> &mut PatternSeq {
        let container = &mut self.midi.as_mut().unwrap().sequences;
        let region: &mut Sequence = container
            .get_mut(&id.pattern_id)
            .expect("Attempt to access pattern with invalid id");
        // region
        if let Sequence::Pattern(pattern) = region {
            pattern
        } else {
            panic!("Attempt to access non-pattern region as pattern")
        }
    }

}


