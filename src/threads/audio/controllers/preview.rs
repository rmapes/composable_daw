use std::{thread, time::Duration};

use crate::models::{sequences::MidiNote, shared::TrackIdentifier};

use super::{MidiInputMessage, MidiSendersMap};

/// Request for the preview thread: play this note on this track for this many milliseconds.
pub type PreviewRequest = (TrackIdentifier, MidiNote, u32);


pub(crate) fn spawn_preview_thread(
    midi_senders: MidiSendersMap,
) -> flume::Sender<PreviewRequest> {
    let (preview_tx, preview_rx) = flume::unbounded::<PreviewRequest>();
    thread::spawn(move || {
        while let Ok((track_id, note, duration_ms)) = preview_rx.recv() {
            let Some(tx) = midi_senders.read().ok().and_then(|g| g.get(&track_id).cloned()) else {
                continue;
            };
            let _ = tx.send(MidiInputMessage::MidiEvent(oxisynth::MidiEvent::NoteOn {
                channel: note.channel,
                key: note.key,
                vel: note.velocity,
            }));
            thread::sleep(Duration::from_millis(duration_ms as u64));
            let _ = tx.send(MidiInputMessage::MidiEvent(oxisynth::MidiEvent::NoteOff {
                channel: note.channel,
                key: note.key,
            }));
        }
    });
    preview_tx
}
