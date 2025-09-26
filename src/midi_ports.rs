use midir::{MidiOutput, MidiOutputPort, MidiOutputConnection};
use std::error::Error;
use std::io::{stdout, stdin, Write};
use std::time::Duration;
use std::thread::sleep;


pub fn play_midi(notes: &[u8]) -> Result<(), Box<dyn Error>> {
    let midi_out = MidiOutput::new("MIDI output")?;
    let out_port = get_output_port(&midi_out)?;
    let mut conn = midi_out.connect(&out_port, "midir-test")?;
    for note in notes {
        play_note(&mut conn, *note, 4);
    }
    Ok(())
}


fn get_output_port(midi_out: &MidiOutput) -> Result<MidiOutputPort, Box<dyn Error>> {
    // Get an output port (read from console if multiple are available)
    let out_ports = midi_out.ports();
    let out_port: MidiOutputPort = match out_ports.len() {
        0 => return Err("no output port found".into()),
        1 => {
            println!(
                "Choosing the only available output port: {}",
                midi_out.port_name(&out_ports[0]).unwrap()
            );
            out_ports[0].clone()
        }
        _ => {
            println!("\nAvailable output ports:");
            for (i, p) in out_ports.iter().enumerate() {
                println!("{}: {}", i, midi_out.port_name(p).unwrap());
            }
            print!("Please select output port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            out_ports
                .get(input.trim().parse::<usize>()?)
                .ok_or("invalid output port selected")?.clone()
        }
    };
    Ok(out_port)
}


fn play_note(conn: &mut MidiOutputConnection, note: u8, duration: u64) {
    const NOTE_ON_MSG: u8 = 0x90;
    const NOTE_OFF_MSG: u8 = 0x80;
    const VELOCITY: u8 = 0x64;
    // We're ignoring errors in here
    let _ = conn.send(&[NOTE_ON_MSG, note, VELOCITY]);
    sleep(Duration::from_millis(duration * 150));
    let _ = conn.send(&[NOTE_OFF_MSG, note, VELOCITY]);
}
