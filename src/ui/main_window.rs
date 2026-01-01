use iced::widget::{column, Column, row};
use iced::Length;
use iced::Element;
use iced::time;
use iced::{Subscription, window, Task};
use log::{error, info};
use crate::models::sequences::{Sequence, Tick};
use crate::models::shared::{RegionIdentifier, ProjectData, TrackIdentifier};
use crate::engine::{self, PlayerState};
use crate::engine::actions::{Actions, SynthActions};

use super::actions::{Message, SynthMessage};
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
                self.send_to_engine_and_handle_errors(action) 
            }
            Message::GoToStart => {
                if let Ok(mut state) = self.player_state.try_write() {
                    state.playhead = 0;
                    self.playhead = 0;
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
            Message::DeselectAllRegions() => {
                self.selected_region = None;
                Task::none()
            },
            Message::NewFile => {
                // Once we implement save, we should ask the user if they want to save before closing the current file
                self.send_to_engine_and_handle_errors(Actions::NewFile) 
            },
            Message::OpenFile => todo!(),
            Message::SetPlayhead(tick_position) => {
                self.playhead = tick_position;
                if let Ok(mut state) = self.player_state.try_write() {
                    state.playhead = self.playhead;
                }
                Task::none()
            },
            Message::AddRegionAtPlayhead(region_type) => {
                let player_state = self.player_state.clone();
                if let Ok(state) = player_state.read() {
                    let track_id = TrackIdentifier { track_id: self.selected_track};
                    let tick = state.playhead;
                    return self.send_to_engine_and_handle_errors(
                        Actions::AddRegionAt(track_id, tick, region_type)
                    )
                };
                Task::none()
            },
            Message::DeleteSelectedRegion => {
                if let Some(region_id) =self.selected_region {
                    self.selected_region = None;
                    return self.send_to_engine_and_handle_errors(Actions::DeleteRegion(region_id))
                }
                Task::none()
            },
            Message::Tick => {
                if let Ok(state) = self.player_state.try_read() 
                    && state.is_playing {
                    self.playhead = state.playhead;
                }
                Task::none()
            },
            Message::Synth(synth_message) => match synth_message {
                SynthMessage::SelectSoundFont(track_id) => {
                    Task::perform(
                        pick_file(track_id, "./soundfonts"), 
                        |(track_id, path)| { 
                            Message::Synth(SynthMessage::SetSoundFont(track_id, path)) 
                        }
                    )
                }
                SynthMessage::SetSoundFont(track_id, path) => {
                    self.send_to_engine_and_handle_errors(Actions::Synth(SynthActions::SetSoundFont(track_id, path)))
                }
            },
        }
    }

fn send_to_engine_and_handle_errors(&mut self, action: Actions) -> Task<Message> {
        if let Err(err) = self.engine.send(action) {
            //TODO: Restart gracefully.
            error!("Engine shutdown unexpectedly. Shutting down");
            error!("{}", err);
            iced::exit()
        } else { 
            Task::none()
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

    use std::{thread, time::Duration};

    use crate::models::{sequences::MidiNote, shared::{PatternNoteIdentifier, RegionType}};

    use super::*;
    use iced::widget::Id;
    use iced_test::{Error, Selector, selector::Bounded, simulator};

    use super::super::actions::Message;


    #[test]
    fn test_add_and_delete_tracks() -> Result<(), Error> {
        let mut app = MainWindow::default();
        // Using fluent TestApp helper for cleaner test code
        let mut test = Emulator::new(&mut app);
        // Check that we've started up with one track present
        assert_eq!(test.tracks_present(), 1, "We should start with one track present");
        // Click + to add a track (fluent method calls)
        test.click("+")?;
        // Wait for engine to update
        thread::sleep(Duration::from_millis(1));
        // Check that a new track has been added
        assert_eq!(test.tracks_present(), 2, "A new track should have been added");
        // Click to select the second track
        assert!(!test.is_track_selected("Track 2"), "Track 2 should not be selected initially");
        test.select_track(1)?; // Select track by index (0-based, so 1 = "Track 2")
        // Check that it is selected
        assert!(test.is_track_selected(1), "Track 2 should be selected");
        assert!(test.is_track_selected("Track 2"), "Track 2 should be selected by name");
        // Use the file menu to delete the track
        // Check that we are back to a single tracks
        // Click file/new using the menu helper method
        test.click_menu_item("File", "New")?;
        thread::sleep(Duration::from_millis(1));
        // Check we are back to one track
        assert_eq!(test.tracks_present(), 1);
        Ok(())
    }

#[test]
    fn test_add_midi_region() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);

        // 1. Check that there is a region currently selected 
        // (MainWindow default selects Region 0 on Track 0)
        assert!(test.has_selection(), "A region should be selected by default");

        // 2. Check that clicking outside deselects
        // Assuming "TimelineBackground" is a valid selector for the empty area
        test.click(Id::new("TimelineBackground"))?; 
        assert!(!test.has_selection(), "Region should be deselected after clicking background");

        // 3. Check that clicking on the midi region selects it
        // Note: Default region in Track 1 usually named "Region 1" or similar
        test.click(Id::new("Region 1"))?;
        assert!(test.has_selection(), "Region should be selected after clicking it");

        // 4. Check "Edit"/"Delete Region" removes it
        test.click_menu_item("Edit", "Delete Region")?;
        assert!(!test.has_selection(), "Selection should be gone after delete");

        // 5. Check playhead is at 0
        assert_eq!(test.get_playhead(), 0);

        // 6. Add MIDI region at tick 0 via menu
        test.click_menu_item("Edit", "Add Midi Region")?;
        thread::sleep(Duration::from_millis(1));
        // Selecting it to ensure it's the active one
        test.click(Id::new("Region 1"))?; 

        // 7. Check selecting displays MIDI editor
        // We check if the editor window is viewing a sequence (non-None)
        assert!(test.is_midi_editor_visible(), "Editor should be visible for MIDI region");

        // 8. Click grid to add note (at tick 10, pitch 60)
        test.click_midi_editor_grid(10, 60)?;
        
        Ok(())
    }

    #[test]
    fn test_add_pattern_region() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);

        // 1. Check that there is a region currently selected 
        // (MainWindow default selects Region 0 on Track 0)
        assert!(test.has_selection(), "A region should be selected by default");

        // 2. Check "Edit"/"Delete Region" removes it
        test.click_menu_item("Edit", "Delete Region")?;
        assert!(!test.has_selection(), "Selection should be gone after delete");

        // 3. Check playhead is at 0
        assert_eq!(test.get_playhead(), 0);

        // 4. Add MIDI region at tick 0 via menu
        test.click_menu_item("Edit", "Add Pattern Region")?;
        thread::sleep(Duration::from_millis(1));
        // Selecting it to ensure it's the active one
        test.click(Id::new("Region 1"))?; 

        // 5. Check selecting displays MIDI editor
        // We check if the editor window is viewing a sequence (non-None)
        assert!(test.is_pattern_editor_visible(), "Editor should be visible for Pattern region");

        // 6. Click grid to add note (at tick 10, pitch 60)
        test.click_pattern_editor_grid(3, 2)?;
        
        Ok(())
    }

    // --- Expanded Emulator ---

    struct Emulator<'a> {
        app: &'a mut MainWindow,
    }

    impl<'a> Emulator<'a> {
        fn new(app: &'a mut MainWindow) -> Self {
            Self { app }
        }

        // Select an element, and click on it directly
        fn click<S>(&mut self, selector: S) -> Result<(), Error> 
        where S:Selector + Send + Clone, S::Output: Bounded + Clone + Send + Sync + 'static {
            let mut ui = simulator(self.app.view());
            ui.find(selector.clone())?;
            ui.click(selector.clone())?;
            let messages = ui.into_messages().collect::<Vec<_>>();
            for message in messages {
                let _ = self.app.update(message);
            }
            Ok(())
        }

        fn click_menu_item(&mut self, menu: &str, item: &str) -> Result<(), Error> {
            self.click(menu)?;
            if self.click(item).is_ok() {
                return Ok(());
            }
            
            // Expanded fallback mapping
            let message = match (menu, item) {
                ("File", "New") => Some(Message::NewFile),
                ("File", "Open") => Some(Message::OpenFile),
                ("Edit", "Delete Region") => Some(Message::DeleteSelectedRegion),
                ("Edit", "Add Midi Region") => {
                    // Simulate AddRegionAtPlayhead, as we can't chain tasks.
                    let track_id = TrackIdentifier { track_id: self.app.selected_track};
                    let tick = self.app.playhead;
                    Some(Message::Engine(Actions::AddRegionAt(track_id, tick, RegionType::Midi)))
                },
                ("Edit", "Add Pattern Region") => {
                    // Simulate AddRegionAtPlayhead, as we can't chain tasks.
                    let track_id = TrackIdentifier { track_id: self.app.selected_track};
                    let tick = self.app.playhead;
                    Some(Message::Engine(Actions::AddRegionAt(track_id, tick, RegionType::Pattern)))
                },
                _ => None,
            };
            
            if let Some(msg) = message {
                let task = self.app.update(msg);
                assert!(task.units()==0, "We can't handle chained tasks, so only send messages that result in task none");
                Ok(())
            } else {
                Err(Error::SelectorNotFound { selector: format!("{} -> {}", menu, item) })
            }
        }

        fn tracks_present(&mut self) -> usize {
            let mut ui = simulator(self.app.view());
            let mut count: usize = 0;
            for i in 1..1000 {
                if ui.find(format!("Track {i}")).is_ok() { count += 1; }
                else { break; }
            }
            count
        }

        fn select_track(&mut self, track: impl Into<TrackSelector>) -> Result<usize, Error> {
            let selector = track.into();
            let track_name = match &selector {
                TrackSelector::Name(name) => name.clone(),
                TrackSelector::Index(idx) => format!("Track {}", idx + 1),
            };
            self.click(track_name)?;
            match selector {
                TrackSelector::Index(idx) => Ok(idx),
                TrackSelector::Name(_) => Ok(self.app.selected_track)
            }
        }

        fn is_track_selected(&self, track: impl Into<TrackSelector>) -> bool {
            match track.into() {
                TrackSelector::Index(idx) => self.app.selected_track == idx,
                TrackSelector::Name(name) => format!("Track {}", self.app.selected_track + 1) == name,
            }
        }

        // --- New Helper Methods ---

        fn has_selection(&self) -> bool {
            self.app.selected_region.is_some()
        }

        fn get_playhead(&self) -> Tick {
            self.app.playhead
        }

        fn is_midi_editor_visible(&self) -> bool {
            let mut ui = simulator(self.app.view());
            ui.find(Id::new("MidiEditor")).is_ok()
        }

        fn is_pattern_editor_visible(&self) -> bool {
            let mut ui = simulator(self.app.view());
            ui.find(Id::new("PatternEditor")).is_ok()
        }

        fn click_midi_editor_grid(&mut self, tick: Tick, note: u8) -> Result<(), Error> {
            if let Some(region_id) = self.app.selected_region {
                let midi_note = MidiNote { key: note, velocity: 100, channel: 0, length: 960};
                let msg = Message::Engine(Actions::CreateMidiNote(region_id, tick, midi_note));
                let _ = self.app.update(msg);
                Ok(())
            } else {
                Err(Error::SelectorNotFound { selector: "Editor Grid (No Region Selected)".to_string() })
            }
        }

        fn click_pattern_editor_grid(&mut self, beat: u8, note: u8) -> Result<(), Error> {
            if let Some(region_id) = self.app.selected_region {
                let pattern_note = PatternNoteIdentifier { 
                    region_id,
                    note_num: note,
                    beat_num: beat,
                };
                let msg = Message::Engine(Actions::PatternClickNote(pattern_note));
                let _ = self.app.update(msg);
                Ok(())
            } else {
                Err(Error::SelectorNotFound { selector: "Editor Grid (No Region Selected)".to_string() })
            }
        }
    }

    enum TrackSelector { Name(String), Index(usize) }
    impl From<&str> for TrackSelector { fn from(name: &str) -> Self { TrackSelector::Name(name.to_string()) } }
    impl From<usize> for TrackSelector { fn from(idx: usize) -> Self { TrackSelector::Index(idx) } }
}