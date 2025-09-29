use std::{collections::HashMap, cmp::max};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventPriority {
    SYSTEM,
    AUDIO,
    OTHER
}

// Event time allows different representations of when an event should occur
// Ultimately, we will want to convert this to ticks based on a given sample rate of ticks per second
// In some implementations, we will already have converted u32o the final tick, for efficiency
// and sample_rate may be ignored. It is up to the system to ensure this never results in a sample_rate mismatch
pub trait EventTime {
    fn as_ticks(&self, sample_rate: u32) -> u32;
}

pub trait Event {
    fn get_priority(&self) -> EventPriority;
    fn get_event_time(&self) -> Box<dyn EventTime>;
    fn to_midi(&self) -> oxisynth::MidiEvent;
}

// Event Stream actually consists of a hashmap of ticks to events, 
// where each tick is mapped to a further hashmap of events by priority
pub trait EventStream {
    fn store_event(&mut self, event: Box<dyn Event>);
    fn get_events(&self, tick: u32, priority: EventPriority) -> &Vec<Box<dyn Event>>;
    fn get_length_in_ticks(&self) -> u32;
    fn get_tick_duration(&self) -> std::time::Duration;
}

pub trait Sequence {
    fn to_event_stream(&self) -> Box<dyn EventStream>;
}

///////////////////////
/// Concrete implementations
/// 
// Event Stream actually consists of a hashmap of ticks to events, 
// where each tick is mapped to a further hashmap of events by priority
struct BaseEventStream{
    sample_rate: u32,
    events: HashMap<u32, HashMap<EventPriority, Vec<Box::<dyn Event>>>>,
    length_in_ticks: u32,
    no_events: Vec<Box::<dyn Event>>,
}

impl BaseEventStream{
    pub fn new(sample_rate: u32) -> BaseEventStream {
        BaseEventStream {
            sample_rate,
            events: HashMap::new(),
            length_in_ticks: 0,
            no_events: Vec::new(),
        }
    }
}

impl EventStream for BaseEventStream {
    /* Take ownership of event and add to event list */
    fn store_event(&mut self, event: Box<dyn Event>) {
        // Add event at its tick and priority
        let event_tick = event.get_event_time().as_ticks(self.sample_rate);
        if !self.events.contains_key(&event_tick) {
            self.events.insert(event_tick, HashMap::new());
        }
        let tick_block = self.events.get_mut(&event_tick).unwrap();
        if !tick_block.contains_key(&event.get_priority()) {
            tick_block.insert(event.get_priority(), Vec::new());
        }
        let tick_priority_block = tick_block.get_mut(&event.get_priority()).unwrap();
        tick_priority_block.push(event);
        self.length_in_ticks = max(self.length_in_ticks, event_tick);
    }
    // Return list of events at tick and priority
    fn get_events(&self, tick: u32, priority: EventPriority) -> &Vec<Box<dyn Event>> {
        if self.events.contains_key(&tick) {
            let tick_block = self.events.get(&tick).expect("Tick {tick} not found in events");
            if tick_block.contains_key(&priority) {
                let tick_priority_block = tick_block.get(&priority).expect("Priority {priority} not found in tick_block");
                return &tick_priority_block
            }
        }
        return &self.no_events;
    }
    // Return length in ticks
    fn get_length_in_ticks(&self) -> u32 {
        return self.length_in_ticks
    }
    // Return length of ticks
    fn get_tick_duration(&self) -> std::time::Duration {
        return std::time::Duration::from_secs_f32(1.0_f32/self.sample_rate as f32);
    }

}


pub struct PatternSeq {
    pub note_values: Vec<u8>,
    pub num_notes: u8,
    pub num_beats: u8,
    pub bpm: u8,
    pub pattern: Vec<Vec<bool>>,
    pub sample_rate: u32, /* ticks per second */
}

// TODO: stop using oxisynth midi event
struct RawEventTime {
    ticks: u32
}

impl EventTime for RawEventTime {
    fn as_ticks(&self, _sample_rate: u32) -> u32 {
        return self.ticks;
    }
}

pub struct MidiEvent {
    event: oxisynth::MidiEvent,
    ticks: u32
}

impl Event for MidiEvent {
    fn get_priority(&self) -> EventPriority {
        return EventPriority::AUDIO;
    }
    fn get_event_time(&self) -> Box<dyn EventTime> {
        return Box::new(RawEventTime{ ticks: self.ticks })
    }  
    fn to_midi(&self) -> oxisynth::MidiEvent {
        return self.event.clone();
    }  
}

impl Sequence for PatternSeq {
    fn to_event_stream(&self) -> Box<dyn EventStream> {
        let ticks_per_beat = self.sample_rate * 60 / self.bpm as u32;
        let mut playing_notes = Vec::new();
        let mut event_stream = BaseEventStream::new(self.sample_rate);
        for beat in 0..self.num_beats {
            let current_tick = (beat as u32) * ticks_per_beat;
            // Add events for note off
            for note in &playing_notes {
                event_stream.store_event(Box::new(MidiEvent {
                    event: oxisynth::MidiEvent::NoteOff { channel: 0, key: *note }, 
                    ticks: current_tick
                }));
            }
            playing_notes.clear();
            // Now add new notes to play
            for note_num in 0..self.num_notes {
                let note = self.note_values[note_num as usize];
                if self.pattern[beat as usize][note_num as usize] {
                    event_stream.store_event(Box::new(MidiEvent {
                        event: oxisynth::MidiEvent::NoteOn { channel: 0, key: note, vel: 100 }, 
                        ticks: current_tick
                    }));
                    playing_notes.push(note);  
                }
            }
        }
        // Turn off all final notes
        let current_tick = (self.num_beats as u32) * ticks_per_beat;
        // Add events for note off
        for note in &playing_notes {
            event_stream.store_event(Box::new(MidiEvent {
                event: oxisynth::MidiEvent::NoteOff { channel: 0, key: *note }, 
                ticks: current_tick
            }));
        }
        return Box::new(event_stream)
    }    
}