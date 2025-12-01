pub enum Actions {
    PlayMidi,
    Quit,
    Internal(SystemActions),
}

pub enum SystemActions {
    SamplesPlayed(usize),
    SetSampleRate(u32),
    PlaybackFinished,
}
