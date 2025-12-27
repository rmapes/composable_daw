#[derive(Debug, Clone)]
pub enum Actions {
    Play,
    Pause,
    Quit,
    Internal(SystemActions),
}

#[derive(Debug, Clone)]
pub enum SystemActions {
    SamplesPlayed(usize),
    SetSampleRate(u32),
    PlaybackStarted,
    PlaybackFinished,
}
