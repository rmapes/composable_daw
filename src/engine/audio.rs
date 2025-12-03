

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;

use super::actions::{Actions, SystemActions};
use super::buss::{Buss, Output};
use std::sync::Mutex;
use std::sync::Arc;
use std::error::Error;
use std::sync::RwLock;
use std::sync::mpsc;

pub struct AudioEngine {
	_stream: cpal::Stream,
    _input: Arc<Mutex<Buss>>,
    pub sample_rate: u32,
}

impl AudioEngine {
    pub fn start(&mut self) -> Result<(), cpal::PlayStreamError>{
        self._stream.play()
    }
    pub fn pause(&mut self) -> Result<(), cpal::PauseStreamError>{
        self._stream.pause()
    }
    pub fn add_input(&mut self, o: Arc<RwLock<Box<dyn Output>>>) {
        if let Ok(mut guard) = self._input.lock() {
            guard.add_input(o);
        }
    }
}


pub(crate) fn init_audio(tx: &mpsc::Sender<Actions>) -> Result<AudioEngine, Box<dyn Error>> {
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
            let tx = tx.clone();
            let _ = tx.send(Actions::Internal(SystemActions::SetSampleRate(config.sample_rate.0)));
            device.build_output_stream(
                &config,
                move |data: &mut [f32], _| fill_output_buffer(data, channels, &buss_for_cb, &tx),
                err_fn,
                None,
            )?
        }
        other => return Err(format!("Unsupported sample format: {other:?}").into()),
    };


    Ok(AudioEngine { _stream: stream , _input: buss, sample_rate: supported.sample_rate().0})
}

fn fill_output_buffer(data: &mut [f32], channels: usize, buss: &Arc<Mutex<Buss>>, tx: &mpsc::Sender<Actions>) {
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
    // Tell the system to move the playhead. Send samples per channel, not total samples
    let _ = tx.send(Actions::Internal(SystemActions::SamplesPlayed(data.len()/channels)));
}


/////////////////////////
///  Tests
/// 

#[cfg(test)]
mod tests {

    use super::*;


    // Audio Engine
    #[test]
    fn audio_engine_start() {
        // Smoke test to make sure everything works
        let (tx, _) = mpsc::channel::<Actions>();
        let engine = init_audio(&tx);
        assert!(engine.is_ok());
    }

    // Test transferring data from Buss to audio output
    const MOCK_INPUT_LEN: usize = 10; 
    struct MockInput {
        lbuff: [f32;MOCK_INPUT_LEN],
        rbuff: [f32;MOCK_INPUT_LEN],
    }
    impl MockInput {
        fn new() -> Self {
            Self {
                lbuff: [0.0, 0.01, 0.02, 0.03, 0.04, 0.05, 0.06, 0.07, 0.08, 0.09],
                rbuff: [0.10, 0.11, 0.12, 0.13, 0.14, 0.15, 0.16, 0.17, 0.18, 0.19],
            }
        }
    }
    impl Output for MockInput {
        fn write_f32(&mut self, 
            len: usize, 
            left_out: &mut [f32], 
            _loff: usize, 
            _lincr: usize, 
            right_out: &mut [f32], 
            _roff: usize, 
            _rincr: usize,
        ) {
            assert!(len == MOCK_INPUT_LEN); // Fixing the size for test purposes
            for i in 0..len {
                left_out[i] = self.lbuff[i];
                right_out[i] = self.rbuff[i];
            }
        }
    }
    macro_rules! assert_approx_eq {
        ($x:expr, $y:expr, $d:expr) => {
            if !($x - $y < $d || $y - $x < $d) { panic!(); }
        }
    }

    fn assert_approx_eq_array(ary: &[f32], expected:&[f32]) {
        assert_eq!(ary.len(), expected.len());
        for i in 0..ary.len() {
            assert_approx_eq!(ary[i], expected[i], 0.0001);
        }
    }

    #[test]
    fn fill_data_buffer_should_combine_stereo_for_mono_output() {
        // Set up
        let mut raw_buss = Buss::new();
        let input: Arc<RwLock<Box<dyn Output>>> = Arc::new(RwLock::new(Box::new(MockInput::new()))); 
        raw_buss.add_input(input);
        let buss = Arc::new(Mutex::new(raw_buss));
        // Test
        let mut data = [0.0_f32; MOCK_INPUT_LEN];
        let (tx, _) = mpsc::channel::<Actions>();
        fill_output_buffer(&mut data, 1, &buss, &tx);
        assert_approx_eq_array(&data, &[0.05, 0.06, 0.07, 0.08, 0.09, 0.10, 0.11, 0.12, 0.13, 0.14])
    }

    #[test]
    fn fill_data_buffer_should_interleave_stereo_for_stereo_output() {
        // Set up
        let mut raw_buss = Buss::new();
        let input: Arc<RwLock<Box<dyn Output>>> = Arc::new(RwLock::new(Box::new(MockInput::new()))); 
        raw_buss.add_input(input);
        let buss = Arc::new(Mutex::new(raw_buss));
        // Test
        let mut data = [0.0_f32; 2*MOCK_INPUT_LEN];
        let (tx, _) = mpsc::channel::<Actions>();
        fill_output_buffer(&mut data, 2, &buss, &tx);
        assert_approx_eq_array(&data, &[0.0, 0.1, 0.01, 0.11, 0.02, 0.12, 0.03, 0.13, 0.04, 0.14, 0.05, 0.15, 0.06, 0.16, 0.07, 0.17, 0.08, 0.18, 0.09, 0.19])
    }

    #[test]
    fn fill_data_buffer_should_interleave_stereo_for_multichannel_output() {
        // Set up
        let mut raw_buss = Buss::new();
        let input: Arc<RwLock<Box<dyn Output>>> = Arc::new(RwLock::new(Box::new(MockInput::new()))); 
        raw_buss.add_input(input);
        let buss = Arc::new(Mutex::new(raw_buss));
        // Test
        let mut data = [0.0_f32; 3*MOCK_INPUT_LEN];
        let (tx, _) = mpsc::channel::<Actions>();
        fill_output_buffer(&mut data, 3, &buss, &tx);
        // Interleave, but fill channels above 2 with 0.0
        assert_approx_eq_array(&data, &[
            0.0, 0.1, 0.0, 
            0.01, 0.11, 0.0,
            0.02, 0.12, 0.0,
            0.03, 0.13, 0.0,
            0.04, 0.14, 0.0,
            0.05, 0.15, 0.0,
            0.06, 0.16, 0.0,
            0.07, 0.17, 0.0,
            0.08, 0.18, 0.0,
            0.09, 0.19, 0.0,
        ])
    }
}