mod synth;

use synth::play_sequence;
use std::error::Error;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crate::models::sequences::{PatternSeq};

pub struct EngineController {
    tx: mpsc::Sender<Actions>,
}

pub struct PlayerState {
    pub is_playing: bool
}

pub struct StateObserver<F> 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static,
{
    on_change: F,
    player_state: Arc<Mutex<PlayerState>>,
}

impl<F> StateObserver<F> 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static,
{
    fn new(callback: F, player_state: Arc<Mutex<PlayerState>>) -> StateObserver<F> {
        StateObserver {
            on_change: callback,
            player_state,
        }
    }
    pub fn notify(&self) {
        if let Ok(state) = self.player_state.lock() {
            (self.on_change)(&state);
        }
    }
}
enum Actions {
    PlayMidi,
    Quit,
}

pub fn start<F>(observer_callback: F) -> EngineController 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static {
    let (tx, rx) = mpsc::channel::<Actions>();
    let player_state = Arc::new(Mutex::new(PlayerState { is_playing: false }));

    let observer = StateObserver::new(observer_callback, Arc::clone(&player_state));

    thread::spawn(move || {
       loop {
        let received = rx.recv().unwrap();
        match received {
            Actions::PlayMidi => {
                if let Ok(mut state) = player_state.lock() {
                    state.is_playing = true;
                }
                observer.notify();
                play_scale().unwrap();
                if let Ok(mut state) = player_state.lock() {
                    state.is_playing = false;
                }
                observer.notify();
            },
            Actions::Quit => break,
        }
       }
    });

    
    return EngineController {tx}
}

impl EngineController {
    pub fn play_midi(&self) {
        let _ = self.tx.send(Actions::PlayMidi);
    }
    pub fn quit(&self) {
        let _ = self.tx.send(Actions::Quit);
    }

}

fn play_scale() -> Result<(), Box<dyn Error>> {
    // let _notes: [u8; 8] = [60, 62, 64, 65, 67, 69, 71, 72];
    // play_midi(&notes)
    let pattern = PatternSeq {
        note_values: vec![60, 62, 64, 65, 67, 69, 71, 72],
        num_notes: 8, // Derive this from length of note_values
        num_beats: 8,
        bpm: 120,
        pattern: vec![
            vec![true,false,false,false,false,false,false,false],
            vec![false,true,false,false,false,false,false,false],
            vec![false,false,true,false,false,false,false,false],
            vec![false,false,false,true,false,false,false,false],
            vec![false,false,false,false,true,false,false,false],
            vec![false,false,false,false,false,true,false,false],
            vec![false,false,false,false,false,false,true,false],
            vec![false,false,false,false,false,false,false,true],
        ],
        sample_rate: 960, /* ticks per second */    
    };
    play_sequence(&pattern)

}