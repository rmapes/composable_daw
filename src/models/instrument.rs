use std::any::Any;
use std::sync::Arc;

/// Opaque instrument action. Engine and UI pass this without knowing the variants.
/// Instrument implementations (e.g. synth) downcast to their concrete action type.
/// Uses Arc so Actions (and Message) can derive Clone.
pub type InstrumentAction = Arc<dyn Any + Send + Sync>;

/// Trait for instrument configuration. Implemented per instrument type (e.g. SimpleSynth);
/// only the registry and the implementing module need to know the concrete type.
pub trait InstrumentConfig: std::fmt::Debug + Send {
    /// Apply an instrument action. Returns true if the config changed (e.g. for project save).
    /// Implementations downcast `action` to their concrete action type (e.g. SynthActions).
    fn apply_action(&mut self, action: &dyn Any) -> bool;

    /// For downcast to concrete config type inside the instrument module.
    fn as_any(&self) -> &dyn Any;

    /// Clone this config into a new box. Used when cloning VirtualInstrument.
    fn clone_box(&self) -> Box<dyn InstrumentConfig>;
}
