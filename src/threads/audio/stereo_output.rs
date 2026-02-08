use super::buss::{Buss, BussProducer};

/// Controller for stereo output that manages the ring buffer producer
/// and populates it from a final buss when capacity is available
pub struct StereoOutputController {
    producer: BussProducer,
}

impl StereoOutputController {
    pub fn new() -> (super::buss::BussConsumer, StereoOutputController) {
        let (consumer, producer) = BussProducer::new();
        let controller = StereoOutputController {
            producer,
        };
        (consumer, controller)
    }

    /// Called on each tick to populate the ring buffer from the final buss if capacity is available
    pub fn on_tick(&mut self, final_buss: &mut Buss) {
        if self.producer.has_capacity() {
            self.producer.write_from_buss(final_buss);
        }
    }

    /// Check if the ring buffer needs more data (for more frequent polling if needed)
    pub fn needs_data(&self) -> bool {
        self.producer.has_capacity()
    }
}
