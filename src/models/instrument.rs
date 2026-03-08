use std::any::Any;
use std::path::PathBuf;

/// Commands that can be applied to an instrument (e.g. set soundfont, bank, program).
/// Engine and UI use this type; instrument implementations interpret it.
#[derive(Debug, Clone)]
pub enum InstrumentActions {
    SetSoundFont(Option<PathBuf>),
    SetBank(u32),
    SetProgram(u8),
}

/// Trait for instrument configuration. Implemented per instrument type (e.g. SimpleSynth);
/// only the registry and the implementing module need to know the concrete type.
pub trait InstrumentConfig: std::fmt::Debug + Send {
    /// Apply an instrument action. Returns true if the config changed (e.g. for project save).
    fn apply_action(&mut self, action: &InstrumentActions) -> bool;

    /// For downcast to concrete config type inside the instrument module.
    fn as_any(&self) -> &dyn Any;

    /// Clone this config into a new box. Used when cloning VirtualInstrument.
    fn clone_box(&self) -> Box<dyn InstrumentConfig>;
}
