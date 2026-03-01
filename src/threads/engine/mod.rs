pub mod actions;
mod sources;

use std::collections::HashMap;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::SendError;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;

use log::{error, info};

use audio::sources::synth::SynthActions;
use sources::AudioSources;
use super::audio::controllers::MidiSendersMap;
use super::audio;
use super::audio::controllers::preview::{self, PreviewMessage};
use crate::models::components::Track;
use crate::models::instuments::Instrument;
use crate::models::sequences::MidiNote;
use crate::models::shared::{ProjectData, RegionType, TrackIdentifier};




#[derive(Debug)]
pub struct EngineController {
    tx: mpsc::Sender<actions::Actions>,
    pub data_change_receiver: flume::Receiver<ProjectData>,
}

impl EngineController {
    pub fn send(&self, action: actions::Actions) -> Result<(), SendError<actions::Actions>> {
        self.tx.send(action)
    }
    pub fn quit(&self) {
        let _ = self.tx.send(actions::Actions::Quit);
    }
}


#[derive(Debug, Clone)]
pub struct PlayerState {
    pub is_playing: bool,
    pub is_active: bool,
    pub is_audio_initialized: bool, // Only for use in this file
    // State for tracking audio output
    pub playhead: u32,


    // Internal system tracking
    samples_played: usize,
    sample_rate: u32,
}

impl PlayerState {
    pub fn new() -> Self {
        Self { 
            is_playing: false, 
            is_active: true, 
            is_audio_initialized: false,
            playhead: 0, 
            samples_played: 0, 
            sample_rate: 1 }
    }
}

pub struct StateObserver<F> 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static,
{
    on_change: F,
    player_state: Arc<RwLock<PlayerState>>,
}

impl<F> StateObserver<F> 
where 
    F: Fn(&PlayerState) + Send + Sync + 'static,
{
    fn new(callback: F, player_state: Arc<RwLock<PlayerState>>) -> StateObserver<F> {
        StateObserver {
            on_change: callback,
            player_state,
        }
    }
    pub fn notify(&self) {
        if let Ok(state) = self.player_state.read() {
            (self.on_change)(&state);
        }
    }
}

#[derive(Debug)]
enum ActionFollowUp {
    ProjectDataUpdate,
    PlayerStateUpdate,
    Continue,
    Exit,
}

const ALWAYS_PLAY_FROM_START: bool = false;
const PREVIEW_DURATION_MS_ONE_BEAT: u32 = 500;
const ENGINE_FILL_TIMEOUT_MS: u64 = 5;


pub fn start<F>(observer_callback: F, project_ref: &ProjectData) -> (EngineController, Arc<RwLock<PlayerState>>)
where
    F: Fn(&PlayerState) + Send + Sync + 'static,
{
    let (tx, rx) = mpsc::channel::<actions::Actions>();
    let (data_change_sender, data_change_receiver) = flume::unbounded();
    let player_state = Arc::new(RwLock::new(PlayerState::new()));
    let midi_senders: MidiSendersMap = Arc::new(RwLock::new(HashMap::new()));

    let observer = StateObserver::new(observer_callback, Arc::clone(&player_state));

    thread::spawn({
        info!("Starting main engine thread");
        let tx = tx.clone();
        let player_state = player_state.clone();
        let midi_senders = Arc::clone(&midi_senders);
        let mut project = project_ref.clone();
        move || {
            let (audio, stereo_output) = match audio::init_audio(&tx.clone()) {
                Ok((audio, stereo_output)) => {
                    if let Ok(mut state) = player_state.write() {
                        state.is_audio_initialized = true;
                    }
                    (audio, stereo_output)
                }
                Err(e) => {
                    error!("Failed to initialize audio: {}. Continuing without audio output.", e);
                    let dummy_audio = audio::AudioEngine::dummy(44100);
                    let (_consumer, dummy_stereo_output) = audio::controllers::stereo_output::StereoOutputController::new();
                    if let Ok(mut state) = player_state.write() {
                        state.sample_rate = 44100;
                        state.is_audio_initialized = false;
                    }
                    (dummy_audio, dummy_stereo_output)
                }
            };
            let mut audio_sources = AudioSources::new(audio, stereo_output, &project.tracks, Arc::clone(&midi_senders));
            let preview_tx = preview::spawn_preview_thread(Arc::clone(&midi_senders));
            let fill_timeout = Duration::from_millis(ENGINE_FILL_TIMEOUT_MS);
            loop {
                let received = match rx.recv_timeout(fill_timeout) {
                    Ok(a) => a,
                    Err(RecvTimeoutError::Timeout) => {
                        let _ = preview_tx.send(PreviewMessage::Clock);
                        if audio_sources.has_buffer_capacity() {
                            if let Ok(state) = player_state.read() {
                                if state.is_playing {
                                    audio_sources.on_tick(state.playhead);
                                } else {
                                    audio_sources.fill_buffer();
                                }
                            }
                        }
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                };
            let follow_up = match received {
                actions::Actions::Play => {
                    info!("Playing audio");
                    if let Ok(mut state) = player_state.write() {
                        if ALWAYS_PLAY_FROM_START && !state.is_playing {
                            state.playhead = 0;
                            state.samples_played = 0;
                        } else {
                            state.samples_played = (state.playhead  *  state.sample_rate / project.ticks_per_second()) as usize;
                                // info!("Initialize playhead to {} ({} samples)", state.playhead, state.samples_played);
                        }
                        state.is_playing = true;
                    }
                    ActionFollowUp::Continue
                },
                actions::Actions::Pause => {
                    if let Ok(mut state) = player_state.write()
                        && state.is_active {
                            state.is_playing = false;
                    }
                    ActionFollowUp::PlayerStateUpdate
                }
                actions::Actions::Quit => {
                    if let Ok(mut state) = player_state.write() {
                        state.is_active = false;
                    }
                    ActionFollowUp::Exit
                },
                // Project
                actions::Actions::NewFile => {
                    project.reset();
                    ActionFollowUp::ProjectDataUpdate
                },    
                // Track
                actions::Actions::AddTrack => {
                    let new_track_id = project.new_track();
                    let new_track: &Track = project.get_track_by_id(&new_track_id);
                    if let Err(e) = audio_sources.add_track(new_track) {
                        error!("FATAL: Unexpected error adding track: {}", e);
                        ActionFollowUp::Exit
                    } else {
                        ActionFollowUp::ProjectDataUpdate
                    }
                },
                actions::Actions::AddRegionAt(track_id, tick, region_type) => {
                    let track = &mut project.tracks[track_id.track_id];
                    let _ = match region_type {
                        RegionType::Pattern => track.add_pattern_at(tick),
                        RegionType::Midi => track.add_midi_region_at(tick),
                    };
                    if let Err(e) = audio_sources.update_track(track) {
                        error!("FATAL: Unexpected error adding region: {}", e);
                        ActionFollowUp::Exit
                    } else {
                        ActionFollowUp::ProjectDataUpdate
                    }
                },
                actions::Actions::MoveRegion(region_id, target_track_id, new_tick) => {
                    let src_idx = region_id.track_id.track_id;
                    let tgt_idx = target_track_id.track_id;
                    match project.tracks[src_idx].remove_region(&region_id) {
                        Some(sequence) => {
                            let target_track = &mut project.tracks[tgt_idx];
                            match target_track.insert_region(new_tick, sequence) {
                                Ok(()) => {
                                    if let Err(e) = audio_sources.update_track(&mut project.tracks[src_idx]) {
                                        error!("FATAL: Unexpected error updating source track: {}", e);
                                        ActionFollowUp::Exit
                                    } else if src_idx != tgt_idx
                                        && audio_sources.update_track(&mut project.tracks[tgt_idx]).is_err()
                                    {
                                        error!("FATAL: Unexpected error updating target track");
                                        ActionFollowUp::Exit
                                    } else {
                                        ActionFollowUp::ProjectDataUpdate
                                    }
                                }
                                Err((_, sequence)) => {
                                    if project.tracks[src_idx].insert_region(region_id.region_id, sequence).is_err() {
                                        error!("FATAL: Failed to restore region after move collision");
                                        ActionFollowUp::Exit
                                    } else if let Err(e) = audio_sources.update_track(&mut project.tracks[src_idx]) {
                                        error!("FATAL: Unexpected error restoring track: {}", e);
                                        ActionFollowUp::Exit
                                    } else {
                                        ActionFollowUp::Continue
                                    }
                                }
                            }
                        }
                        None => {
                            error!("MoveRegion: region not found");
                            ActionFollowUp::Continue
                        }
                    }
                },
                actions::Actions::DeleteRegion(region_id) => {
                    let track = &mut project.tracks[region_id.track_id.track_id];
                    track.delete_pattern(&region_id);
                    if let Err(e) = audio_sources.update_track(track) {
                        error!("FATAL: Unexpected error adding region: {}", e);
                        ActionFollowUp::Exit
                    } else {
                        ActionFollowUp::ProjectDataUpdate
                    }
                },
                // Pattern
                actions::Actions::PatternClickNote(note_identifier) => {
                    let track = project.get_track_by_id(&note_identifier.region_id.track_id);
                    track
                        .get_pattern_by_id(&note_identifier.region_id)
                        .toggle_on(note_identifier.beat_num, note_identifier.note_num);
                    if let Err(e) = audio_sources.update_track(track) {
                        error!("FATAL: Unexpected error adding region: {}", e);
                        ActionFollowUp::Exit
                    } else {
                        const DEFAULT_NOTE_VALUES: [u8; 8] = [72, 71, 69, 67, 65, 64, 62, 60];
                        let key = DEFAULT_NOTE_VALUES
                            .get(note_identifier.note_num as usize)
                            .copied()
                            .unwrap_or(60);
                        let note = MidiNote {
                            channel: 0,
                            key,
                            velocity: 100,
                            length: 0,
                        };
                        println!("Previewing note: {:?}", note);
                        let _ = preview_tx.send(PreviewMessage::Request((
                            note_identifier.region_id.track_id,
                            note,
                            PREVIEW_DURATION_MS_ONE_BEAT,
                        )));
                        ActionFollowUp::ProjectDataUpdate
                    }
                },
                actions::Actions::PreviewMidiNote(track_id, note) => {
                    let _ = preview_tx.send(PreviewMessage::Request((
                        track_id,
                        note,
                        PREVIEW_DURATION_MS_ONE_BEAT,
                    )));
                    ActionFollowUp::Continue
                },
                actions::Actions::PreviewPatternNote(track_id, note_num, _beat_num) => {
                    const DEFAULT_NOTE_VALUES: [u8; 8] = [72, 71, 69, 67, 65, 64, 62, 60];
                    let key = DEFAULT_NOTE_VALUES
                        .get(note_num as usize)
                        .copied()
                        .unwrap_or(60);
                    let note = MidiNote {
                        channel: 0,
                        key,
                        velocity: 100,
                        length: 0,
                    };
                    let _ = preview_tx.send(PreviewMessage::Request((
                        track_id,
                        note,
                        PREVIEW_DURATION_MS_ONE_BEAT,
                    )));
                    ActionFollowUp::Continue
                },
                actions::Actions::CreateMidiNote(region_identifier, start, note) => {
                    let track = &mut project.tracks[region_identifier.track_id.track_id];
                    let region = track.get_midi_by_id(&region_identifier);
                    region.add_note(start, note);
                    if let Err(e) = audio_sources.update_track(track) {
                        error!("FATAL: Unexpected error adding region: {}", e);
                        ActionFollowUp::Exit
                    } else {
                        ActionFollowUp::ProjectDataUpdate
                    }
                },
                actions::Actions::DeleteMidiNote(region_identifier, start_tick, note_index) => {
                    let track = &mut project.tracks[region_identifier.track_id.track_id];
                    let region = track.get_midi_by_id(&region_identifier);
                    let _ = region.remove_note(start_tick, note_index);
                    if let Err(e) = audio_sources.update_track(track) {
                        error!("FATAL: Unexpected error updating track: {}", e);
                        ActionFollowUp::Exit
                    } else {
                        ActionFollowUp::ProjectDataUpdate
                    }
                },
                actions::Actions::DeleteMultipleMidiNotes(region_identifier, notes_to_delete) => {
                    let track = &mut project.tracks[region_identifier.track_id.track_id];
                    let region = track.get_midi_by_id(&region_identifier);
                    
                    // Sort by tick descending, then by note_index descending
                    // This ensures we delete from highest index to lowest, avoiding index shifting
                    let mut sorted_notes = notes_to_delete;
                    sorted_notes.sort_by(|(tick_a, idx_a), (tick_b, idx_b)| {
                        match tick_b.cmp(tick_a) {
                            std::cmp::Ordering::Equal => idx_b.cmp(idx_a),
                            other => other,
                        }
                    });
                    
                    // Delete all notes
                    for (start_tick, note_index) in sorted_notes {
                        let _ = region.remove_note(start_tick, note_index);
                    }
                    
                    if let Err(e) = audio_sources.update_track(track) {
                        error!("FATAL: Unexpected error updating track: {}", e);
                        ActionFollowUp::Exit
                    } else {
                        ActionFollowUp::ProjectDataUpdate
                    }
                },
                actions::Actions::UpdateMidiNote(region_identifier, old_start_tick, note_index, new_start_tick, updated_note) => {
                    let track = &mut project.tracks[region_identifier.track_id.track_id];
                    let region = track.get_midi_by_id(&region_identifier);
                    // Remove note from old position
                    if let Some(_removed_note) = region.remove_note(old_start_tick, note_index) {
                        // Add note at new position
                        region.add_note(new_start_tick, updated_note);
                    }
                    if let Err(e) = audio_sources.update_track(track) {
                        error!("FATAL: Unexpected error updating track: {}", e);
                        ActionFollowUp::Exit
                    } else {
                        ActionFollowUp::ProjectDataUpdate
                    }
                },    
                actions::Actions::Synth(action) => {
                    if let Err(e) = audio_sources.handle_synth_action(action.clone()) {
                        error!("FATAL: Unexpected error forwarding action to instrument: {}", e);
                        ActionFollowUp::Exit
                    } else {
                        match action {
                            SynthActions::SetSoundFont(track_id, soundfont_path) => {
                                if let Some(path) = soundfont_path {
                                        let instrument = &mut project.tracks[track_id.track_id].instrument.kind;
                                        let Instrument::Synth(synth) = instrument;
                                        synth.soundfont = path.file_name().map(|x| { x.to_str() }).expect("File picker should return valid string").unwrap().to_string();
                                        ActionFollowUp::ProjectDataUpdate
                                } else {
                                    ActionFollowUp::Continue
                                }
                            }
                            SynthActions::SetBank(track_id, bank) => {
                                let instrument = &mut project.tracks[track_id.track_id].instrument.kind;
                                let Instrument::Synth(synth) = instrument;
                                synth.bank = bank;
                                ActionFollowUp::ProjectDataUpdate
                            },
                            SynthActions::SetProgram(track_id, program) => {
                                let instrument = &mut project.tracks[track_id.track_id].instrument.kind;
                                let Instrument::Synth(synth) = instrument;
                                synth.program = program;
                                ActionFollowUp::ProjectDataUpdate
                            }
                        }
                    }
                },           
                actions::Actions::Internal(sys_ev) => {
                    match sys_ev {
                        actions::SystemActions::SamplesPlayed(num_samples) => {
                            // debug!("Samples played");
                            if let Ok(mut state) = player_state.write() 
                                && state.is_playing {
                                state.samples_played += num_samples;
                                // Convert to playhead location
                                let new_playhead = project.ticks_per_second() * state.samples_played as u32 / state.sample_rate;
                                if new_playhead != state.playhead {
                                    // info!("Playhead moved to {new_playhead}  ({} samples)", state.samples_played);
                                    for tick in state.playhead..new_playhead {
                                        audio_sources.on_tick(tick);
                                    }
                                    state.playhead = new_playhead;
                                    
                                    // Check if playback should automatically stop (last note off processed)
                                    if audio_sources.should_stop_playback(new_playhead) {
                                        info!("Playback finished - last note off processed, stopping playback");
                                        state.is_playing = false;
                                    }
                                }
                            }
                            ActionFollowUp::PlayerStateUpdate                             
                        }
                        actions::SystemActions::SetSampleRate(sample_rate) => {
                            if let Ok(mut state) = player_state.write() {
                                state.sample_rate = sample_rate;
                            } 
                            ActionFollowUp::Continue                               
                        }
                    }
                }
            };
            match follow_up {
                ActionFollowUp::ProjectDataUpdate => {
                    if data_change_sender.send(project.clone()).is_err() {
                        error!("Couldn't update ui. Quitting");
                        break;
                    }
                },
                ActionFollowUp::PlayerStateUpdate => {observer.notify();},
                ActionFollowUp::Exit => break,
                ActionFollowUp::Continue => {},
            };
       }
       info!("Exiting loop. Assuming Quit was pressed");
    }});

    
    (EngineController {tx, data_change_receiver}, player_state.clone())
}
