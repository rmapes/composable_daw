use std::collections::BTreeMap;
use std::thread;
use std::time::Instant;

use crate::models::{sequences::MidiNote, shared::TrackIdentifier};

use super::{MidiInputMessage, MidiSendersMap};

/// Request for the preview thread: play this note on this track for this many milliseconds.
pub type PreviewRequest = (TrackIdentifier, MidiNote, u32);

/// Message to the preview thread: either a preview request or a clock tick.
#[derive(Clone)]
pub enum PreviewMessage {
    Request(PreviewRequest),
    Clock,
}

/// Pending NoteOff: track and (channel, key) for reconstructing oxisynth::MidiEvent::NoteOff.
type PendingNoteOff = (TrackIdentifier, u8, u8);

pub(crate) fn spawn_preview_thread(midi_senders: MidiSendersMap) -> flume::Sender<PreviewMessage> {
    let (preview_tx, preview_rx) = flume::unbounded::<PreviewMessage>();
    thread::spawn(move || {
        let start = Instant::now();
        let mut pending_note_offs: BTreeMap<u64, Vec<PendingNoteOff>> = BTreeMap::new();

        while let Ok(msg) = preview_rx.recv() {
            match msg {
                PreviewMessage::Request((track_id, note, duration_ms)) => {
                    let Some(tx) = midi_senders
                        .read()
                        .ok()
                        .and_then(|g| g.get(&track_id).cloned())
                    else {
                        continue;
                    };
                    let _ = tx.send(MidiInputMessage::MidiEvent(oxisynth::MidiEvent::NoteOn {
                        channel: note.channel,
                        key: note.key,
                        vel: note.velocity,
                    }));
                    let now_ms = start.elapsed().as_millis() as u64;
                    let note_off_at = now_ms + (duration_ms as u64);
                    pending_note_offs.entry(note_off_at).or_default().push((
                        track_id,
                        note.channel,
                        note.key,
                    ));
                }
                PreviewMessage::Clock => {
                    let now_ms = start.elapsed().as_millis() as u64;
                    let to_drain: Vec<_> = pending_note_offs
                        .range(..=now_ms)
                        .map(|(&t, _)| t)
                        .collect();
                    for time_key in to_drain {
                        if let Some(entries) = pending_note_offs.remove(&time_key)
                            && let Ok(senders) = midi_senders.read()
                        {
                            for (track_id, channel, key) in entries {
                                if let Some(tx) = senders.get(&track_id) {
                                    let _ = tx.send(MidiInputMessage::MidiEvent(
                                        oxisynth::MidiEvent::NoteOff { channel, key },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    preview_tx
}
