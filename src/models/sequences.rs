use std::collections::{BTreeMap, HashMap};
use std::slice::Iter;

use log::debug;

use crate::models::shared::RegionIdentifier;




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

// Event Stream actually consists of a hashmap of ticks to events, 
// where each tick is mapped to a further hashmap of events by priority
pub trait EventStreamSource {
    fn to_event_stream(&self) -> EventStream;
}

pub struct EventStream{
    ppq: u32, //ppq = pulses per quarter = pulses per quarter beat = ticks per beat. 
    events: HashMap<u32, HashMap<EventPriority, Vec<MidiEventAt>>>,
    length_in_ticks: u32,
    no_events: Vec<MidiEventAt>,
}

impl EventStream{
    fn new(ppq: u32, length_in_ticks: u32) -> EventStream {
        EventStream {
            ppq,
            events: HashMap::new(),
            length_in_ticks: length_in_ticks,
            no_events: Vec::new(),
        }
    }
    /* Take ownership of event and add to event list */
    fn store_event(&mut self, event: MidiEventAt) {
        // Add event at its tick and priority
        let event_tick = event.get_event_time();
        let tick_block = self.events.entry(event_tick).or_default();
        let tick_priority_block = tick_block.entry(event.get_priority()).or_default();
        tick_priority_block.push(event);
    }
    // Return list of events at tick and priority
    pub fn get_events(&self, tick: u32, priority: EventPriority) -> &Vec<MidiEventAt> {
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
    pub fn get_length_in_ticks(&self) -> u32 {
        self.length_in_ticks
    }
    // Return length of ticks
    pub fn get_tick_duration(&self, bpm: u8) -> std::time::Duration {
        std::time::Duration::from_secs_f32(60.0_f32/(bpm as u32 * self.ppq) as f32)
    }
}

// Define a sequence trait, to specify common functions for all sequences
pub trait TSequence {
    fn length_in_ticks(&self) -> Tick;
}

#[derive(Clone)]
pub struct PatternSeq {
    pub id: RegionIdentifier,
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

    pub fn new(id: RegionIdentifier, ppq: u32) -> Self {
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


// create local midi type to mirror oxisynth midi event so we can make into a value type
type U7 = u8; // From oxisynth
type U14 = u16; // From oxisynth
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)] // We should implement the full spec now, despite not needing all of it yet
pub enum MidiEvent {
    /// Send a noteon message.
    NoteOn {
        channel: u8,
        key: U7,
        vel: U7,
    },
    /// Send a noteoff message.
    NoteOff {
        channel: u8,
        key: U7,
    },
    /// Send a control change message.
    ControlChange {
        channel: u8,
        ctrl: U7,
        value: U7,
    },
    AllNotesOff {
        channel: u8,
    },
    AllSoundOff {
        channel: u8,
    },
    /// Send a pitch bend message.
    PitchBend {
        channel: u8,
        value: U14,
    },
    /// Send a program change message.
    ProgramChange {
        channel: u8,
        program_id: U7,
    },
    /// Set channel pressure
    ChannelPressure {
        channel: u8,
        value: U7,
    },
    /// Set key pressure (aftertouch)
    PolyphonicKeyPressure {
        channel: u8,
        key: U7,
        value: U7,
    },
    /// Send a reset.
    ///
    /// A reset turns all the notes off and resets the controller values.
    ///
    /// Purpose:
    /// Respond to the MIDI command 'system reset' (0xFF, big red 'panic' button)
    SystemReset,
}

impl MidiEvent {
    pub fn to_oxisynth(&self) -> oxisynth::MidiEvent {
        match self {
            MidiEvent::NoteOn { channel, key, vel } => { oxisynth::MidiEvent::NoteOn { channel: *channel, key: *key, vel: *vel } },
            MidiEvent::NoteOff { channel, key } => { oxisynth::MidiEvent::NoteOff { channel: *channel, key: *key } },
            MidiEvent::ControlChange { channel, ctrl, value } => { oxisynth::MidiEvent::ControlChange { channel: *channel, ctrl: *ctrl, value: *value } },
            MidiEvent::AllNotesOff { channel } => { oxisynth::MidiEvent::AllNotesOff { channel: *channel } },
            MidiEvent::AllSoundOff { channel } => { oxisynth::MidiEvent::AllSoundOff { channel: *channel } },
            MidiEvent::PitchBend { channel, value } => { oxisynth::MidiEvent::PitchBend { channel: *channel, value: *value } },
            MidiEvent::ProgramChange { channel, program_id } => { oxisynth::MidiEvent::ProgramChange { channel: *channel, program_id: *program_id } },
            MidiEvent::ChannelPressure { channel, value } => { oxisynth::MidiEvent::ChannelPressure { channel: *channel, value: *value } },
            MidiEvent::PolyphonicKeyPressure { channel, key, value } => { oxisynth::MidiEvent::PolyphonicKeyPressure { channel: *channel, key: *key, value: *value } },
            MidiEvent::SystemReset => { oxisynth::MidiEvent::SystemReset },
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MidiEventAt {
    event: MidiEvent,
    ticks: Tick,
}

impl MidiEventAt {
    pub fn get_priority(&self) -> EventPriority {
        EventPriority::Audio
    }
    pub fn get_event_time(&self) ->  Tick {
        self.ticks
    }  
    pub fn to_midi(&self) -> oxisynth::MidiEvent {
        self.event.to_oxisynth()
    }  
    pub fn clone_at(&self, new_tick: u32) -> Self {
        Self {
            event: self.event, // Uses clone
            ticks: new_tick,
        }
    }
}

impl EventStreamSource for PatternSeq {
    fn to_event_stream(&self) -> EventStream {
        debug!("Operating on pattern with beats {} and notes {}",self.num_beats, self.num_notes);
        debug!("Container array has size {} * {}", self.pattern.len(), self.pattern[0].len());
        let ticks_per_beat = self.ppq / self.beats_per_quarter_note as u32; // sample rate = ticks per second
        let mut playing_notes = Vec::new();
        let mut event_stream = EventStream::new(self.ppq, self.length_in_ticks());
        for beat in 0..self.num_beats {
            let current_tick = (beat as u32) * ticks_per_beat;
            // Add events for note off
            for note in &playing_notes {
                event_stream.store_event(MidiEventAt {
                    event: MidiEvent::NoteOff { channel: 0, key: *note }, 
                    ticks: current_tick
                });
            }
            playing_notes.clear();
            // Now add new notes to play
            for note_num in 0..self.num_notes {
                // debug!("Note {note_num}, beat {beat}");
                let note = self.note_values[note_num as usize];
                if self.pattern[beat as usize][note_num as usize] {
                    event_stream.store_event(MidiEventAt {
                        event: MidiEvent::NoteOn { channel: 0, key: note, vel: 100 }, 
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
            event_stream.store_event(MidiEventAt {
                event: MidiEvent::NoteOff { channel: 0, key: *note }, 
                ticks: current_tick
            });
        }
        event_stream
    }    
}

// Implement midi seq 
#[derive(Debug, Copy, Clone)]
pub struct MidiNote {
    pub channel: u8,
    pub key: u8,
    pub velocity: u8,
    pub length: Tick,
}

#[derive(Clone)]
pub struct MidiSeq {
    pub id: RegionIdentifier,
    pub notes: BTreeMap<Tick, Vec<MidiNote>>,
    pub length: Tick,
    pub ppq: u32, /* ticks per quarter note */
}

impl MidiSeq {
    pub fn new(id: RegionIdentifier, ppq: u32) -> Self {
        Self { 
            id,
            notes: BTreeMap::new(), 
            length: ppq * 4,
            ppq,
        }
    }
}

impl TSequence for MidiSeq {
    fn length_in_ticks(&self) -> Tick {
        self.length
    }
}

impl EventStreamSource for MidiSeq {
    fn to_event_stream(&self) -> EventStream {
        let mut event_stream = EventStream::new(self.ppq, self.length_in_ticks());
        for current_tick in self.notes.keys() {
            for note in self.notes[current_tick].as_slice() {
                // Add event for note on
                event_stream.store_event(MidiEventAt {
                    event: MidiEvent::NoteOn { channel: note.channel, key: note.key, vel: note.velocity }, 
                    ticks: *current_tick,
                });
                // Add event for note off
                event_stream.store_event(MidiEventAt {
                    event: MidiEvent::NoteOff { channel: note.channel, key: note.key }, 
                    ticks: *current_tick + note.length
                });
            }
        }
        event_stream
    }    
}

// Implement Sequence Polymorphism

#[allow(dead_code)] // Possibly a YAGN, but we're anticipating needing SequenceContainer.
pub enum Sequence {
    Pattern(PatternSeq),
    Midi(MidiSeq),
    SequenceContainer(SequenceContainer)
}
                                                                                               
impl EventStreamSource for Sequence {
    fn to_event_stream(&self) -> EventStream {
        debug!("Picking Sequence to convert to event stream");
        match &self {
            Sequence::Pattern(seq) => seq.to_event_stream(),
            Sequence::Midi(seq) => seq.to_event_stream(),
            Sequence::SequenceContainer(seq) => seq.to_event_stream()
        }
    }
}

impl TSequence for Sequence {
    fn length_in_ticks(&self) -> Tick {
        match &self {
            Sequence::Pattern(seq) => seq.length_in_ticks(),
            Sequence::SequenceContainer(seq) => seq.length_in_ticks(),
            Sequence::Midi(seq) => seq.length_in_ticks(),
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
            if self.sequences.contains_key(&tick) 
                && self.sequences[&tick].length_in_ticks() > start_tick - tick {
                    return true;
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
    fn to_event_stream(&self) -> EventStream {
        let mut event_stream = EventStream::new(self.ppq, self.length_in_ticks());
        debug!("Converting {} sequences into events", self.sequences.len());
        for (offset, sequence) in  self.sequences.iter() {
            debug!("Operating on sequence at {}", offset);
            let sequence_events = sequence.to_event_stream();
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
        event_stream
    }
}




/////////////////////////
///  Tests
/// 

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::models::shared::TrackIdentifier;

    use super::*;

    // Sequences
    #[test]
    fn event_priority_has_iterator() {
        assert!(EventPriority::iter().len() > 0);
    }

    //////// Event stream
    #[test]
    fn event_stream_can_be_created() {
        let length_in_ticks = 960*4;
        let event_stream = EventStream::new(24000, length_in_ticks);
        assert_eq!(event_stream.get_length_in_ticks(), length_in_ticks)
    }

    #[test]
    fn event_stream_can_store_and_retreve_events() {
        let length_in_ticks = 960*4;
        let mut event_stream = EventStream::new(24000, length_in_ticks);
        let event = MidiEventAt { event: MidiEvent::SystemReset {}, ticks: 12 };
        let _ = event_stream.store_event(event);
        assert!(event_stream.get_events( 12, EventPriority::Audio).contains(&event)); // All midi events are currently Audio. TODO: distriguish
        // assert!(event_stream.get_events( 0, EventPriority::System).is_empty());  TODO: create tests showing that these calls panick, or make return is empty
        // assert!(event_stream.get_events( 12, EventPriority::Other).is_empty());
        // assert!(event_stream.get_events( 12, EventPriority::Audio).is_empty());
    }

    #[test]
    fn event_stream_can_calculate_tick_duration_for_a_given_bpm() {
        let length_in_ticks = 960*4;
        let bpm = 120;
        let event_stream = EventStream::new(960, length_in_ticks);
        assert_eq!(event_stream.get_tick_duration(bpm), Duration::from_secs_f32(60.0/(960.0 * bpm as f32)))
    }


    #[test]
    fn pattern_seq_can_be_created() {
        let length_in_ticks = 960*4;
        let pattern = PatternSeq::new(
            RegionIdentifier { track_id: TrackIdentifier { track_id: 1 }, region_id: 1 }, 
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
            RegionIdentifier { track_id: TrackIdentifier { track_id: 1 }, region_id: 1 }, 
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
            RegionIdentifier { track_id: TrackIdentifier { track_id: 1 }, region_id: 1 }, 
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
            RegionIdentifier { track_id: TrackIdentifier { track_id: 1 }, region_id: 1 }, 
            960);
        let event_stream = pattern.to_event_stream();
        assert_eq!(event_stream.get_length_in_ticks(), pattern.length_in_ticks()); // We should rationalise naming here.
    }
}