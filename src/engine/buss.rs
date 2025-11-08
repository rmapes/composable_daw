/*
BUSS is a mechanism to take multiple audio inputs and combine into a single output
*/

use std::cmp::min;

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
    inputs: Vec<Box<dyn Output>>,
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

/////////////////////////
///  Tests
/// 

#[cfg(test)]
mod tests {
 
    use super::*;

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
        fn write_f32(&mut self, len: usize, left_out: &mut [f32], loff: usize, lincr: usize, right_out: &mut [f32], roff: usize, rincr: usize) {
            for i in 0..len {
                left_out[loff + lincr*i] = self.lbuff[i];
                right_out[roff + rincr*i] = self.rbuff[i];
            }
        }
    }


    // Buss

    #[test]
    fn buss_can_add_input() {
        let mut buss = Buss::new();
        buss.add_input(Box::new(MockInput::new()));
        assert_eq!(buss.inputs.len(), 1);
    }

    #[test]
    fn buss_can_write_f32() {
        let mut buss = Buss::new();
        let input = MockInput::new();
        let expected_left_out = input.lbuff.clone();
        let expected_right_out = input.rbuff.clone();
        buss.add_input(Box::new(input));
        let mut left_out = [0.0_f32; 10];
        let mut right_out = [0.0_f32; 10];
        buss.write_f32(10, &mut left_out, 0, 1, &mut right_out, 0, 1);
        assert_eq!(left_out, expected_left_out);
        assert_eq!(right_out, expected_right_out);
    }

    // BufferedOutput

    #[test]
    fn buffered_output_can_be_created() {
        let buffered_output = BufferedOutput::new();
        assert_eq!(buffered_output.left_buf.len(), 0);
        assert_eq!(buffered_output.right_buf.len(), 0);
    }

    #[test]
    fn buffered_output_can_read_f32() {
        let mut buffered_output = BufferedOutput::new();
        let mut input = MockInput::new();
        buffered_output.read_f32(10, &mut input);
        assert_eq!(buffered_output.left_buf, input.lbuff);
        assert_eq!(buffered_output.right_buf, input.rbuff);
    }
}