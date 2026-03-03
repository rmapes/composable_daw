use std::path::PathBuf;

const SOUNDFONT_DIR_PATH: &str = "./soundfonts/";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SimpleSynth {
    pub soundfont: String,
    pub bank: u32,
    pub program: u8,
}

impl Default for SimpleSynth {
    fn default() -> Self {
        Self {
            soundfont: "airfont_340.sf2".to_string(),
            bank: 0,
            program: 0,
        }
    }
}

impl SimpleSynth {
    pub fn get_soundfont_path(&self) -> String {
        get_soundfont_path(&self.soundfont)
    }

    /// Apply an instrument-level action to this synth.
    /// Returns true if this synth's configuration changed.
    pub fn handle_instrument_action(&mut self, action: &InstrumentActions) -> bool {
        match action {
            InstrumentActions::SetSoundFont(soundfont_path) => {
                if let Some(path) = soundfont_path {
                    if let Some(file_name) = path
                        .file_name()
                        .and_then(|os| os.to_str())
                    {
                        self.soundfont = file_name.to_string();
                        return true;
                    }
                }
                false
            }
            InstrumentActions::SetBank(bank) => {
                self.bank = *bank;
                true
            }
            InstrumentActions::SetProgram(program) => {
                self.program = *program;
                true
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Instrument {
    Synth(SimpleSynth),
}

#[derive(Debug, Clone)]
pub enum InstrumentActions {
    SetSoundFont(Option<PathBuf>),
    SetBank(u32),
    SetProgram(u8),
}

pub fn get_soundfont_path(soundfont: &String) -> String {
    format!("{SOUNDFONT_DIR_PATH}{0}", soundfont)
}
