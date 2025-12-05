pub enum Actions {
    PlayMidi,
    Pause,
    Quit,
    Internal(SystemActions),
}

pub enum SystemActions {
    SamplesPlayed(usize),
    SetSampleRate(u32),
    PlaybackStarted,
    PlaybackFinished,
}
