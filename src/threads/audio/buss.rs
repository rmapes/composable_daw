/*
BUSS is a mechanism to take multiple audio inputs and combine into a single output
*/

use std::cmp::min;

use ringbuf::{HeapCons, HeapProd, HeapRb, traits::*};

use super::interfaces::Output;

const BUF_SIZE: usize = 512;
const RING_BUF_SIZE: usize = 2 * BUF_SIZE;

/// A Buss that combines multiple audio inputs into a single output
pub struct Buss {
    inputs: Vec<Box<dyn Output>>,
    buf_size: usize,
    left_buf: [f32; BUF_SIZE],
    right_buf: [f32; BUF_SIZE],
}

impl Buss {
    pub fn new() -> Self {
        Buss {
            inputs: Vec::new(),
            buf_size: BUF_SIZE,
            left_buf: [0.0_f32; BUF_SIZE],
            right_buf: [0.0_f32; BUF_SIZE],
        }
    }

    pub fn add_input(&mut self, input: Box<dyn Output>) {
        self.inputs.push(input);
    }
}

impl Output for Buss {
    fn write_f32(&mut self, 
        len: usize, 
        left_out: &mut [f32], 
        loff: usize, 
        lincr: usize, 
        right_out: &mut [f32], 
        roff: usize, 
        rincr: usize,
    ) {
        for input in self.inputs.iter_mut() {
            let buf_size = min(len, self.buf_size);
            let mut loff_local = loff;
            let mut roff_local = roff;
            let mut bytes_left = len;
            while bytes_left > 0 {
                let bytes_to_read = min(buf_size, bytes_left);
                input.write_f32(bytes_to_read, &mut self.left_buf, 0, 1, &mut self.right_buf, 0, 1);
                bytes_left -= bytes_to_read;
                for i in 0..bytes_to_read {
                    left_out[loff_local] = 1.0_f32.min(left_out[loff_local] + self.left_buf[i]);
                    loff_local += lincr;
                    right_out[roff_local] = 1.0_f32.min(right_out[roff_local] + self.right_buf[i]);
                    roff_local += rincr
                }
            }
        }
    }
}

/// Ring buffer consumer for reading audio from the ring buffer (used by CPAL audio thread)
pub struct BussConsumer {
    buf_size: usize,
    left_input: HeapCons<f32>,
    right_input: HeapCons<f32>
}

// SAFETY: BussConsumer is only used in the CPAL audio callback thread, which is single-threaded.
// The ring buffer is lock-free and designed for cross-thread communication, so this is safe.
unsafe impl Sync for BussConsumer {}

impl Output for BussConsumer {
    /// Pulls samples from the ringbuffer into the provided output slices
    fn write_f32(&mut self, 
        len: usize, 
        left_out: &mut [f32], 
        _loff: usize, 
        _lincr: usize, 
        right_out: &mut [f32], 
        _roff: usize, 
        _rincr: usize,
    ) {
        // pop_slice returns how many items were actually copied
        // We only use the first len elements, ignoring offset/increment for simplicity
        let left_slice = if len <= left_out.len() { &mut left_out[..len] } else { left_out };
        let right_slice = if len <= right_out.len() { &mut right_out[..len] } else { right_out };
        let _ = self.left_input.pop_slice(left_slice);
        let _ = self.right_input.pop_slice(right_slice);        
    }
}

/// Ring buffer producer for writing audio to the ring buffer (used by engine thread)
pub struct BussProducer {
    inputs: Vec<Box<dyn Output>>,
    // Maintain Heap buffers to avoid allocating memory during playback
    buf_size: usize, // Fix in new. Cannot be altered, as tied to Heapally allocated buffers
    left_buf: [f32; BUF_SIZE],
    right_buf: [f32; BUF_SIZE],
    left_output: HeapProd<f32>,
    right_output: HeapProd<f32>
}

impl BussProducer {
    pub fn new() -> (BussConsumer, BussProducer) {
        let (left_prod, left_cons) = HeapRb::<f32>::new(RING_BUF_SIZE).split();
        let (right_prod, right_cons) = HeapRb::<f32>::new(RING_BUF_SIZE).split();
        let producer = BussProducer {
            inputs: Vec::new(),
            buf_size: BUF_SIZE,
            left_buf: [0.0_f32; BUF_SIZE],
            right_buf: [0.0_f32; BUF_SIZE],
            left_output: left_prod,
            right_output: right_prod
        };
        let consumer = BussConsumer {
            buf_size: BUF_SIZE,
            left_input: left_cons,
            right_input: right_cons,
        };
        (consumer, producer)
    }

    // Note Buss should not own inputs, only borrow them
    pub fn add_input(&mut self, input: Box<dyn Output>) {
        self.inputs.push(input);
    }

    pub fn on_tick(&mut self) {
        // Repopulate buffers
        let left_space = min(self.buf_size, self.left_output.vacant_len());
        let right_space = min(self.buf_size, self.right_output.vacant_len());
        let left_buf = &mut vec![0.0_f32; left_space];
        let left = left_buf.as_mut_slice();
        let right_buf = &mut vec![0.0_f32; right_space];
        let right = right_buf.as_mut_slice();
        self.write_f32(min(left_space, right_space), left,0, 1, right, 0, 1);
        self.left_output.push_slice(left);
        self.right_output.push_slice(right);
    }

    /// Check if there's capacity in the ring buffer
    pub fn has_capacity(&self) -> bool {
        self.left_output.vacant_len() >= self.buf_size && self.right_output.vacant_len() >= self.buf_size
    }

    /// Get the available capacity (minimum of left and right)
    pub fn available_capacity(&self) -> usize {
        min(self.left_output.vacant_len(), self.right_output.vacant_len())
    }

    /// Write audio from a Buss into the ring buffer
    pub fn write_from_buss(&mut self, buss: &mut Buss) {
        let capacity = min(self.buf_size, self.available_capacity());
        if capacity > 0 {
            let mut left_buf = vec![0.0_f32; capacity];
            let mut right_buf = vec![0.0_f32; capacity];
            buss.write_f32(capacity, &mut left_buf, 0, 1, &mut right_buf, 0, 1);
            let left_written = self.left_output.push_slice(&left_buf);
            let right_written = self.right_output.push_slice(&right_buf);
            // Both should write the same amount, but handle gracefully if not
            let _ = min(left_written, right_written);
        }
    }
}

// SAFETY: BussProducer is only used in the engine thread, which is single-threaded.
// The ring buffer is lock-free and designed for cross-thread communication, so this is safe.
unsafe impl Sync for BussProducer {}

impl Output for BussProducer {
    fn write_f32(&mut self, 
        len: usize, 
        left_out: &mut [f32], 
        loff: usize, 
        lincr: usize, 
        right_out: &mut [f32], 
        roff: usize, 
        rincr: usize,
    ) {
        for input in self.inputs.iter_mut() {
            let buf_size = min(len, self.buf_size);
            let mut loff_local = loff;
            let mut roff_local = roff;
            let mut bytes_left = len;
            while bytes_left > 0 {
                let bytes_to_read = min(buf_size, bytes_left);
                input.write_f32(bytes_to_read, &mut self.left_buf, 0, 1, &mut self.right_buf, 0, 1);
                bytes_left -= bytes_to_read;
                for i in 0..bytes_to_read {
                    left_out[loff_local] = 1.0_f32.min(left_out[loff_local] + self.left_buf[i]);
                    loff_local += lincr;
                    right_out[roff_local] = 1.0_f32.min(right_out[roff_local] + self.right_buf[i]);
                    roff_local += rincr
                }
            }
        }
    }
}


// /////////////////////////
// ///  Tests
// /// 

// #[cfg(test)]
// mod tests {
 
//     use super::super::buffered_output::BufferedOutput;

//     use super::*;

//     const MOCK_INPUT_LEN: usize = 10; 
//     struct MockInput {
//         lbuff: [f32;MOCK_INPUT_LEN],
//         rbuff: [f32;MOCK_INPUT_LEN],
//     }
//     impl MockInput {
//         fn new() -> Self {
//             Self {
//                 lbuff: [0.0, 0.01, 0.02, 0.03, 0.04, 0.05, 0.06, 0.07, 0.08, 0.09],
//                 rbuff: [0.10, 0.11, 0.12, 0.13, 0.14, 0.15, 0.16, 0.17, 0.18, 0.19],
//             }
//         }
//     }
//     impl Output for MockInput {
//         fn write_f32(&mut self, len: usize, left_out: &mut [f32], loff: usize, lincr: usize, right_out: &mut [f32], roff: usize, rincr: usize) {
//             for i in 0..len {
//                 left_out[loff + lincr*i] = self.lbuff[i];
//                 right_out[roff + rincr*i] = self.rbuff[i];
//             }
//         }
//     }


//     // Buss

//     #[test]
//     fn buss_can_add_input() {
//         let mut buss = Buss::new();
//         let input: Box<dyn Output> = Box::new(MockInput::new());
//         buss.add_input(input);
//         assert_eq!(buss.inputs.len(), 1);
//     }

//     #[test]
//     fn buss_can_write_f32() {
//         let mut buss = Buss::new();
//         let input = Box::new(MockInput::new());
//         let expected_left_out = input.lbuff.clone();
//         let expected_right_out = input.rbuff.clone();
//         buss.add_input(input);
//         let mut left_out = [0.0_f32; 10];
//         let mut right_out = [0.0_f32; 10];
//         buss.write_f32(10, &mut left_out, 0, 1, &mut right_out, 0, 1);
//         assert_eq!(left_out, expected_left_out);
//         assert_eq!(right_out, expected_right_out);
//     }

//     #[test]
//     fn multiple_inputs_are_merged() {
//         let mut buss = Buss::new();
//         let mut input = MockInput::new();
//         for _ in 0..10 {
//             let mut buffered_output = BufferedOutput::new();
//             buffered_output.read_f32(10, &mut input);
//             buss.add_input(Box::new(buffered_output))
//         }
//         let input = MockInput::new();
//         let expected_left_out = input.lbuff.clone().map(|i| {10.0*i});
//         let expected_right_out = input.rbuff.clone().map(|i| {(10.0*i).min(1.0)});
//         // Get outputs
//         let mut left_out = [0.0_f32; 10];
//         let mut right_out = [0.0_f32; 10];
//         buss.write_f32(10, &mut left_out, 0, 1, &mut right_out, 0, 1);
//         for i in 0..expected_left_out.len() {
//             assert!((left_out[i] - expected_left_out[i]).abs() < 0.011, "Left:  {} != {}", left_out[i], expected_left_out[i]);
//             // Should saturate at 1
//             assert!((right_out[i] - expected_right_out[i]).abs() < 0.11, "Right:  {} != {}", right_out[i], expected_right_out[i]);
//         }
//     }

// }