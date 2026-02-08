use std::any::Any;

pub trait Output: Any + Send + Sync {
    // fn write<S: IsSamples>(&mut self, samples: S);
    // Needs to be mutable to allow buffer usage and storage of state
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

// Enabled outputs to be downcast to original type
impl dyn Output {
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
