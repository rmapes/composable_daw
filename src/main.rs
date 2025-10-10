// #![allow(
//     clippy::too_many_arguments,
//     http://crt.r2m03.amazontrust.com/r2m03.cer
// )]
// mod midi_ports;
mod engine;
mod models;

slint::include_modules!();


use engine::PlayerState;
use models::shared::SongData;
use models::sequences::PatternSeq;
use slint::{Model, ModelRc, VecModel};
use std::sync::{Arc, Mutex};


use std::rc::Rc;


fn vec_to_model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    let the_model : Rc<VecModel<T>> =
        Rc::new(VecModel::from(v));
    // Convert it to a ModelRc.
    ModelRc::from(the_model.clone())
}

fn pattern_seq_to_pattern(pattern: &PatternSeq) -> Pattern {
    Pattern {
        note_values: vec_to_model(pattern.note_values.iter().map(|&n| {n as i32}).collect()),
        pattern: vec_to_model(pattern.pattern.iter().map(|row: &Vec<bool>| {vec_to_model(row.to_owned())}).collect()),
        notes: pattern.num_notes as i32,
        beats: pattern.num_beats as i32,
    }
}

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
    let pattern_notes = vec![72,71,69,67,65,64,62,60];
    let num_notes = pattern_notes.len() as u8;
    let num_beats: u8 = 16;
    let pattern: Vec<Vec<bool>> = (0..num_beats).map(|_| { (0..num_notes).map(|_| {false}).collect()}).collect();
    let empty_pattern = PatternSeq {
        num_notes,
        num_beats,
        note_values: pattern_notes, // From top to bottom
        pattern,
        bpm: 120,
        sample_rate: 960,
    };
    if let Ok(mut song) = shared_song_data.lock() { 
        song.patterns.push(empty_pattern.clone());
    }
    main_window.global::<Handlers>().set_cur_pattern(pattern_seq_to_pattern(&empty_pattern));
    
    // Handle state updates
    main_window.global::<Handlers>().on_pattern_changed({
        let shared_song_data = shared_song_data.clone();
        let main_window = main_window.clone_strong();
        move || {
        if let Ok(mut song) = shared_song_data.lock() {   
            let cur_pattern_num = main_window.global::<Handlers>().get_cur_pattern_num();     
            while song.patterns.len() <= cur_pattern_num as usize {
                song.patterns.push(empty_pattern.clone());
            }
            if let Some(stored_pattern) = song.patterns.get(cur_pattern_num as usize) {                
                main_window.global::<Handlers>().set_cur_pattern(pattern_seq_to_pattern(stored_pattern));
            } else {
                println!("Can't get pattern at {}", cur_pattern_num);
            }
        }
    }});
    main_window.global::<Handlers>().on_save_pattern({
        let shared_song_data = shared_song_data.clone();
        let main_window = main_window.clone_strong();
        move |pattern: Pattern| {
        println!("Saving pattern");
        let cur_pattern_num = main_window.global::<Handlers>().get_cur_pattern_num();        
        let contained_pattern: Vec<Vec<bool>> = pattern.pattern.iter().map(|row| {
            row.iter().collect()
        }).collect();
        let local_note_values: Vec<u8> = pattern.note_values.iter().map(|n| n as u8).collect();
        if let Ok(mut song) = shared_song_data.lock() {
            let stored_pattern = PatternSeq { 
                note_values: local_note_values,
                pattern: contained_pattern,
                num_notes: pattern.notes as u8,
                num_beats: pattern.beats as u8,
                bpm: 120,
                sample_rate: 960,
            };
            if let Some(stored_pattern_ref) = song.patterns.get_mut(cur_pattern_num as usize) {
                stored_pattern_ref.pattern = stored_pattern.pattern;
            }
        }
    }});

    // Set up callbacks
    main_window.on_play_midi(move || {
        engine_clone.play_midi();
    });

    // Run program
    main_window.run()?;
    engine.quit();
    Ok(())
}