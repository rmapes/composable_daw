mod synth;
mod buss;
mod audio;

use std::cmp::max;
use std::error::Error;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;

use log::info;

use crate::engine::buss::BufferedOutput;
use crate::engine::synth::prepare_output;
use crate::models::shared::ProjectData;
use crate::models::components::{Track};

pub struct EngineController {
    tx: mpsc::Sender<Actions>,
}

pub struct PlayerState {
    pub is_playing: bool,
    pub is_active: bool,
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
enum Actions {
    PlayMidi,
    Quit,
}

pub fn start<F>(observer_callback: F, shared_data: Arc<RwLock<ProjectData>>) -> EngineController 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static {
    let (tx, rx) = mpsc::channel::<Actions>();
    let player_state = Arc::new(RwLock::new(PlayerState { is_playing: false, is_active: true }));

    let observer = StateObserver::new(observer_callback, Arc::clone(&player_state));

    thread::spawn(move || {
       loop {
        let received = rx.recv().unwrap();
        match received {
            Actions::PlayMidi => {
                if let Ok(mut state) = player_state.write() {
                    state.is_playing = true;
                }
                observer.notify();
                if let Ok(song) = shared_data.read() {
                    play_structure(&song).unwrap();
                }
                if let Ok(mut state) = player_state.write() {
                    state.is_playing = false;
                }
                observer.notify();
            },
            Actions::Quit => {
                if let Ok(mut state) = player_state.write() {
                    state.is_active = false;
                }
                break;
            },
        }
       }
    });

    
    EngineController {tx}
}

impl EngineController {
    pub fn play_midi(&self) {
        let _ = self.tx.send(Actions::PlayMidi);
    }
    pub fn quit(&self) {
        let _ = self.tx.send(Actions::Quit);
    }

}

fn play_structure(structure: &ProjectData) -> Result<(), Box<dyn Error>> {
	let mut engine = audio::init_audio()?;
    // Match synth sample rate to the device sample rate so pitch/timing are correct
    let mut len = std::time::Duration::from_millis(0);
    let outputs: Vec<BufferedOutput> = structure.tracks.iter().map(|track| {
        len = max(len, track.duration(structure.ticks_per_second()));
        get_buffered_output_for_track(track, engine.sample_rate as u32, structure.bpm)
    }).collect();
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

 fn get_buffered_output_for_track(track: &Track, sample_rate: u32, bpm: u8) -> BufferedOutput {
    // Get the midi event stream
    if let Some(event_stream) = &track.midi {
    // For the moment, just pipe into synth. Eventually, we'll want to determine the audio generator from the track config
        prepare_output(event_stream, sample_rate, bpm).unwrap()
    } else {
        BufferedOutput::new()
    }
 }
 