use super::interfaces::Output;

// Ignore unused code as we will use it in a later iteration
#[allow(dead_code)]
pub struct BufferedOutput {
    pub(crate) left_buf: Vec<f32>,
    pub(crate) right_buf: Vec<f32>,
    pub(crate) left_read_start: usize,
    pub(crate) right_read_start: usize,
}

impl BufferedOutput {
    #[allow(dead_code)]
    pub fn new() -> Self {
        BufferedOutput {
            left_buf: Vec::new(),
            right_buf: Vec::new(),
            left_read_start: 0,
            right_read_start: 0,
        }
    }
    #[allow(dead_code)]
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

    // Mock Input (duplicate from buss TODO: refactor into shared resource)
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
