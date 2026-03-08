use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use log::info;

use super::audio::controllers::{MidiInputMessage, MidiSendersMap};
use super::audio::sources::synth::{InstrumentRegistry, TrackSynth};
use super::audio::{
    AudioEngine, buss::Buss, controllers::stereo_output::StereoOutputController, interfaces::Output,
};
use crate::models::instrument::InstrumentAction;
use crate::models::sequences::{EventStreamSource, Tick};
use crate::models::{components::Track, shared::TrackIdentifier};

/**
 * Manages audio sources (synths) in the engine thread.
 * Each synth receives MIDI via a channel (region ticks + preview/raw events).
 */
pub struct AudioSources {
    audio: AudioEngine,
    stereo_output: StereoOutputController,
    final_buss: Buss,
    tracks: HashMap<TrackIdentifier, Rc<RefCell<TrackSynth>>>,
    midi_senders: MidiSendersMap,
    registry: Arc<InstrumentRegistry>,
}

impl AudioSources {
    pub fn new(
        audio: AudioEngine,
        stereo_output: StereoOutputController,
        tracks: &Vec<Track>,
        midi_senders: MidiSendersMap,
        registry: Arc<InstrumentRegistry>,
    ) -> Self {
        info!(
            "Creating new Audio Source Controller with {} tracks",
            tracks.len()
        );
        let mut this = Self {
            audio,
            stereo_output,
            final_buss: Buss::new(),
            tracks: HashMap::new(),
            midi_senders,
            registry,
        };
        for track in tracks {
            let _ = this.add_track(track);
        }
        this
    }

    pub fn add_track(&mut self, track: &Track) -> Result<(), &str> {
        info!("Adding track {}", track.id.track_id);
        let Some(seq) = &track.midi else {
            return Err("Not midi");
        };
        let default_config = self.registry.default_config(&track.instrument.kind);
        let config: &dyn crate::models::instrument::InstrumentConfig = match &track.instrument.config {
            Some(c) => c.as_ref(),
            None => default_config
                .as_ref()
                .ok_or("unknown instrument kind")?
                .as_ref(),
        };
        let (midi_tx, midi_rx) = flume::unbounded();
        let track_synth = self
            .registry
            .create_track_synth(
                &track.instrument.kind,
                track.id,
                seq,
                self.audio.sample_rate,
                config,
                midi_rx,
            )
            .map_err(|_| "create_track_synth failed")?;
        if let Ok(mut senders) = self.midi_senders.write() {
            senders.insert(track.id, midi_tx);
        }
        let track_synth_rc = Rc::new(RefCell::new(track_synth));
        let wrapper = Rc::clone(&track_synth_rc);
        self.final_buss
            .add_input(Box::new(RefCellOutputWrapper { inner: wrapper }));
        self.tracks.insert(track.id, track_synth_rc);
        Ok(())
    }

    pub fn update_track(&mut self, track: &Track) -> Result<(), &str> {
        info!("Updating track {}", track.id.track_id);
        if let Some(seq) = &track.midi {
            let event_stream = seq.to_event_stream();
            if let Some(track_synth) = self.tracks.get(&track.id) {
                track_synth.borrow_mut().update_event_stream(event_stream);
            }
            Ok(())
        } else {
            Err("Not midi")
        }
    }

    /// Process a tick: send RegionTick to each track's MIDI input, then fill ring buffer.
    /// Synths drain their MIDI channel (region + preview events) when generating audio.
    pub fn on_tick(&mut self, tick: Tick) {
        if let Ok(senders) = self.midi_senders.read() {
            for (_track_id, tx) in senders.iter() {
                let _ = tx.send(MidiInputMessage::RegionTick(tick));
            }
        }
        self.stereo_output.on_tick(&mut self.final_buss);
    }

    /// Fill the ring buffer only (no RegionTick). Use when stopped so preview/other MIDI still sounds.
    pub fn fill_buffer(&mut self) {
        self.stereo_output.on_tick(&mut self.final_buss);
    }

    /// True if the ring buffer has capacity so the engine can fill it (for playback and preview).
    pub fn has_buffer_capacity(&self) -> bool {
        self.stereo_output.has_capacity()
    }

    /// Check if playback should stop (all event streams have been processed past their last note off)
    /// This checks if we've reached or passed the maximum end tick of all event streams.
    /// Event streams end with note off events, so once we've processed past the end, all notes are off.
    pub fn should_stop_playback(&self, current_tick: Tick) -> bool {
        if self.tracks.is_empty() {
            return false;
        }

        // Find the maximum end tick across all tracks
        let max_end_tick = self
            .tracks
            .values()
            .map(|track_synth| {
                track_synth
                    .borrow()
                    .get_event_stream()
                    .get_length_in_ticks()
            })
            .max()
            .unwrap_or(0);

        // Stop playback once we've reached or passed the maximum end tick
        // Event streams end with note off events, so by this point all notes should be off
        current_tick >= max_end_tick
    }

    /// Handle instrument actions (e.g. synth soundfont, bank, program changes)
    pub fn handle_instrument_action(
        &mut self,
        track_id: TrackIdentifier,
        action: &InstrumentAction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(track_synth) = self.tracks.get(&track_id) {
            track_synth
                .borrow_mut()
                .handle_instrument_action(track_id, action.as_ref())?;
        }
        Ok(())
    }
}

/// Wrapper to make Rc<RefCell<TrackSynth>> implement Output
/// SAFETY: This is only used in a single-threaded context (engine thread)
struct RefCellOutputWrapper {
    inner: Rc<RefCell<TrackSynth>>,
}

// SAFETY: We only use this in the engine thread, never across threads
unsafe impl Send for RefCellOutputWrapper {}
unsafe impl Sync for RefCellOutputWrapper {}

impl Output for RefCellOutputWrapper {
    fn write_f32(
        &mut self,
        len: usize,
        left_out: &mut [f32],
        loff: usize,
        lincr: usize,
        right_out: &mut [f32],
        roff: usize,
        rincr: usize,
    ) {
        self.inner
            .borrow_mut()
            .write_f32(len, left_out, loff, lincr, right_out, roff, rincr);
    }
}
