use oxisynth::*;
use std::error::Error;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioEngine {
	pub synth: Arc<Mutex<Synth>>,
	_stream: cpal::Stream,
}

pub fn play_midi(notes: &[u8]) -> Result<(), Box<dyn Error>> {
	let engine = start_audio()?;
	for note in notes {
		play_note(&engine.synth, *note, 4);
	}
	// Give the tail some time to ring out before dropping the stream
	sleep(Duration::from_millis(250));
	Ok(())
}


fn create_synth() -> Synth {
	let mut synth = Synth::default();
	let mut file = File::open("./soundfonts/Antares_SoundFont.sf2").unwrap();
	let font = SoundFont::load(&mut file).unwrap();
	synth.add_font(font, true);
	// If needed, select a default program: bank 0, program 0 on channel 0
	// let _ = synth.program_select(0, 0, 0, 0);
	synth
}

fn play_note(synth: &Arc<Mutex<Synth>>, note: u8, duration: u64) {
	const VELOCITY: u8 = 0x64;
	let note_on_msg: MidiEvent = MidiEvent::NoteOn {
		channel: 0,
		key: note,
		vel: VELOCITY,
	};
	let note_off_msg: MidiEvent = MidiEvent::NoteOff { channel: 0, key: note };

	if let Ok(mut guard) = synth.lock() {
		let _ = guard.send_event(note_on_msg);
	}
	sleep(Duration::from_millis(duration * 150));
	if let Ok(mut guard) = synth.lock() {
		let _ = guard.send_event(note_off_msg);
	}
}

fn start_audio() -> Result<AudioEngine, Box<dyn Error>> {
	let host = cpal::default_host();
	let device = host
		.default_output_device()
		.ok_or("No default output device")?;
	let supported = device
		.default_output_config()
		.map_err(|e| format!("Failed to get default output config: {e}"))?;
	let channels = supported.channels() as usize;

	let synth = Arc::new(Mutex::new(create_synth()));
	// Match synth sample rate to the device sample rate so pitch/timing are correct
	if let Ok(mut guard) = synth.lock() {
		guard.set_sample_rate(supported.sample_rate().0 as f32);
	}
	let synth_for_cb = Arc::clone(&synth);

	let err_fn = |err| eprintln!("audio stream error: {err}");

	let stream = match supported.sample_format() {
		cpal::SampleFormat::F32 => {
			let config: cpal::StreamConfig = supported.clone().into();
			device.build_output_stream(
				&config,
				move |data: &mut [f32], _| fill_output_buffer(data, channels, &synth_for_cb),
				err_fn,
				None,
			)?
		}
		other => return Err(format!("Unsupported sample format: {other:?}").into()),
	};

	stream.play()?;

	Ok(AudioEngine { synth, _stream: stream })
}

fn fill_output_buffer(data: &mut [f32], channels: usize, synth: &Arc<Mutex<Synth>>) {
	let frames = data.len() / channels;
	// Render exactly 'frames' samples per channel using write_f32 as per docs
	let mut left = vec![0.0_f32; frames];
	let mut right = vec![0.0_f32; frames];
	if let Ok(mut guard) = synth.lock() {
		// https://docs.rs/oxisynth/0.1.0/oxisynth/struct.Synth.html#method.write
		let _ = guard.write_f32(frames, &mut left, 0, 1, &mut right, 0, 1);
	}

	match channels {
		1 => {
			for i in 0..frames { data[i] = 0.5 * (left[i] + right[i]); }
		}
		2 => {
			for i in 0..frames {
				let di = i * 2;
				data[di] = left[i];
				data[di + 1] = right[i];
			}
		}
		c => {
			for i in 0..frames {
				let base = i * c;
				data[base] = left[i];
				if c > 1 { data[base + 1] = right[i]; }
				for ch in 2..c { data[base + ch] = 0.0; }
			}
		}
	}
}