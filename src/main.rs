use std::error::Error;

// #![allow(
//     clippy::too_many_arguments,
//     http://crt.r2m03.amazontrust.com/r2m03.cer
// )]
// mod midi_ports;
mod engine;
mod models;
mod ui;




fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting app");
    ui::run().map_err(|err| Box::new(err) as Box<dyn Error>)
 
    // Handle state updates
    // main_window.global::<Handlers>().on_pattern_changed({
    //     let shared_song_data = shared_song_data.clone();
    //     let main_window = main_window.clone_strong();
    //     move || {
    //         if let Ok(mut song) = shared_song_data.lock() {   
    //             let cur_pattern_num = main_window.global::<Handlers>().get_cur_pattern_num();
    //             let song_state = main_window.global::<State>().get_song();
    //             let cur_track_num = main_window.global::<State>().get_cur_track() as usize;
    //             println!("Updating track {} out of {}", cur_track_num, song_state.tracks.row_count());
    //             if let Some(track) = song_state.tracks.row_data(cur_track_num) {
    //                 let track_rf = track.midi_content.patterns.clone();
    //                 let patterns: &VecModel<(Pattern, i32)> = track_rf.as_any().downcast_ref::<VecModel<(Pattern, i32)>>().unwrap();
    //                 println!("Increasing patterns from {} to {}", patterns.row_count(), cur_pattern_num);
    //                 while patterns.row_count() <= cur_pattern_num as usize {
    //                     patterns.push((Pattern::new(), 0));
    //                 }
    //                 let (selected_pattern, _tick) = patterns.row_data(cur_pattern_num as usize).unwrap();
    //                 println!("Updating cur pattern");
    //                 main_window.global::<Handlers>().set_cur_pattern(selected_pattern);
    //                 song_state.sync_from(&song);
    //             }
    //         }
    //     }
    // });
    // main_window.global::<Handlers>().on_save_pattern({
    //     let shared_song_data = shared_song_data.clone();
    //     let main_window = main_window.clone_strong();
    //     move |pattern: Pattern| {
    //     println!("Saving pattern");
    //     let cur_pattern_num = main_window.global::<Handlers>().get_cur_pattern_num();        
    //     let contained_pattern: Vec<Vec<bool>> = pattern.pattern.iter().map(|row| {
    //         row.iter().collect()
    //     }).collect();
    //     let local_note_values: Vec<u8> = pattern.note_values.iter().map(|n| n as u8).collect();
    //     if let Ok(mut song) = shared_song_data.lock() {
    //         let stored_pattern = PatternSeq { 
    //             note_values: local_note_values,
    //             pattern: contained_pattern,
    //             num_notes: pattern.notes as u8,
    //             num_beats: pattern.beats as u8,
    //             bpm: 120,
    //             sample_rate: 960,
    //         };
    //         if let Some(stored_pattern_ref) = song.patterns.get_mut(cur_pattern_num as usize) {
    //             stored_pattern_ref.pattern = stored_pattern.pattern;
    //         }
    //     }
    // }});
}
