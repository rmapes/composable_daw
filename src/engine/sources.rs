use std::{collections::HashMap, thread::JoinHandle};

use log::info;

use crate::{engine::{audio::AudioEngine, synth::{TrackThread, TrackThreadEvents}}, models::{components::Track, instuments::Instrument, shared::TrackIdentifier}};
use crate::models::sequences::EventStreamSource;
/**
 * Threads for tracks and instruments
 * These will feed into the audio thread
 */

 pub struct AudioSources {
    audio: AudioEngine,
    tracks: HashMap<TrackIdentifier, JoinHandle<()>>,
    tick_source: crossbeam_channel::Receiver<TrackThreadEvents>,
 }

 impl AudioSources {
    pub fn new(audio: AudioEngine, tick_source: crossbeam_channel::Receiver<TrackThreadEvents>, tracks: &Vec<Track>) -> Self {
        info!("Creating new Audio Source Controller with {} tracks", tracks.len());
        let mut this = Self {
            audio,
            tracks: HashMap::new(),
            tick_source,
        };
        for track in tracks { let _ = this.add_track(track); }
        this
    }
    pub fn add_track(&mut self, track: &Track) -> Result<(), &str> {
        info!("Adding track {}", track.id.track_id);
        match &track.instrument.kind {
            Instrument::Synth(instrument) => {
                if let Some(seq) = &track.midi {
                    let track_thread = TrackThread::new(
                            track.id,
                            seq, 
                            self.audio.sample_rate, 
                            &instrument.get_soundfont_path(), 
                            instrument.bank, 
                            instrument.program
                        );
                    self.audio.add_input(track_thread.synth.clone());
                    let handle = track_thread.run(self.tick_source.clone());
                    self.tracks.insert(track.id, handle);
                    Ok(())
                } else {
                    Err("Not midi")
                }
            }
        }
    }

    pub fn update_track(&mut self, track: &Track, event_sender: crossbeam_channel::Sender<TrackThreadEvents>) -> Result<(), &str> {
        info!("Sending update for track {}", track.id.track_id);
        if let Some(seq) = &track.midi {
            let event_stream = seq.to_event_stream();
            event_sender.send(TrackThreadEvents::Update(track.id, event_stream)).expect("Failed to send update");
            Ok(())
        } else {
            Err("Not midi")
        }
    }
}