// mod midi_ports;
mod engine;
mod models;

slint::include_modules!();

use engine::PlayerState;

use std::rc::Rc;


fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    let engine = Rc::new(engine::start({
        let win = main_window.as_weak();
        move |player_state: &PlayerState| {
            let is_playing = player_state.is_playing;
            let _ = win.upgrade_in_event_loop(
                move |handle| {
                    handle.set_is_playing(is_playing)
                }
            );
        }
    }));
    let engine_clone = Rc::clone(&engine);
    
    // Handle state updates
    

    // Set up callbacks
    main_window.on_play_midi(move || {
        engine_clone.play_midi();
    });

    // Run program
    main_window.run()?;
    engine.quit();
    Ok(())
}
