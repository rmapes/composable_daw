use crate::threads::audio::sources::synth::SynthMessage;


/// Events for the instrument editor. Dispatched by the editor; only the instrument editor and
/// instrument-specific UIs (e.g. synth editor) need to know the variants.
#[derive(Debug, Clone)]
pub enum Event {
    Synth(SynthMessage),
}
