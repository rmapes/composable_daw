// mod midi_ports;
mod engine;

slint::include_modules!();

use std::rc::Rc;


fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    let engine = Rc::new(engine::start());
    let engine_clone = Rc::clone(&engine);
    
    // Set up callbacks
    main_window.on_play_midi(move || {
        engine_clone.play_midi();
    });

    // Run program
    main_window.run()?;
    engine.quit();
    Ok(())
}
