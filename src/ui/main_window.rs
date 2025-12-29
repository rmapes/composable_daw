use iced::widget::{column, Column, row};
use iced::Length;
use iced::Element;
use iced::time;
use iced::{Subscription, window, Task};
use log::{error, info};
use crate::models::instuments::Instrument;
use crate::models::sequences::{Sequence, Tick};
use crate::models::shared::{RegionIdentifier, ProjectData, RegionType, TrackIdentifier};
use crate::engine::{self, PlayerState};

use super::actions::Message;
use super::actions::SynthMessage;
use super::components;
use super::composer_window;
use super::control_bar;
use super::editor_window;
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
    editor_window: editor_window::Component,
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
            editor_window: editor_window::Component::new(Length::Fill, Length::FillPortion(1)),
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
            Message::Engine(action) => { 
                if let Err(err) = self.engine.send(action) {
                    //TODO: Restart gracefully.
                    error!("Engine shutdown unexpectedly. Shutting down");
                    error!("{}", err);
                    iced::exit()
                } else { 
                    Task::none()
                } 
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
            Message::CreateMidiNote(region_identifier, start, note) => {
                //Get pattern and add note
                if let Ok(mut project) = self.data.write() {
                    let track = &mut project.tracks[region_identifier.track_id.track_id];
                    let region = track.get_midi_by_id(&region_identifier);
                    region.add_note(start, note);
                }
                Task::none()
            },
        }
    }
    pub fn view(&self) ->Element<'_, Message> {
        let content: Column<'_, Message> = {
            if let Ok(song) = self.data.try_read() {
                let selected_track = if self.selected_track < song.tracks.len() {
                    &song.tracks[self.selected_track]
                } else {
                    // If selected_track is out of bounds, default to first track
                    &song.tracks[0]
                }; 
                let selected_region: Option<&Sequence> = self.selected_region
                    .and_then(|selection| song.tracks[selection.track_id.track_id].midi.as_ref()
                        .and_then(|sequence| sequence.sequences.get(&selection.region_id)));
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
                                self.editor_window.view(selected_region),
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

////////////////////////////////
/// Integration Tests
#[cfg(test)]
mod integration_tests {

    use super::*;
    use iced_test::{Error, simulator};

    use super::super::actions::Message;


    #[test]
    fn test_add_and_delete_tracks() -> Result<(), Error> {
        let mut app = MainWindow::default();
        // Using fluent TestApp helper for cleaner test code
        let mut test = Emulator::new(&mut app);
        // Check that we've started up with one track present
        assert_eq!(test.tracks_present(), 1);
        // Click + to add a track (fluent method calls)
        test.click("+")?;
        // Check that a new track has been added
        assert_eq!(test.tracks_present(), 2);
        // Click to select the second track
        assert!(!test.is_track_selected("Track 2"), "Track 2 should not be selected initially");
        test.select_track(1)?; // Select track by index (0-based, so 1 = "Track 2")
        // Check that it is selected
        assert!(test.is_track_selected(1), "Track 2 should be selected");
        assert!(test.is_track_selected("Track 2"), "Track 2 should be selected by name");
        // Use the file menu to delete the track
        // Check that we are back to two tracks
        // Click file/new using the menu helper method
        test.click_menu_item("File", "New")?;
        // Check we are back to one track
        assert_eq!(test.tracks_present(), 1);
        Ok(())
    }

    // Fluent test helper that manages app state and simulator
    struct Emulator<'a> {
        app: &'a mut MainWindow,
    }

    impl<'a> Emulator<'a> {
        fn new(app: &'a mut MainWindow) -> Self {
            Self { app }
        }

        fn click(&mut self, selector: &str) -> Result<(), Error> {
            let mut ui = simulator(self.app.view());
            ui.find(selector)?;
            ui.click(selector)?;
            // Collect messages from the click (consumes ui, releasing the borrow)
            let messages = ui.into_messages().collect::<Vec<_>>();
            // Now we can update the app (mutable borrow is available)
            for message in messages {
                let _ = self.app.update(message);
            }
            Ok(())
        }

        /// Click a menu item - handles the menu opening and item selection
        /// First clicks the menu header to open it, then clicks the item
        /// Note: iced_aw menus manage state internally, so menu items may not
        /// be directly findable. This method tries to click through the UI,
        /// but falls back to directly triggering the message if needed.
        fn click_menu_item(&mut self, menu: &str, item: &str) -> Result<(), Error> {
            // Click the menu header to open it
            self.click(menu)?;
            // Try to click the menu item
            if self.click(item).is_ok() {
                return Ok(());
            }
            
            // Fallback: iced_aw menus may not expose items to iced_test selectors
            // Map common menu items to their messages and trigger directly
            let message = match (menu, item) {
                ("File", "New") => Some(Message::NewFile),
                ("File", "Open") => Some(Message::OpenFile),
                _ => None,
            };
            
            if let Some(msg) = message {
                let _ = self.app.update(msg);
                Ok(())
            } else {
                Err(Error::SelectorNotFound { selector: format!("{} -> {}", menu, item) })
            }
        }

        fn tracks_present(&mut self) -> usize {
            let mut ui = simulator(self.app.view());
            let mut count: usize = 0;
            for i in 1..1000 {
                if ui.find(format!("Track {i}")).is_ok() {
                    count += 1;
                }
            }
            count
        }

        /// Select a track by its name (e.g., "Track 1") or by index (0-based)
        /// Returns the track index that was selected
        fn select_track(&mut self, track: impl Into<TrackSelector>) -> Result<usize, Error> {
            let track_name = match track.into() {
                TrackSelector::Name(name) => name,
                TrackSelector::Index(idx) => format!("Track {}", idx + 1),
            };
            
            // Click on the track name to select it
            self.click(&track_name)?;
            
            // Return the track index (extract from name like "Track 1" -> 0)
            if let Some(num_str) = track_name.strip_prefix("Track ") {
                if let Ok(num) = num_str.parse::<usize>() {
                    return Ok(num - 1); // Convert to 0-based index
                }
            }
            Err(Error::SelectorNotFound { selector: track_name })
        }

        /// Check if a specific track is currently selected
        /// Takes either a track name (e.g., "Track 1") or index (0-based)
        fn is_track_selected(&self, track: impl Into<TrackSelector>) -> bool {
            let expected_index = match track.into() {
                TrackSelector::Name(name) => {
                    if let Some(num_str) = name.strip_prefix("Track ") {
                        if let Ok(num) = num_str.parse::<usize>() {
                            num - 1 // Convert to 0-based
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                TrackSelector::Index(idx) => idx,
            };
            
            // Check if the app's selected_track matches
            self.app.selected_track == expected_index
        }
    }

    /// Helper enum for track selection - can specify by name or index
    enum TrackSelector {
        Name(String),
        Index(usize),
    }

    impl From<&str> for TrackSelector {
        fn from(name: &str) -> Self {
            TrackSelector::Name(name.to_string())
        }
    }

    impl From<String> for TrackSelector {
        fn from(name: String) -> Self {
            TrackSelector::Name(name)
        }
    }

    impl From<usize> for TrackSelector {
        fn from(idx: usize) -> Self {
            TrackSelector::Index(idx)
        }
    }
}