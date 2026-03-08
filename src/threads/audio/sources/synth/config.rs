use std::path::PathBuf;

use crate::models::instrument::{InstrumentActions, InstrumentConfig};
use crate::models::shared::TrackIdentifier;

const SOUNDFONT_DIR_PATH: &str = "./soundfonts/";

/// UI messages for the synth instrument editor. Only the synth module and instrument editor handle these.
#[derive(Debug, Clone)]
pub enum SynthMessage {
    SelectSoundFont(TrackIdentifier),
    SetSoundFont(TrackIdentifier, Option<PathBuf>),
}

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

}

impl InstrumentConfig for SimpleSynth {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn crate::models::instrument::InstrumentConfig> {
        Box::new(self.clone())
    }

    fn apply_action(&mut self, action: &InstrumentActions) -> bool {
        match action {
            InstrumentActions::SetSoundFont(soundfont_path) => {
                if let Some(path) = soundfont_path
                    && let Some(file_name) = path.file_name().and_then(|os| os.to_str())
                {
                    self.soundfont = file_name.to_string();
                    return true;
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

pub fn get_soundfont_path(soundfont: &String) -> String {
    format!("{SOUNDFONT_DIR_PATH}{0}", soundfont)
}
