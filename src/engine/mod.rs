mod synth;

use synth::play_midi;
use std::error::Error;
use std::sync::mpsc;
use std::thread;

pub struct EngineController {
    tx: mpsc::Sender<Actions>,
}

enum Actions {
    PlayMidi,
    Quit,
}

pub fn start() -> EngineController {
    let (tx, rx) = mpsc::channel::<Actions>();

    thread::spawn(move || {
       loop {
        let received = rx.recv().unwrap();
        match received {
            Actions::PlayMidi => play_scale().unwrap(),
            Actions::Quit => break,
        }
       }
    });

    
    return EngineController {tx};
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
    let notes: [u8; 8] = [60, 62, 64, 65, 67, 69, 71, 72];
    play_midi(&notes)
}