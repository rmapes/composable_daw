use crossbeam_channel::Receiver;
use log::{debug, error, info};
use oxisynth::*;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};


use crate::models::sequences::{EventPriority, EventStreamSource, Tick, EventStream};
use crate::models::shared::TrackIdentifier;
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


fn on_tick(tick: Tick, synth: Arc<RwLock<Box<dyn Output>>>, event_stream: &EventStream)  -> Result<(), Box<dyn Error>> {
		// println!("Tick: {tick}");
		for priority in [EventPriority::System, EventPriority::Audio, EventPriority::Other] {
		for event in event_stream.get_events(tick, priority) {
			// println!("Event at {tick}");
			if let Ok(mut boxed_output_guard) = synth.try_write() {
				// Get a mutable reference to the Box<dyn Output>
				let boxed_output_ref: &mut Box<dyn Output> = &mut boxed_output_guard;

				// Attempt to downcast the trait object reference (&mut dyn Output)
				if let Some(s) = boxed_output_ref.as_any_mut().downcast_mut::<Synth>() {
					// println!("Playing event at {tick}");
					s.send_event(event.as_midi())?;
				}
			}
		}
	};
	Ok(())
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

pub enum TrackThreadEvents {
	Tick(Tick),
	Update(TrackIdentifier, EventStream)
}

pub struct TrackThread {
	id: TrackIdentifier,
    pub synth: Arc<RwLock<Box<dyn Output>>>, // This will actually be a Synth type which we downcast later
    event_stream: EventStream,
}

impl TrackThread {
	pub fn new<P: AsRef<Path> + ?Sized + ToString>(id: TrackIdentifier, seq: &dyn EventStreamSource, sample_rate: u32, soundfont: &P, bank: u32, program: u8) -> Self {
		let synth: Arc<RwLock<Box<dyn Output>>> = {
			let mut synth = create_synth(soundfont, bank, program).expect("Couldn't create synth");
			synth.set_sample_rate(sample_rate as f32);
			Arc::new(RwLock::new(Box::new(synth)))
		};
		let event_stream = seq.to_event_stream();
		Self { id, synth, event_stream }
	}
	pub fn run(mut self, tick_source: Receiver<TrackThreadEvents>) -> JoinHandle<()> {
		info!("Spawning track thread");
		thread::spawn(move || {
			info!("Starting track thread");
			loop {
				let event_stream = &self.event_stream;
				match tick_source.recv() {
					Ok(event) => {
						match event {
							TrackThreadEvents::Tick(tick) => {
								if tick > event_stream.get_length_in_ticks() {
									//break;
									// Send PlaybackFinished event
									continue;
								}
								if let Err(e) = on_tick(tick, self.synth.clone(), event_stream) {
									error!("Problem processing tick in track thread: {}", e);
									break;
								}
							},
							TrackThreadEvents::Update(id, event_stream) => {
								if id == self.id {
									info!("Updating event stream for track {}", id.track_id);
									self.event_stream = event_stream;
								}
							}
						}
					}
					Err(e) => {
						// Failed to receive tick
						error!("Tick source pipeline broken in track thread: {}", e);
						break;
					}
				}
			}
			info!("Ending track thread");
		})
	}
}


