mod synth;
mod buss;
mod audio;
mod actions;

use std::cmp::max;
use std::error::Error;
use std::sync::{mpsc, Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use log::{debug, info};

use crate::engine::buss::{BufferedOutput, Output};
use crate::engine::synth::{TrackThread, prepare_output};
use crate::models::instuments::{Instrument, SimpleSynth};
use crate::models::sequences::Tick;
use crate::models::shared::ProjectData;
use crate::models::components::Track;

pub struct EngineController {
    tx: mpsc::Sender<actions::Actions>,
}

pub struct PlayerState {
    pub is_preparing_to_play: bool,
    pub is_playing: bool,
    pub is_active: bool,
    // State for tracking audio output
    pub playhead: u32,

    // Internal system tracking
    samples_played: usize,
    sample_rate: u32,
}

impl PlayerState {
    pub fn new() -> Self {
        Self { is_preparing_to_play: false, is_playing: false, is_active: true, playhead: 0, samples_played: 0, sample_rate: 1 }
    }
}

pub struct StateObserver<F> 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static,
{
    on_change: F,
    player_state: Arc<RwLock<PlayerState>>,
}

impl<F> StateObserver<F> 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static,
{
    fn new(callback: F, player_state: Arc<RwLock<PlayerState>>) -> StateObserver<F> {
        StateObserver {
            on_change: callback,
            player_state,
        }
    }
    pub fn notify(&self) {
        if let Ok(state) = self.player_state.read() {
            (self.on_change)(&state);
        }
    }
}

const ALWAYS_PLAY_FROM_START: bool  = false;
pub fn start<F>(observer_callback: F, shared_data: Arc<RwLock<ProjectData>>) -> (EngineController, Arc<RwLock<PlayerState>>) 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static {
    let (tx, rx) = mpsc::channel::<actions::Actions>();
    let (tick_sender, tick_receiver) = crossbeam_channel::unbounded();
    let player_state = Arc::new(RwLock::new(PlayerState::new()));

    let observer = StateObserver::new(observer_callback, Arc::clone(&player_state));

    thread::spawn({
    let tx = tx.clone();
    let player_state = player_state.clone();
    move || {
       loop {
        let received = rx.recv().unwrap();
        match received {
            actions::Actions::PlayMidi => {
                if let Ok(mut state) = player_state.write() {
                    state.is_preparing_to_play = true;
                    if ALWAYS_PLAY_FROM_START {
                        state.playhead = 0;
                        state.sample_rate = 0;
                    } else {
                        if let Ok(song) = shared_data.read() {
                            state.samples_played = (state.playhead  *  state.sample_rate / song.ticks_per_second()) as usize;
                            // info!("Initialize playhead to {} ({} samples)", state.playhead, state.samples_played);
                        }
                    }
                    
                }
                observer.notify();
                let worker_tx = tx.clone();
                let tick_receiver = tick_receiver.clone();
                // Ensure receiver is empty before we start
                loop {
                    if let Err(_) = tick_receiver.recv_timeout(Duration::from_millis(1)) {
                        break;
                    }
                }
                let worker_shared_data = Arc::clone(&shared_data);
                // debug!("Prepare to play");
                thread::spawn(move || {
                    if let Ok(song) = worker_shared_data.read() {
                        // BLOCKING AUDIO CALL
                        play_structure(&song, &worker_tx, tick_receiver).unwrap();
                    }
                    // Notify the main engine thread that playback is done
                    let _ = worker_tx.send(actions::Actions::Internal(
                        actions::SystemActions::PlaybackFinished
                    ));
                });
            },
            actions::Actions::Quit => {
                if let Ok(mut state) = player_state.write() {
                    state.is_active = false;
                }
                break;
            },
            actions::Actions::Internal(sys_ev) => {
                match sys_ev {
                    actions::SystemActions::SamplesPlayed(num_samples) => {
                        // debug!("Samples played");
                        if let Ok(mut state) = player_state.write() {
                            if state.is_playing {
                                state.samples_played += num_samples;
                                // Convert to playhead location
                                if let Ok(song) = shared_data.read() {
                                    let new_playhead = song.ticks_per_second() * state.samples_played as u32 / state.sample_rate;
                                    if new_playhead != state.playhead {
                                        // info!("Playhead moved to {new_playhead}  ({} samples)", state.samples_played);
                                        for tick in state.playhead..new_playhead {
                                            let _ = tick_sender.send(tick);
                                        }
                                        state.playhead = new_playhead;
                                    }
                                }
                            }
                        }                                
                    }
                    actions::SystemActions::SetSampleRate(sample_rate) => {
                        if let Ok(mut state) = player_state.write() {
                            state.sample_rate = sample_rate;
                        }                                
                    }
                    actions::SystemActions::PlaybackStarted => {
                        if let Ok(mut state) = player_state.write() {
                            state.is_playing = true;
                            state.is_preparing_to_play = false;
                            debug!("Starting to play");
                        }
                        observer.notify();
                    },
                    actions::SystemActions::PlaybackFinished => {
                        if let Ok(mut state) = player_state.write() {
                            state.is_playing = false;
                        }
                        debug!("Playback finished");
                        observer.notify();
                    },
                }
            }
        }
       }
    }});

    
    (EngineController {tx}, player_state.clone())
}

impl EngineController {
    pub fn play_midi(&self) {
        let _ = self.tx.send(actions::Actions::PlayMidi);
    }
    pub fn quit(&self) {
        let _ = self.tx.send(actions::Actions::Quit);
    }

}

fn play_structure(structure: &ProjectData, tx: &mpsc::Sender<actions::Actions>, tick_rx: crossbeam_channel::Receiver<Tick>) -> Result<(), Box<dyn Error>> {
	let mut engine = audio::init_audio(&tx)?;
    let _ = engine.pause(); // Engine starts with stream running. Stop it.
    // Match synth sample rate to the device sample rate so pitch/timing are correct
    let tracks: Result<Vec<TrackThread>, _> = structure.tracks.iter().map(|track| {
        match &track.instrument.kind {

            Instrument::Synth(instrument) => {
                if let Some(seq) = &track.midi {
                    Ok(TrackThread::new(seq, engine.sample_rate as u32, structure.bpm, &instrument.get_soundfont_path(), instrument. bank, instrument.program))
                } else {
                    Err("Not midi")
                }
            }
        }
        
    }).filter(|r| {r.is_ok()}).collect();
    let tracks = tracks?;
    debug!("Playing {} tracks", tracks.len());
    let thread_handles: Vec<JoinHandle<()>> = tracks.into_iter().map(|track | {
        engine.add_input(track.synth.clone());       
        track.run(tick_rx.clone())     
    } ).collect();
    engine.start()?;
    let _ = tx.send(actions::Actions::Internal(actions::SystemActions::PlaybackStarted));
    for handle in thread_handles {
        handle.join().expect("Track thread panicked");
    }
    info!("Sequence complete");
    engine.pause()?;
    Ok(())
 }

 fn play_structure_buffered(structure: &ProjectData, tx: &mpsc::Sender<actions::Actions>) -> Result<(), Box<dyn Error>> {
	let mut engine = audio::init_audio(&tx)?;
    let _ = engine.pause(); // Engine starts with stream running. Stop it.
    // Match synth sample rate to the device sample rate so pitch/timing are correct
    let mut len = std::time::Duration::from_millis(0);
    let outputs: Result<Vec<Arc<RwLock<Box<dyn Output>>>>, _> = structure.tracks.iter().map(|track| {
        len = max(len, track.duration(structure.ticks_per_second()));
        match &track.instrument.kind {
            Instrument::Synth(instrument) => get_buffered_output_for_track(track, engine.sample_rate as u32, structure.bpm, instrument).map(|r| {
                let output: Box<dyn Output> = Box::new(r);
                Arc::new(RwLock::new(output))
            })
        }
        
    }).collect();
    let outputs = outputs?;
    let _ = outputs.into_iter().map(|output | {
        engine.add_input(output);       
    } ).count();
    info!("Playing for {} ms", len.as_millis());
    engine.start()?;
    std::thread::sleep(len);
    info!("Sequence complete");
    engine.pause()?;
    Ok(())
 }


 fn get_buffered_output_for_track(track: &Track, sample_rate: u32, bpm: u8, instrument: &SimpleSynth) -> Result<BufferedOutput, Box<dyn Error>> {
    // Get the midi event stream
    if let Some(event_stream) = &track.midi {
    // For the moment, just pipe into synth. Eventually, we'll want to determine the audio generator from the track config
        prepare_output(event_stream, sample_rate, bpm, &instrument.get_soundfont_path(), instrument. bank, instrument.program)
    } else {
        Ok(BufferedOutput::new())
    }
 }
 