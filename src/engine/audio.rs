

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;

use super::buss::{Buss, Output};
use std::sync::Mutex;
use std::sync::Arc;
use std::error::Error;

pub struct AudioEngine {
	_stream: cpal::Stream,
    _input: Arc<Mutex<Buss>>,
    pub sample_rate: u32,
}

impl AudioEngine {
    pub fn start(&mut self) -> Result<(), cpal::PlayStreamError>{
        self._stream.play()
    }
    pub fn add_input<O: Output + 'static>(&mut self, o: O) {
        if let Ok(mut guard) = self._input.lock() {
            guard.add_input(Box::new(o));
        }
    }
}


pub(crate) fn init_audio() -> Result<AudioEngine, Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No default output device")?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("Failed to get default output config: {e}"))?;
    let channels = supported.channels() as usize;

    let buss = Arc::new(Mutex::new(Buss::new()));

    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let buss_for_cb = buss.clone();
            let config: cpal::StreamConfig = supported.clone().into();
            device.build_output_stream(
                &config,
                move |data: &mut [f32], _| fill_output_buffer(data, channels, &buss_for_cb),
                err_fn,
                None,
            )?
        }
        other => return Err(format!("Unsupported sample format: {other:?}").into()),
    };


    Ok(AudioEngine { _stream: stream , _input: buss, sample_rate: supported.sample_rate().0})
}

fn fill_output_buffer(data: &mut [f32], channels: usize, buss: &Arc<Mutex<Buss>>) {
	let frames = data.len() / channels;
	// Render exactly 'frames' samples per channel using write_f32 as per docs
	let mut left = vec![0.0_f32; frames];
	let mut right = vec![0.0_f32; frames];
	if let Ok(mut guard) = buss.lock() {
		// https://docs.rs/oxisynth/0.1.0/oxisynth/struct.Synth.html#method.write
		guard.write_f32(frames, &mut left, 0, 1, &mut right, 0, 1);
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
    // sprintln!("{:#?}", data);
}