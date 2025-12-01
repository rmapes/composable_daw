use log::debug;
use oxisynth::*;
use std::error::Error;
use std::fs::File;
use std::path::Path;


use crate::engine::buss::BufferedOutput;
use crate::models::sequences::{EventStreamSource, EventPriority};
use super::buss::Output;


// pub fn play_midi(notes: &[u8]) -> Result<(), Box<dyn Error>> {
// 	let engine = start_audio()?;
// 	for note in notes {
// 		play_note(&engine.synth, *note, 4);
// 	}
// 	// Give the tail some time to ring out before dropping the stream
// 	sleep(Duration::from_millis(250));
// 	Ok(())
// }


impl Output for Synth {
	fn write_f32(&mut self, 
		len: usize, 
		left_out: &mut [f32], 
		loff: usize, 
		lincr: usize, 
		right_out: &mut [f32], 
		roff: usize, 
		rincr: usize,
	) {
		self.write_f32(len, left_out, loff, lincr, right_out, roff, rincr);
	}
}

pub fn prepare_output<P: AsRef<Path> + ?Sized + ToString>(seq: &dyn EventStreamSource, sample_rate: u32, bpm: u8, soundfont: &P, bank: u32, program: u8 ) -> Result<BufferedOutput, Box<dyn Error>> {
	let mut synth = create_synth(soundfont, bank, program)?;
	synth.set_sample_rate(sample_rate as f32);
	let event_stream = seq.to_event_stream();
	let mut output = BufferedOutput::new();
	debug!("Preparing output for event source");
	for tick in 0..event_stream.get_length_in_ticks() {
		// println!("Tick {tick}");
		for priority in [EventPriority::System, EventPriority::Audio, EventPriority::Other] {
			// println!("Tick: {tick}");
			for event in event_stream.get_events(tick, priority) {
				// println!("Event at {tick}");
				synth.send_event(event.to_midi())?;
			}
		}
		// Wait for next tick
		output.read_f32((event_stream.get_tick_duration(bpm).as_nanos() * sample_rate as u128 / 1e9 as u128) as usize, &mut synth);
	}
	Ok(output)
}


fn create_synth<P: AsRef<Path> + ?Sized + ToString>(soundfont: &P, bank: u32, program: u8) ->  Result<Synth, Box<dyn Error>> {
	debug!("Loading font from {}", ToString::to_string(soundfont));
	let mut synth = Synth::default();
	let mut file = File::open(soundfont)?; 
	let font = SoundFont::load(&mut file)?; // TODO: handle
	let font_id = synth.add_font(font, true);
	// Now select specifed bank and program (limit to channel 0, since only one midi stream per instrument)
	let _ = synth.select_program(0, font_id, bank, program);
	Ok(synth)
}

