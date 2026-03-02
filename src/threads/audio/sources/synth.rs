use crossbeam_channel::Receiver;
use log::{debug, error, info};
use oxisynth::*;
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use super::super::interfaces::Output;
use crate::models::instuments::get_soundfont_path;
use crate::models::sequences::{EventPriority, EventStream, EventStreamSource, Tick};
use crate::models::shared::TrackIdentifier;
use crate::threads::audio::controllers::MidiInputMessage;

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)] // Set is not part of enum name
pub enum SynthActions {
    SetSoundFont(TrackIdentifier, Option<PathBuf>),
    SetBank(TrackIdentifier, u32),
    SetProgram(TrackIdentifier, u8),
}

// pub fn play_midi(notes: &[u8]) -> Result<(), Box<dyn Error>> {
// 	let engine = start_audio()?;
// 	for note in notes {
// 		play_note(&engine.synth, *note, 4);
// 	}
// 	// Give the tail some time to ring out before dropping the stream
// 	sleep(Duration::from_millis(250));
// 	Ok(())
// }

impl Output for Synth {
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
        self.write_f32(len, left_out, loff, lincr, right_out, roff, rincr);
    }
}

fn on_tick(
    tick: Tick,
    synth: Arc<RwLock<Box<dyn Output>>>,
    event_stream: &EventStream,
) -> Result<(), Box<dyn Error>> {
    // println!("Tick: {tick}");
    for priority in [
        EventPriority::System,
        EventPriority::Audio,
        EventPriority::Other,
    ] {
        for event in event_stream.get_events(tick, priority) {
            // println!("Event at {tick}");
            if let Ok(mut boxed_output_guard) = synth.try_write() {
                // Get a mutable reference to the Box<dyn Output>
                let boxed_output_ref: &mut Box<dyn Output> = &mut boxed_output_guard;

                // Attempt to downcast the trait object reference (&mut dyn Output)
                if let Some(s) = boxed_output_ref.as_any_mut().downcast_mut::<Synth>() {
                    // println!("Playing event at {tick}");
                    s.send_event(event.as_midi())?;
                }
            }
        }
    }
    Ok(())
}

fn on_tick_direct(
    tick: Tick,
    synth: &mut Box<dyn Output>,
    event_stream: &EventStream,
) -> Result<(), Box<dyn Error>> {
    for priority in [
        EventPriority::System,
        EventPriority::Audio,
        EventPriority::Other,
    ] {
        for event in event_stream.get_events(tick, priority) {
            if let Some(s) = synth.as_any_mut().downcast_mut::<Synth>() {
                s.send_event(event.as_midi())?;
            }
        }
    }
    Ok(())
}

fn create_synth<P: AsRef<Path> + ?Sized + ToString>(
    soundfont: &P,
    bank: u32,
    program: u8,
) -> Result<(Synth, SoundFontId), Box<dyn Error>> {
    debug!("Loading font from {}", ToString::to_string(soundfont));
    let mut synth = Synth::default();
    let mut file = File::open(soundfont)?;
    let font = SoundFont::load(&mut file)?; // TODO: handle
    let font_id = synth.add_font(font, true);
    // Now select specifed bank and program (limit to channel 0, since only one midi stream per instrument)
    let _ = synth.select_program(0, font_id, bank, program);
    Ok((synth, font_id))
}

#[allow(dead_code)] // Variants used by TrackThread::run(); senders not yet wired in this design.
pub enum TrackThreadEvents {
    Tick(Tick),
    Update(TrackIdentifier, EventStream),
    Synth(SynthActions),
}

pub struct TrackThread {
    id: TrackIdentifier,
    pub synth: Arc<RwLock<Box<dyn Output>>>, // This will actually be a Synth type which we downcast later
    event_stream: EventStream,
    soundfont_id: SoundFontId,
    bank_id: u32,
    program_id: u8,
}

/// Instrument (synth) that receives MIDI from region playback, preview service, or (future) raw MIDI input.
/// Lives in the engine thread; MIDI is delivered via a channel so preview can run on another thread.
pub struct TrackSynth {
    pub id: TrackIdentifier,
    synth: Box<dyn Output>,
    event_stream: EventStream,
    midi_receiver: flume::Receiver<MidiInputMessage>,
    soundfont_id: SoundFontId,
    bank_id: u32,
    program_id: u8,
}

impl Output for TrackSynth {
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
        self.process_midi_input();
        self.synth
            .write_f32(len, left_out, loff, lincr, right_out, roff, rincr);
    }
}

impl TrackSynth {
    pub fn new<P: AsRef<Path> + ?Sized + ToString>(
        id: TrackIdentifier,
        seq: &dyn EventStreamSource,
        sample_rate: u32,
        soundfont: &P,
        bank: u32,
        program: u8,
        midi_receiver: flume::Receiver<MidiInputMessage>,
    ) -> Self {
        let bank_id = bank;
        let program_id = program;
        let (mut synth, soundfont_id) =
            create_synth(soundfont, bank, program).expect("Couldn't create synth");
        synth.set_sample_rate(sample_rate as f32);
        let synth: Box<dyn Output> = Box::new(synth);
        let event_stream = seq.to_event_stream();
        Self {
            id,
            synth,
            event_stream,
            midi_receiver,
            soundfont_id,
            bank_id,
            program_id,
        }
    }

    /// Drain MIDI input (region ticks and preview/raw events) and apply to the synth.
    fn process_midi_input(&mut self) {
        while let Ok(msg) = self.midi_receiver.try_recv() {
            match msg {
                MidiInputMessage::RegionTick(tick) => {
                    if tick <= self.event_stream.get_length_in_ticks() {
                        let _ = on_tick_direct(tick, &mut self.synth, &self.event_stream);
                    }
                }
                MidiInputMessage::MidiEvent(ev) => {
                    if let Some(s) = self.synth.as_any_mut().downcast_mut::<Synth>() {
                        let _ = s.send_event(ev);
                    }
                }
            }
        }
    }

    pub fn update_event_stream(&mut self, event_stream: EventStream) {
        self.event_stream = event_stream;
    }

    pub fn get_event_stream(&self) -> &EventStream {
        &self.event_stream
    }

    pub fn handle_synth_action(&mut self, action: SynthActions) -> Result<(), Box<dyn Error>> {
        match action {
            SynthActions::SetSoundFont(track_id, soundfont_path) => {
                if track_id == self.id
                    && let Some(path) = soundfont_path
                    && let Some(s) = self.synth.as_any_mut().downcast_mut::<Synth>()
                {
                    let soundfont_file = path
                        .file_name()
                        .map(|x| x.to_str())
                        .expect("File picker should return valid string")
                        .unwrap()
                        .to_string();
                    let soundfont_path = get_soundfont_path(&soundfont_file);
                    let mut file = File::open(soundfont_path)?;
                    let font = SoundFont::load(&mut file)?;
                    self.soundfont_id = s.add_font(font, true);
                    let _ = s.select_program(0, self.soundfont_id, self.bank_id, self.program_id);
                }
            }
            SynthActions::SetBank(track_id, bank) => {
                if track_id == self.id
                    && let Some(s) = self.synth.as_any_mut().downcast_mut::<Synth>()
                {
                    self.bank_id = bank;
                    let _ = s.select_bank(0, bank);
                }
            }
            SynthActions::SetProgram(track_id, program) => {
                if track_id == self.id
                    && let Some(s) = self.synth.as_any_mut().downcast_mut::<Synth>()
                {
                    self.program_id = program;
                    let _ = s.select_program(0, self.soundfont_id, self.bank_id, program);
                }
            }
        }
        Ok(())
    }
}

impl TrackThread {
    #[allow(dead_code)]
    pub fn new<P: AsRef<Path> + ?Sized + ToString>(
        id: TrackIdentifier,
        seq: &dyn EventStreamSource,
        sample_rate: u32,
        soundfont: &P,
        bank: u32,
        program: u8,
    ) -> Self {
        let bank_id = 0;
        let program_id = 0;
        let (mut synth, soundfont_id) =
            create_synth(soundfont, bank, program).expect("Couldn't create synth");
        synth.set_sample_rate(sample_rate as f32);
        let synth: std::sync::Arc<RwLock<Box<dyn Output + 'static>>> =
            Arc::new(RwLock::new(Box::new(synth)));
        let event_stream = seq.to_event_stream();
        Self {
            id,
            synth,
            soundfont_id,
            event_stream,
            bank_id,
            program_id,
        }
    }
    #[allow(dead_code)]
    pub fn run(mut self, tick_source: Receiver<TrackThreadEvents>) -> JoinHandle<()> {
        info!("Spawning track thread");
        thread::spawn(move || {
            info!("Starting track thread");
            loop {
                let event_stream = &self.event_stream;
                match tick_source.recv() {
                    Ok(event) => {
                        match event {
                            TrackThreadEvents::Tick(tick) => {
                                if tick > event_stream.get_length_in_ticks() {
                                    //break;
                                    // Send PlaybackFinished event
                                    continue;
                                }
                                if let Err(e) = on_tick(tick, self.synth.clone(), event_stream) {
                                    error!("Problem processing tick in track thread: {}", e);
                                    break;
                                }
                            }
                            TrackThreadEvents::Update(id, event_stream) => {
                                if id == self.id {
                                    info!("Updating event stream for track {}", id.track_id);
                                    self.event_stream = event_stream;
                                }
                            }
                            TrackThreadEvents::Synth(action) => match action {
                                SynthActions::SetSoundFont(track_id, soundfont_path) => {
                                    if track_id == self.id
                                        && let Some(path) = soundfont_path
                                        && let Ok(mut boxed_output_guard) = self.synth.write()
                                    {
                                        // Get a mutable reference to the Box<dyn Output>
                                        let boxed_output_ref: &mut Box<dyn Output> =
                                            &mut boxed_output_guard;
                                        if let Some(s) =
                                            boxed_output_ref.as_any_mut().downcast_mut::<Synth>()
                                        {
                                            let soundfont_file = path
                                                .file_name()
                                                .map(|x| x.to_str())
                                                .expect("File picker should return valid string")
                                                .unwrap()
                                                .to_string();
                                            let soundfont_path =
                                                get_soundfont_path(&soundfont_file);
                                            let mut file = File::open(soundfont_path).unwrap();
                                            let font = SoundFont::load(&mut file).unwrap();
                                            self.soundfont_id = s.add_font(font, true);
                                            let _ = s.select_program(
                                                0,
                                                self.soundfont_id,
                                                self.bank_id,
                                                self.program_id,
                                            );
                                        }
                                    }
                                }
                                SynthActions::SetBank(track_id, bank) => {
                                    if track_id == self.id
                                        && let Ok(mut boxed_output_guard) = self.synth.write()
                                    {
                                        // Get a mutable reference to the Box<dyn Output>
                                        let boxed_output_ref: &mut Box<dyn Output> =
                                            &mut boxed_output_guard;

                                        // Attempt to downcast the trait object reference (&mut dyn Output)
                                        if let Some(s) =
                                            boxed_output_ref.as_any_mut().downcast_mut::<Synth>()
                                        {
                                            self.bank_id = bank;
                                            let _ = s.select_bank(0, bank);
                                            println!("Set bank");
                                        }
                                    }
                                }
                                SynthActions::SetProgram(track_id, program) => {
                                    println!(
                                        "Set program received for track_id {} on track {}",
                                        track_id.track_id, self.id.track_id
                                    );
                                    if track_id == self.id
                                        && let Ok(mut boxed_output_guard) = self.synth.write()
                                    {
                                        // Get a mutable reference to the Box<dyn Output>
                                        let boxed_output_ref: &mut Box<dyn Output> =
                                            &mut boxed_output_guard;

                                        // Attempt to downcast the trait object reference (&mut dyn Output)
                                        if let Some(s) =
                                            boxed_output_ref.as_any_mut().downcast_mut::<Synth>()
                                        {
                                            self.program_id = program;
                                            let _ = s.select_program(
                                                0,
                                                self.soundfont_id,
                                                self.bank_id,
                                                program,
                                            );
                                            println!("Set program");
                                        }
                                    }
                                }
                            },
                        }
                    }
                    Err(e) => {
                        // Failed to receive tick
                        error!("Tick source pipeline broken in track thread: {}", e);
                        break;
                    }
                }
            }
            info!("Ending track thread");
        })
    }
}
