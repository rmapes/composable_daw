use std::{collections::HashMap};
use std::option::{Option};
use std::slice::Iter;

use log::debug;

use crate::models::shared::PatternIdentifier;




#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventPriority {
    System,
    Audio,
    Other
}

impl EventPriority {
    fn iter() -> Iter<'static, EventPriority> {
        static PRIORITIES: [EventPriority; 3] = [EventPriority::System, EventPriority::Audio, EventPriority::Other];
        PRIORITIES.iter()
    }
}

// Event time allows different representations of when an event should occur
// Ultimately, we will want to convert this to ticks based on a given sample rate of ticks per second
// In some implementations, we will already have converted u32o the final tick, for efficiency
// and sample_rate may be ignored. It is up to the system to ensure this never results in a sample_rate mismatch
pub trait EventTime {
    fn as_ticks(&self, sample_rate: u32) -> u32;
}

// Event Stream actually consists of a hashmap of ticks to events, 
// where each tick is mapped to a further hashmap of events by priority
pub trait EventStream {
    fn store_event(&mut self, event: MidiEvent);
    fn get_events(&self, tick: u32, priority: EventPriority) -> &Vec<MidiEvent>;
    fn get_length_in_ticks(&self) -> u32;
    fn get_tick_duration(&self, bpm: u8) -> std::time::Duration;
}

pub trait EventStreamSource {
    fn to_event_stream(&self) -> Option<Box<dyn EventStream>>;
}

///////////////////////
/// Concrete implementations
/// 
// Event Stream actually consists of a hashmap of ticks to events, 
// where each tick is mapped to a further hashmap of events by priority
struct BaseEventStream{
    ppq: u32, //ppq = pulses per quarter = pulses per quarter beat = ticks per beat. 
    events: HashMap<u32, HashMap<EventPriority, Vec<MidiEvent>>>,
    length_in_ticks: u32,
    no_events: Vec<MidiEvent>,
}

impl BaseEventStream{
    pub fn new(sample_rate: u32, length_in_ticks: u32) -> BaseEventStream {
        BaseEventStream {
            ppq: sample_rate,
            events: HashMap::new(),
            length_in_ticks: length_in_ticks,
            no_events: Vec::new(),
        }
    }
}

impl EventStream for BaseEventStream {
    /* Take ownership of event and add to event list */
    fn store_event(&mut self, event: MidiEvent) {
        // Add event at its tick and priority
        let event_tick = event.get_event_time().as_ticks(self.ppq);
        let tick_block = self.events.entry(event_tick).or_default();
        let tick_priority_block = tick_block.entry(event.get_priority()).or_default();
        tick_priority_block.push(event);
    }
    // Return list of events at tick and priority
    fn get_events(&self, tick: u32, priority: EventPriority) -> &Vec<MidiEvent> {
        if self.events.contains_key(&tick) {
            let tick_block = self.events.get(&tick).expect("Tick {tick} not found in events");
            if tick_block.contains_key(&priority) {
                return tick_block.get(&priority)
                .expect("Priority {priority} not found in tick_block");
            }
        }
        &self.no_events
    }
    // Return length in ticks
    fn get_length_in_ticks(&self) -> u32 {
        self.length_in_ticks
    }
    // Return length of ticks
    fn get_tick_duration(&self, bpm: u8) -> std::time::Duration {
        std::time::Duration::from_secs_f32(60.0_f32/(bpm as u32 * self.ppq) as f32)
    }

}

// Define a sequence trait, to specify common functions for all sequences
pub trait TSequence {
    fn length_in_ticks(&self) -> Tick;
}

#[derive(Clone)]
pub struct PatternSeq {
    pub id: PatternIdentifier,
    pub note_values: Vec<u8>,
    pub num_notes: u8,
    pub num_beats: u8,
    pub bpm: u8,
    pub pattern: Vec<Vec<bool>>,
    pub ppq: u32, /* ticks per quarter note */
    pub beats_per_quarter_note: u8,
}

impl PatternSeq {
    pub fn is_on(&self, beat_num: u8, note_num: u8) -> &bool {
        self.pattern.get(beat_num as usize).and_then(|notes| {notes.get(note_num as usize)})
        .expect("Attempt to access pattern out of range")
    }

    pub fn toggle_on(&mut self, beat_num: u8, note_num: u8) {
        self.pattern[beat_num as usize][note_num as usize] = !self.pattern[beat_num as usize][note_num as usize];
    }

    pub fn new(id: PatternIdentifier, ppq: u32) -> Self {
        let note_values = vec![72,71,69,67,65,64,62,60];
        let num_notes = note_values.len() as u8;
        let num_beats = 16;
        let pattern = (0..num_beats).map(|_| { (0..num_notes).map(|_| {false}).collect() }).collect();
        let bpm= 120; // TODO: Get from Project
        let beats_per_quarter_note = 4;
        Self { 
            id,
            note_values, 
            num_notes, 
            num_beats, 
            bpm, 
            pattern, 
            ppq,
            beats_per_quarter_note,
        }
    }
}

impl TSequence for PatternSeq {
    fn length_in_ticks(&self) -> Tick {
        self.num_beats as u32 * self.ppq / self.beats_per_quarter_note as u32
    }
}


// TODO: stop using oxisynth midi event
struct RawEventTime {
    ticks: u32
}

impl EventTime for RawEventTime {
    fn as_ticks(&self, _sample_rate: u32) -> u32 {
        self.ticks
    }
}

pub struct MidiEvent {
    event: oxisynth::MidiEvent,
    ticks: u32
}

impl MidiEvent {
    pub fn get_priority(&self) -> EventPriority {
        EventPriority::Audio
    }
    pub fn get_event_time(&self) -> Box<dyn EventTime> {
        Box::new(RawEventTime{ ticks: self.ticks })
    }  
    pub fn to_midi(&self) -> oxisynth::MidiEvent {
        self.event
    }  
    pub fn clone_at(&self, new_tick: u32) -> Self {
        Self {
            event: self.event, // Uses clone
            ticks: new_tick,
        }
    }
}

impl EventStreamSource for PatternSeq {
    fn to_event_stream(&self) -> Option<Box<dyn EventStream>> {
        debug!("Operating on pattern with beats {} and notes {}",self.num_beats, self.num_notes);
        debug!("Container array has size {} * {}", self.pattern.len(), self.pattern[0].len());
        let beats_per_minute: u32 = self.beats_per_quarter_note as u32 * self.bpm as u32;
        let ticks_per_beat = self.ppq * 60 / beats_per_minute; // sample rate = ticks per second
        let mut playing_notes = Vec::new();
        let mut event_stream = BaseEventStream::new(self.ppq, self.length_in_ticks());
        for beat in 0..self.num_beats {
            let current_tick = (beat as u32) * ticks_per_beat;
            // Add events for note off
            for note in &playing_notes {
                event_stream.store_event(MidiEvent {
                    event: oxisynth::MidiEvent::NoteOff { channel: 0, key: *note }, 
                    ticks: current_tick
                });
            }
            playing_notes.clear();
            // Now add new notes to play
            for note_num in 0..self.num_notes {
                // debug!("Note {note_num}, beat {beat}");
                let note = self.note_values[note_num as usize];
                if self.pattern[beat as usize][note_num as usize] {
                    event_stream.store_event(MidiEvent {
                        event: oxisynth::MidiEvent::NoteOn { channel: 0, key: note, vel: 100 }, 
                        ticks: current_tick
                    });
                    playing_notes.push(note);  
                }
            }
        }
        // Turn off all final notes
        let current_tick = (self.num_beats as u32) * ticks_per_beat;
        // Add events for note off
        for note in &playing_notes {
            event_stream.store_event(MidiEvent {
                event: oxisynth::MidiEvent::NoteOff { channel: 0, key: *note }, 
                ticks: current_tick
            });
        }
        Some(Box::new(event_stream))
    }    
}

// Implement Sequence Polymorphism

pub enum Sequence {
    Pattern(PatternSeq),
    SequenceContainer(SequenceContainer)
}
                                                                                               
impl EventStreamSource for Sequence {
    fn to_event_stream(&self) -> Option<Box<dyn EventStream>> {
        debug!("Picking Sequence to convert to event stream");
        match &self {
            Sequence::Pattern(seq) => seq.to_event_stream(),
            Sequence::SequenceContainer(seq) => seq.to_event_stream()
        }
    }
}

impl TSequence for Sequence {
    fn length_in_ticks(&self) -> Tick {
        match &self {
            Sequence::Pattern(seq) => seq.length_in_ticks(),
            Sequence::SequenceContainer(seq) => seq.length_in_ticks()
        }
    }
}


/// Sequence Container: one container to rule them all
/// 
pub type Tick = u32;

pub struct SequenceContainer {
    pub sequences: HashMap<Tick, Sequence>,
    ppq: u32,
}

impl SequenceContainer {
    pub fn new(ppq: u32)-> Self {
        Self {
            sequences: HashMap::new(),
            ppq
        }
    }

    pub fn region_collides_with_existing(&self, start_tick: Tick, length: Tick) -> bool {
        // Check start not in preceding region
        for tick in 0..start_tick {
            if self.sequences.contains_key(&tick) {
                if self.sequences[&tick].length_in_ticks() > start_tick - tick {
                    return true;
                }
            }
        }
        // Check no region starts in this region
        let end_tick = start_tick + length;
        for tick in start_tick..end_tick {
            if self.sequences.contains_key(&tick) {
                return true;
            }
        }
        false
    }

}

impl TSequence for SequenceContainer {
    fn length_in_ticks(&self) -> Tick {
        let last_sequence_start = self.sequences.keys().max().unwrap_or(&0);
        let last_length = self.sequences[last_sequence_start].length_in_ticks();
        last_sequence_start + last_length
    }
}

impl EventStreamSource for SequenceContainer {
    fn to_event_stream(&self) -> Option<Box<dyn EventStream>> {
        let mut event_stream = BaseEventStream::new(self.ppq, self.length_in_ticks());
        debug!("Converting {} sequences into events", self.sequences.len());
        let _ = self.sequences.iter().for_each(|(offset, sequence)| {
            debug!("Operating on sequence at {}", offset);
            if let Some(sequence_events) = sequence.to_event_stream() {
                debug!("Processing substream events");
                // Check whether we need to resample due to different sample rates
                // we want 1 second in source sequence = 1 second in target
                // so 1 tick in source = tick duration => n ticks in target where n = tick duration * sample rate
                let tick_ratio = if event_stream.ppq == self.ppq { 1.0 } else { event_stream.ppq as f64 / self.ppq as f64 };
                for tick in 0..sequence_events.get_length_in_ticks() {
                    for priority in EventPriority::iter() {
                        let new_tick = (tick as f64 * tick_ratio).round() as u32 + offset;
                        for event in sequence_events.get_events(tick, *priority) {
                            // debug!("Event at {}", tick);
                            let new_event = event.clone_at(new_tick);
                            event_stream.store_event(new_event);
                        }                        
                    }
                }
            }
        });
        Some(Box::new(event_stream))
    }
}




/////////////////////////
///  Tests
/// 

#[cfg(test)]
mod tests {
    use crate::models::shared::TrackIdentifier;

    use super::*;

    // Sequences
    #[test]
    fn event_priority_has_iterator() {
        assert!(EventPriority::iter().len() > 0);
    }

    #[test]
    fn event_stream_can_be_created() {
        let length_in_ticks = 960*4;
        let event_stream = BaseEventStream::new(24000, length_in_ticks);
        assert_eq!(event_stream.get_length_in_ticks(), length_in_ticks)
    }


    #[test]
    fn pattern_seq_can_be_created() {
        let length_in_ticks = 960*4;
        let pattern = PatternSeq::new(
            PatternIdentifier { track_id: TrackIdentifier { track_id: 1 }, pattern_id: 1 }, 
            960);
        assert_eq!(pattern.ppq, 960);
        assert_eq!(pattern.beats_per_quarter_note, 4);
        assert_eq!(pattern.num_beats, 16);
        assert_eq!(960*16/4, length_in_ticks);
        assert_eq!(pattern.length_in_ticks(), length_in_ticks);
    }

    #[test]
    fn pattern_seq_all_beats_are_initially_off() {
        let pattern = PatternSeq::new(
            PatternIdentifier { track_id: TrackIdentifier { track_id: 1 }, pattern_id: 1 }, 
            960);
        for note in 0..pattern.num_notes {
            for beat in 0..pattern.num_beats {
                assert_eq!(pattern.is_on(beat, note), &false)
            }
        }
    }

    #[test]
    fn pattern_seq_can_turn_beats_on_and_off() {
        let mut pattern = PatternSeq::new(
            PatternIdentifier { track_id: TrackIdentifier { track_id: 1 }, pattern_id: 1 }, 
            960);
        let beat=3;
        let note = 5;
        assert_eq!(pattern.is_on(beat, note), &false);
        pattern.toggle_on(beat, note);
        assert_eq!(pattern.is_on(beat, note), &true);
        pattern.toggle_on(beat, note);
        assert_eq!(pattern.is_on(beat, note), &false);
    }

    #[test]
    fn pattern_seq_can_create_event_stream() {
        let pattern = PatternSeq::new(
            PatternIdentifier { track_id: TrackIdentifier { track_id: 1 }, pattern_id: 1 }, 
            960);
        let event_stream = pattern.to_event_stream();
        assert!(event_stream.is_some()); // TODO: Get rid of Option wrapper. Not needed
        let event_stream = event_stream.unwrap(); // Shouldn't panic here due to test above
        assert_eq!(event_stream.get_length_in_ticks(), pattern.length_in_ticks()); // We should rationalise naming here.
    }
}