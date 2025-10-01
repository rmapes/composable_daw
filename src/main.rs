// mod midi_ports;
mod engine;
mod models;

slint::include_modules!();


use engine::PlayerState;
use models::shared::SongData;
use models::sequences::PatternSeq;
use slint::Model;
use std::sync::{Arc, Mutex};


use std::rc::Rc;


fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    let shared_song_data = Arc::new(Mutex::new(SongData::new()));
    let engine = Rc::new(engine::start(
        {
            let win = main_window.as_weak();
            move |player_state: &PlayerState| {
                let is_playing = player_state.is_playing;
                let _ = win.upgrade_in_event_loop(
                    move |handle| {
                        handle.set_is_playing(is_playing);
                    }
                );
            }
        },
        Arc::clone(&shared_song_data),)
        
    );
    let engine_clone = Rc::clone(&engine);
    
    // Handle state updates
    let shared_song_data = shared_song_data.clone();
    main_window.global::<Handlers>().on_save_pattern(move |pattern: Pattern| {
        println!("Saving pattern");
        let contained_pattern: Vec<Vec<bool>> = pattern.pattern.iter().map(|row| {
            row.iter().collect()
        }).collect();
        let local_note_values: Vec<u8> = pattern.note_values.iter().map(|n| n as u8).collect();
        let local_pattern = PatternSeq {
            note_values: local_note_values,
            num_notes: pattern.notes as u8,
            num_beats: pattern.beats as u8,
            bpm: 120,
            pattern: contained_pattern,
            sample_rate: 960,
        };
        if let Ok(mut song) = shared_song_data.lock() {
            song.patterns.clear();
            song.patterns.push(local_pattern);
        }
    });

    // Set up callbacks
    main_window.on_play_midi(move || {
        engine_clone.play_midi();
    });

    // Run program
    main_window.run()?;
    engine.quit();
    Ok(())
}