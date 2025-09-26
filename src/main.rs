// mod midi_ports;
mod engine;

slint::include_modules!();

use engine::synth::play_midi;
use std::error::Error;

fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    // Set up callbacks
    // let main_window_weak = main_window.as_weak();
    main_window.on_play_midi(move || {
        if let Err(err) = play_scale() {
            eprintln!("play_scale error: {}", err);
        }
    });

    // Run program
    main_window.run()
}

fn play_scale() -> Result<(), Box<dyn Error>> {
    let notes: [u8; 8] = [60, 62, 64, 65, 67, 69, 71, 72];
    play_midi(&notes)?;
    Ok(())
}