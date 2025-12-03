mod synth;
mod buss;
mod audio;
mod actions;

use std::cmp::max;
use std::error::Error;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;

use log::info;

use crate::engine::buss::{BufferedOutput, Output};
use crate::engine::synth::prepare_output;
use crate::models::instuments::{Instrument, SimpleSynth};
use crate::models::shared::ProjectData;
use crate::models::components::Track;

pub struct EngineController {
    tx: mpsc::Sender<actions::Actions>,
}

pub struct PlayerState {
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
        Self { is_playing: false, is_active: true, playhead: 0, samples_played: 0, sample_rate: 1 }
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

pub fn start<F>(observer_callback: F, shared_data: Arc<RwLock<ProjectData>>) -> (EngineController, Arc<RwLock<PlayerState>>) 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static {
    let (tx, rx) = mpsc::channel::<actions::Actions>();
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
                    state.is_playing = true;
                    state.playhead = 0;
                    state.samples_played = 0;
                    
                }
                observer.notify();
                let worker_tx = tx.clone();
                let worker_shared_data = Arc::clone(&shared_data);
                thread::spawn(move || {
                    if let Ok(song) = worker_shared_data.read() {
                        // BLOCKING AUDIO CALL
                        play_structure(&song, &worker_tx).unwrap();
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
                        if let Ok(mut state) = player_state.write() {
                            state.samples_played += num_samples;
                            // Convert to playhead location
                            if let Ok(song) = shared_data.read() {
                                state.playhead = song.ticks_per_second() * state.samples_played as u32 / state.sample_rate;
                                // println!("Player: {}", state.playhead);
                                // println!("Ticks per second: {}, Samples Played: {}, sample rate: {}", song.ticks_per_second(), state.samples_played, state.sample_rate);
                                // println!("Player: {}", state.playhead);
                                // println!("Beats: {}", state.playhead / song.ppq);
                            }
            
                        }                                
                    }
                    actions::SystemActions::SetSampleRate(sample_rate) => {
                        if let Ok(mut state) = player_state.write() {
                            state.sample_rate = sample_rate;
                        }                                
                    }
                    actions::SystemActions::PlaybackFinished => {
                        if let Ok(mut state) = player_state.write() {
                            state.is_playing = false;
                        }
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

fn play_structure(structure: &ProjectData, tx: &mpsc::Sender<actions::Actions>) -> Result<(), Box<dyn Error>> {
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
 