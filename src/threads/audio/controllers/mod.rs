/*
 * Controllers for the audio thread.
 *
 * Controllers are responsible for triggering actions on audio sources.
 */

pub mod preview;
pub mod stereo_output;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use crate::models::sequences::Tick;
use crate::models::shared::TrackIdentifier;

/// MIDI input to an instrument: from region playback (tick) or from preview/raw input (event).
/// The instrument (synth) is a single consumer of MIDI from multiple sources.
#[derive(Clone)]
pub enum MidiInputMessage {
    /// Region playback: process events at this tick from the track's event stream.
    RegionTick(Tick),
    /// Direct MIDI event (preview, future: raw MIDI input buss).
    MidiEvent(oxisynth::MidiEvent),
}


 /// Shared map of per-track MIDI senders so the preview thread can inject note_on/note_off.
pub type MidiSendersMap = Arc<RwLock<HashMap<TrackIdentifier, flume::Sender<MidiInputMessage>>>>;
