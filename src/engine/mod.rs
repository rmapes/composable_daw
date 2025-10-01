mod synth;

use synth::play_sequence;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crate::models::shared::SongData;

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

pub fn start<F>(observer_callback: F, shared_data: Arc<Mutex<SongData>>) -> EngineController 
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
                if let Ok(song) = shared_data.lock() {
                    play_sequence(&*song).unwrap();
                }
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

