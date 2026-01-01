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
        format!("{SOUNDFONT_DIR_PATH}{0}", self.soundfont)
    }
}

#[derive(Debug, Clone)]
pub enum Instrument {
    Synth(SimpleSynth)
}