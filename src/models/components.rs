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

use crate::models::instuments::{Instrument, SimpleSynth};
use crate::models::sequences::{MidiSeq, PatternSeq, Sequence, SequenceContainer, TSequence, Tick};
use crate::models::shared::{RegionIdentifier, TrackIdentifier};

pub struct VirtualInstrument {
    pub kind: Instrument,
}

impl Default for VirtualInstrument {
    fn default() -> Self {
        Self {
            kind: Instrument::Synth(SimpleSynth::default()),
        }
    }
}


/*** Containers ****/

pub struct Track {
    /////////////////
    /// Fixed structures
    /// 
    /// 
    // Inputs
    // audio_input: AudioBuss, 
    // Input needs to be a buss to support Buss tracks. 
    // Alernative is to allow busses to act as independent entities and each track accept only a single input.
    // This allows routing of busses to multiple outputs via
    // a wiring diagram, but doesn't really give much more flexibility over using sends



    // Generator pipelines
    pub midi: Option<SequenceContainer>,
    pub instrument: VirtualInstrument,

    // Outputs
    /// Sends + Returns
    /// Volume/Fader
    /// Pan
    /// Output connector
    // pub audio_output: AudioConnector,

    // Track Metadata
    pub id: TrackIdentifier,
    pub name: String,
    pub ppq: u32, // TODO: Pass this in as a global settings object

}

#[derive(Debug, Clone)]
pub struct CollisionError;

impl Error for CollisionError {
}

impl fmt::Display for CollisionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "overlapping regions not allowed")
    }
}


impl Track {
    pub fn new(id: TrackIdentifier, name: String, ppq: u32) -> Self {
        Self {
            id,
            name,
            midi: Some(SequenceContainer::new(ppq)), // If track type == midi
            instrument: VirtualInstrument::default(),
            ppq,
        }
    }

    pub fn length_in_ticks(&self) -> Tick {
        self.midi.as_ref().map(|s| s.length_in_ticks()).unwrap_or(0)
    }

    pub fn duration(&self, ticks_per_second: u32) -> Duration {
        // Find last sequence
        // 15000 * pattern.num_beats as u64 / pattern.bpm as u64;
        let length_in_ticks = self.length_in_ticks() as f32;
        Duration::from_secs_f32(length_in_ticks / ticks_per_second as f32)
    }

    pub fn add_midi_region_at(&mut self, tick: Tick) -> Result<(), CollisionError> {
        let sequence = self.midi.get_or_insert(SequenceContainer::new(self.ppq));
        let midi = MidiSeq::new(RegionIdentifier{ track_id: self.id, region_id: tick }, self.ppq);
        // Check for collisions
        if sequence.region_collides_with_existing(tick, midi.length_in_ticks()) {
            return Err(CollisionError {});
        }
        let region = Sequence::Midi(midi);
        sequence.sequences.insert(tick, region);
        Ok(())
    }

    pub fn get_midi_by_id(&mut self, id: &RegionIdentifier) -> &mut MidiSeq {
        let container = &mut self.midi.as_mut().unwrap().sequences;
        let region: &mut Sequence = container
            .get_mut(&id.region_id)
            .expect("Attempt to access region with invalid id");
        // region
        if let Sequence::Midi(seq) = region {
            seq
        } else {
            panic!("Attempt to access non-midi region as midi")
        }
    }

    pub fn add_pattern_at(&mut self, tick: Tick) -> Result<(), CollisionError> {
        let sequence = self.midi.get_or_insert(SequenceContainer::new(self.ppq));
        let pattern = PatternSeq::new(RegionIdentifier{ track_id: self.id, region_id: tick }, self.ppq);
        // Check for collisions
        if sequence.region_collides_with_existing(tick, pattern.length_in_ticks()) {
            return Err(CollisionError {});
        }
        let region = Sequence::Pattern(pattern);
        sequence.sequences.insert(tick, region);
        Ok(())
    }

    pub fn get_pattern_by_id(&mut self, id: &RegionIdentifier) -> &mut PatternSeq {
        let container = &mut self.midi.as_mut().unwrap().sequences;
        let region: &mut Sequence = container
            .get_mut(&id.region_id)
            .expect("Attempt to access pattern with invalid id");
        // region
        if let Sequence::Pattern(pattern) = region {
            pattern
        } else {
            panic!("Attempt to access non-pattern region as pattern")
        }
    }

    pub fn delete_pattern(&mut self, id: &RegionIdentifier) {
        let container = &mut self.midi.as_mut().unwrap().sequences;
        let _ = container
            .remove(&id.region_id)
            .expect("Attempt to access pattern with invalid id");
    }
}


