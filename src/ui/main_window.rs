use iced::widget::{column, Column, row};
use iced::Length;
use iced::Element;
use iced::time;
use iced::{Subscription, window, Task};
use log::info;
use crate::models::instuments::Instrument;
use crate::models::sequences::{Sequence, Tick};
use crate::models::shared::{RegionIdentifier, ProjectData, RegionType, TrackIdentifier};
use crate::engine::{self, PlayerState};

use super::actions::Message;
use super::actions::SynthMessage;
use super::components;
use super::composer_window;
use super::control_bar;
use super::pattern_editor;
use super::main_menu::top_menu_view;
use super::track_settings;
use super::file_picker::pick_file;

use std::sync::{Arc, RwLock};
use std::rc::Rc;

//////////////////////
/// Entry point for iced ui
/// 

pub struct MainWindow {
    // Core application data and engine
    engine: Rc<engine::EngineController>,
    player_state: Arc<RwLock<PlayerState>>,
    data: Arc<RwLock<ProjectData>>,

    // Mutable state
    selected_track: usize,
    selected_region: Option<RegionIdentifier>,
    playhead: Tick,
    // Preferences
    width: Length,
    height: Length,

    // UI subcomponents
    control_bar: control_bar::Component,
    composer_window: composer_window::Component,
    pattern_editor: pattern_editor::Component,
    track_settings: track_settings::Component,


}

impl Default for MainWindow {
    fn default() -> Self {
        let data = Arc::new(RwLock::new(ProjectData::new()));
        let (engine, player_state) = {
            let (engine, player_state) = engine::start(
            {
                move |_player_state: &engine::PlayerState| {
                    // Ignore this for the moment.
                    // Eventually, I'll need to work out how to handle internal state updates
                }
            },
            Arc::clone(&data),);
            (Rc::new(engine), player_state)
        };
        let selected_track = TrackIdentifier{ track_id: 0 };
    
        Self {
            engine,
            player_state,
            data,
            selected_track: selected_track.track_id,
            selected_region: Some(RegionIdentifier { track_id: selected_track, region_id: 0 }), // Temporary: select pattern by default. Relies on track beging created with initial pattern
            playhead: 0,
            width: Length::Fill, //600_f32,
            height: Length::Fill, //400_f32,
            control_bar: control_bar::Component::new(Length::Fill, Length::Fixed(50_f32)),
            composer_window: composer_window::Component::new(Length::Fill, Length::FillPortion(2)),
            pattern_editor: pattern_editor::Component::new(Length::Fill, Length::FillPortion(1)),
            track_settings: track_settings::Component::new(Length::Fixed(100_f32),Length::Fill),            
        }
    }
}

impl MainWindow {
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::WindowEvent(event) => match event {
                window::Event::CloseRequested => {
                    self.shutdown();
                    iced::exit()
                }
                _ => Task::none(),
            }
            Message::PatternClickNote(note_identifier) => {
                // toggle note on in pattern
                if let Ok(mut song) = self.data.try_write() {
                    song.get_track_by_id(&note_identifier.region_id.track_id)
                    .get_pattern_by_id(&note_identifier.region_id)
                    .toggle_on(note_identifier.beat_num, note_identifier.note_num);
                }               
                // No further task to do
                Task::none()
            },
            Message::Play => {
                if let Ok(mut state) = self.player_state.try_write() 
                    && state.is_audio_initialized {
                        state.is_playing = true;
                        return Task::none()
                }
                self.engine.play_midi();
                Task::none()
            },
            Message::Stop => {
                // self.engine.pause();
                if let Ok(mut state) = self.player_state.try_write() 
                    && state.is_audio_initialized {
                        state.is_playing = false;
                }
                Task::none()
            }
            Message::GoToStart => {
                if let Ok(mut state) = self.player_state.try_write() {
                    state.playhead = 0;
                    self.playhead = 0;
                }
                Task::none()
            },
            Message::AddTrack => {
                if let Ok(mut song) = self.data.try_write() {
                    song.new_track();
                }               
                Task::none()
            },
            Message::SelectTrack(id) => {
                self.selected_track = id.track_id;
                Task::none()
            }
            Message::SelectRegion(id, _is_multi_select) => {
                self.selected_region = Some(id);
                Task::none()
            }
            Message::Synth(synth_message) => match synth_message {
                SynthMessage::SelectSoundFont(track_id) => {
                    Task::perform(
                        pick_file(track_id, "./soundfonts"), 
                        |(track_id, path)| { Message::Synth(SynthMessage::SetSoundFont(track_id, path)) } 
                    ) 
                }
                SynthMessage::SetSoundFont(track_id, soundfont_path) => {
                    if let Some(path) = soundfont_path  
                        && let Ok(mut project) = self.data.write() {
                            let instrument = &mut project.tracks[track_id.track_id].instrument.kind;
                            let Instrument::Synth(synth) = instrument;
                            synth.soundfont = path.file_name().map(|x| { x.to_str() }).expect("File picker should return valid string").unwrap().to_string();
                    }
                        Task::none()
                }
                SynthMessage::SetBank(track_id, bank) => {
                    if let Ok(mut project) = self.data.write() {
                        let instrument = &mut project.tracks[track_id.track_id].instrument.kind;
                        let Instrument::Synth(synth) = instrument;
                        synth.bank = bank;
                    }
                    Task::none()
                },
                SynthMessage::SetProgram(track_id, program) => {
                    if let Ok(mut project) = self.data.write() {
                        let instrument = &mut project.tracks[track_id.track_id].instrument.kind;
                        let Instrument::Synth(synth) = instrument;
                            synth.program = program;
                        }
                    Task::none()
                }
            },
            Message::AddRegionAtPlayhead(region_type) => {
                let player_state = self.player_state.clone();
                let selected_track = self.selected_track;
                Task::perform(async move {
                    if let Ok(state) = player_state.read() {
                        let track_id = TrackIdentifier { track_id: selected_track};
                        let tick = state.playhead;
                        return (Some(track_id), tick, region_type)
                    }
                    ( None, 0, RegionType::Pattern)
                }, |(maybe_track_id, tick, region_type)| { 
                    if let Some(track_id) = maybe_track_id {
                        Message::AddRegionAt(track_id, tick, region_type) 
                    } else {
                        Message::Ignore
                    }
                })
            },
            Message::AddRegionAt(track_id, tick, region_type) => {
                if let Ok(mut project) = self.data.write() {
                    let track = &mut project.tracks[track_id.track_id];
                    let _ = match region_type {
                        RegionType::Pattern => track.add_pattern_at(tick),
                        RegionType::Midi => track.add_midi_region_at(tick),
                    };
                }
                Task::none()
            },
            Message::DeselectAllRegions() => {
                self.selected_region = None;
                Task::none()
            },
            Message::DeleteSelectedRegion() => {
                if let Some(pattern) =self.selected_region {
                if let Ok(mut project) = self.data.write() {
                    let track = &mut project.tracks[pattern.track_id.track_id];
                    track.delete_pattern(&pattern);
                }

                self.selected_region = None;
                }
                Task::none()
            },
            Message::NewFile => {
                // Once we implement save, we should ask the user if they want to save before closing the current file
                if let Ok(mut project) = self.data.write() {
                    project.reset();
                }
                Task::none()
            },
            Message::OpenFile => todo!(),
            Message::SetPlayhead(tick_position) => {
                self.playhead = tick_position;
                if let Ok(mut state) = self.player_state.try_write() {
                    state.playhead = self.playhead;
                }
                Task::none()
            },
            Message::Tick => {
                if let Ok(state) = self.player_state.try_read() 
                    && state.is_playing {
                    self.playhead = state.playhead;
                    // println!("Tick: {}", state.playhead);
                }
                Task::none()
            },
            Message::Ignore => Task::none(),
        }
    }
    pub fn view(&self) ->Element<'_, Message> {
        let content: Column<'_, Message> = {
            if let Ok(song) = self.data.try_read() {
                let selected_track =  &song.tracks[self.selected_track]; 
                let selected_region: Option<&Sequence> = self.selected_region
                    .and_then(|selection| song.tracks[selection.track_id.track_id].midi.as_ref()
                        .and_then(|sequence| sequence.sequences.get(&selection.region_id)));
                let selected_pattern = match selected_region {
                    Some(Sequence::Pattern(p)) => Some(p),
                    _ => None, // Fix this when we support other region types
                };
                column![
                    top_menu_view(),
                    self.control_bar.view(),
                    // Replace the following row and column layout with https://github.com/iced-rs/iced/blob/master/examples/pane_grid/README.md
                    row![
                        components::module_slot(
                            self.track_settings.view(selected_track)
                        ).width(Length::Shrink), // Shrink to fit channel strips
                        column![
                            components::module_slot(
                                self.composer_window.view(&song.tracks, self.selected_track, song.ppq, self.playhead),
                            ),
                            components::module_slot(
                                self.pattern_editor.view(selected_pattern),
                            )
                        ]
                    ]
                ].width(self.width).height(self.height)
                } else {
                column![] // TODO: store a local copy of the song data to deal with try_lock failing
            }
        };
        components::rack(content.into()).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // 1. Subscription for Window Events
        let window_events = iced::window::events().map(|(_id, event)| Message::WindowEvent(event));
    
        // 2. Subscription for the Millisecond Tick
        // Every 1 millisecond, send a Message::Tick
        let tick = time::every(time::Duration::from_millis(1)).map(|_| Message::Tick);
    
        // 3. Combine both subscriptions
        Subscription::batch(vec![window_events, tick])
    }
    
    // Don't forget to stop engine on shutdown
    fn shutdown(&self) {
        info!("Shutting down");
        self.engine.quit();
    }
}
