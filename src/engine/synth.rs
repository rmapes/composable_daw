use log::debug;
use oxisynth::*;
use std::error::Error;
use std::fs::File;


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

pub fn prepare_output(seq: &dyn EventStreamSource, sample_rate: u32, bpm: u8 ) -> Result<BufferedOutput, Box<dyn Error>> {
	let mut synth = create_synth();
	synth.set_sample_rate(sample_rate as f32);
	let event_stream = seq.to_event_stream();
	let mut output = BufferedOutput::new();
	debug!("Preparing output for event source");
	if event_stream.is_none() {
		// Empty event stream, so exit without playing anything
		debug!("Nothing to play");
		return Ok(output);
	}
	let event_stream = event_stream.unwrap();
	for tick in 0..event_stream.get_length_in_ticks() {
		// println!("Tick {tick}");
		for priority in [EventPriority::System, EventPriority::Audio, EventPriority::Other] {
			// println!("Tick: {tick}");
			for event in event_stream.get_events(tick, priority) {
				synth.send_event(event.to_midi())?;
			}
		}
		// Wait for next tick
		output.read_f32((event_stream.get_tick_duration(bpm).as_nanos() * sample_rate as u128 / 1e9 as u128) as usize, &mut synth);
	}
	Ok(output)
}


fn create_synth() -> Synth {
	let mut synth = Synth::default();
	let mut file = File::open("./soundfonts/airfont_340.sf2").unwrap();
	let font = SoundFont::load(&mut file).unwrap();
	synth.add_font(font, true);
	// If needed, select a default program: bank 0, program 0 on channel 0
	// let _ = synth.program_select(0, 0, 0, 0);
	synth
}

