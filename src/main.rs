// mod midi_ports;
mod engine;

use engine::synth::play_midi;

use std::error::Error;


fn main() {
    println!("Playing a scale!");
    match play_scale() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

fn play_scale() -> Result<(), Box<dyn Error>> {
    let notes: [u8; 8] = [60, 62, 64, 65, 67, 69, 71, 72];
    play_midi(&notes)?;
    Ok(())
}

