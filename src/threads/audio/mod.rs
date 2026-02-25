

pub mod buss;
pub mod interfaces; //TODO: Make private
pub mod buffered_output; //TODO: Make private
pub mod stereo_output;

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;


use super::engine::actions::{Actions, SystemActions};
use std::error::Error;
use std::sync::mpsc;

use buss::BussConsumer;
use interfaces::Output;
use stereo_output::StereoOutputController;

pub struct AudioEngine {
	_stream: Option<cpal::Stream>,
    pub sample_rate: u32,
}

impl AudioEngine {
    // pub fn start(&mut self) -> Result<(), cpal::PlayStreamError>{
    //     self._stream.play()
    // }
    // pub fn pause(&mut self) -> Result<(), cpal::PauseStreamError>{
    //     self._stream.pause()
    // }
    
    /// Create a dummy AudioEngine for use when audio initialization fails (e.g., in tests)
    pub fn dummy(sample_rate: u32) -> Self {
        Self {
            _stream: None,
            sample_rate,
        }
    }
}


// Production init_audio: uses CPAL to create a real output stream.
#[cfg(not(test))]
pub(crate) fn init_audio(
    tx: &mpsc::Sender<Actions>,
) -> Result<(AudioEngine, StereoOutputController), Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No default output device")?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("Failed to get default output config: {e}"))?;
    let channels = supported.channels() as usize;

    let (mut buss_consumer, stereo_output) = StereoOutputController::new();

    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let config: cpal::StreamConfig = supported.clone().into();
            let tx = tx.clone();
            let _ = tx.send(Actions::Internal(SystemActions::SetSampleRate(config.sample_rate.0)));
            device.build_output_stream(
                &config,
                move |data: &mut [f32], _| fill_output_buffer(data, channels, &mut buss_consumer, &tx),
                err_fn,
                None,
            )?
        }
        other => return Err(format!("Unsupported sample format: {other:?}").into()),
    };


    Ok((
        AudioEngine {
            _stream: Some(stream),
            sample_rate: supported.sample_rate().0,
        },
        stereo_output,
    ))
}

// Test-only init_audio: no CPAL, no real audio stream.
// All tests run against a dummy AudioEngine and an in-memory StereoOutputController.
#[cfg(test)]
pub(crate) fn init_audio(
    tx: &mpsc::Sender<Actions>,
) -> Result<(AudioEngine, StereoOutputController), Box<dyn Error>> {
    let sample_rate = 44_100u32;
    // Keep engine PlayerState.sample_rate consistent with the dummy engine.
    let _ = tx.send(Actions::Internal(SystemActions::SetSampleRate(
        sample_rate,
    )));
    let (_consumer, stereo_output) = StereoOutputController::new();
    Ok((AudioEngine::dummy(sample_rate), stereo_output))
}

fn fill_output_buffer(data: &mut [f32], channels: usize, buss: &mut BussConsumer, tx: &mpsc::Sender<Actions>) {
	let frames = data.len() / channels;
	// Render exactly 'frames' samples per channel using write_f32 as per docs
	let mut left = vec![0.0_f32; frames];
	let mut right = vec![0.0_f32; frames];
    // https://docs.rs/oxisynth/0.1.0/oxisynth/struct.Synth.html#method.write
    buss.write_f32(frames, &mut left, 0, 1, &mut right, 0, 1);

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

    use crate::threads::audio::buss::{Buss, BussProducer};

    use super::*;


    // Audio Engine
    #[test]
    fn audio_engine_start() {
        let (tx, _) = mpsc::channel::<Actions>();
        let (engine, _stereo) =
            init_audio(&tx).expect("init_audio should succeed in tests with dummy audio");
        // In tests we must never create a real CPAL stream.
        assert!(engine._stream.is_none());
        assert_eq!(engine.sample_rate, 44_100);
    }

    // Test transferring data from Buss to audio output
    const MOCK_INPUT_LEN: usize = 10; 
    struct MockInput {
        lbuff: [f32; buss::BUF_SIZE],
        rbuff: [f32; buss::BUF_SIZE],
    }
    impl MockInput {
        fn new() -> Self {
            // Initialize with test pattern for first 10 values, rest filled with zeros
            let mut lbuff = [0.0_f32; buss::BUF_SIZE];
            let mut rbuff = [0.0_f32; buss::BUF_SIZE];
            let test_left = [0.0, 0.01, 0.02, 0.03, 0.04, 0.05, 0.06, 0.07, 0.08, 0.09];
            let test_right = [0.10, 0.11, 0.12, 0.13, 0.14, 0.15, 0.16, 0.17, 0.18, 0.19];
            lbuff[..MOCK_INPUT_LEN].copy_from_slice(&test_left);
            rbuff[..MOCK_INPUT_LEN].copy_from_slice(&test_right);
            Self { lbuff, rbuff }
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
            // Write up to len samples, repeating the pattern if needed
            for i in 0..len {
                left_out[i] = self.lbuff[i % self.lbuff.len()];
                right_out[i] = self.rbuff[i % self.rbuff.len()];
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
        // Set up ring buffer and populate it with test data
        let (mut consumer, mut producer) = BussProducer::new();
        let mut buss = Buss::new();
        let input: Box<dyn Output> = Box::new(MockInput::new()); 
        buss.add_input(input);
        // Populate ring buffer
        producer.add_input(Box::new(buss));
        producer.on_tick();
        // Test
        let mut data = [0.0_f32; MOCK_INPUT_LEN];
        let (tx, _) = mpsc::channel::<Actions>();
        fill_output_buffer(&mut data, 1, &mut consumer, &tx);
        assert_approx_eq_array(&data, &[0.05, 0.06, 0.07, 0.08, 0.09, 0.10, 0.11, 0.12, 0.13, 0.14])
    }

    #[test]
    fn fill_data_buffer_should_interleave_stereo_for_stereo_output() {
        // Set up ring buffer and populate it with test data
        let (mut consumer, mut producer) = BussProducer::new();
        let mut buss = Buss::new();
        let input: Box<dyn Output> = Box::new(MockInput::new()); 
        buss.add_input(input);
        // Populate ring buffer
        producer.add_input(Box::new(buss));
        producer.on_tick();
        // Test
        let mut data = [0.0_f32; 2*MOCK_INPUT_LEN];
        let (tx, _) = mpsc::channel::<Actions>();
        fill_output_buffer(&mut data, 2, &mut consumer, &tx);
        assert_approx_eq_array(&data, &[0.0, 0.1, 0.01, 0.11, 0.02, 0.12, 0.03, 0.13, 0.04, 0.14, 0.05, 0.15, 0.06, 0.16, 0.07, 0.17, 0.08, 0.18, 0.09, 0.19])
    }

    #[test]
    fn fill_data_buffer_should_interleave_stereo_for_multichannel_output() {
        // Set up ring buffer and populate it with test data
        let (mut consumer, mut producer) = BussProducer::new();
        let mut buss = Buss::new();
        let input: Box<dyn Output> = Box::new(MockInput::new()); 
        buss.add_input(input);
        // Populate ring buffer
        producer.add_input(Box::new(buss));
        producer.on_tick();
        // Test
        let mut data = [0.0_f32; 3*MOCK_INPUT_LEN];
        let (tx, _) = mpsc::channel::<Actions>();
        fill_output_buffer(&mut data, 3, &mut consumer, &tx);
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