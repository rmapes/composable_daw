/*
BUSS is a mechanism to take multiple audio inputs and combine into a single output
*/

use std::cmp::min;
use std::sync::{Arc, Mutex};

pub trait Output: Send + Sync {
    // fn write<S: IsSamples>(&mut self, samples: S);
    fn write_f32(&mut self, 
        len: usize, 
        left_out: &mut [f32], 
        loff: usize, 
        lincr: usize, 
        right_out: &mut [f32], 
        roff: usize, 
        rincr: usize,
    );
    // fn write_f64(
    //         &mut self,
    //         len: usize,
    //         left_out: &mut [f64],
    //         loff: usize,
    //         lincr: usize,
    //         right_out: &mut [f64],
    //         roff: usize,
    //         rincr: usize,
    // );      
}

const BUF_SIZE: usize = 512;

pub struct Buss {
    inputs: Vec<Arc<Mutex<dyn Output>>>,
    // Maintain static buffers to avoid allocating memory during playback
    buf_size: usize, // Fix in new. Cannot be altered, as tied to statically allocated buffers
    left_buf: [f32; BUF_SIZE],
    right_buf: [f32; BUF_SIZE],
}


impl Buss {
    pub fn new() -> Buss {
        Buss {
            inputs: Vec::new(),
            buf_size: BUF_SIZE,
            left_buf: [0.0_f32; BUF_SIZE],
            right_buf: [0.0_f32; BUF_SIZE],
        }
    }
    // Note Buss should not own inputs, only borrow them
    pub fn add_input(&mut self, input: Arc<Mutex<dyn Output>>) {
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
            let Ok(mut guard) = input.lock() else {
                input.clear_poison();
                continue;
            }; 
            while bytes_left > 0 {
                let bytes_to_read = min(buf_size, bytes_left);
                (*guard).write_f32(bytes_to_read, &mut self.left_buf, 0, 1, &mut self.right_buf, 0, 1);
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

pub struct BufferedOutput {
    left_buf: Vec<f32>,
    right_buf: Vec<f32>,
    left_read_start: usize,
    right_read_start: usize,
}

impl BufferedOutput {
    pub fn new() -> Self {
        BufferedOutput {
            left_buf: Vec::new(),
            right_buf: Vec::new(),
            left_read_start: 0,
            right_read_start: 0,
        }
    }
    pub fn read_f32<T: Output>(&mut self, len: usize, input: &mut T) {
        let loff = self.left_buf.len();
        let roff = self.right_buf.len();
        self.left_buf.resize(loff+len, 0.0_f32);
        self.right_buf.resize(roff+len, 0.0_f32);
        input.write_f32(len, self.left_buf.as_mut_slice(), loff, 1, self.right_buf.as_mut_slice(), roff, 1);
    }
}


impl Output for BufferedOutput {
    fn write_f32(&mut self, 
        len: usize, 
        left_out: &mut [f32], 
        loff: usize, 
        lincr: usize, 
        right_out: &mut [f32], 
        roff: usize, 
        rincr: usize,
    ) {
        for i in 0..len {
            left_out[loff + lincr*i] = *self.left_buf.get(self.left_read_start + i).unwrap_or(&0.0_f32);
            right_out[roff + rincr*i] = *self.right_buf.get(self.right_read_start + i).unwrap_or(&0.0_f32);
        }
        self.left_read_start += len;
        self.right_read_start += len;
    }
}