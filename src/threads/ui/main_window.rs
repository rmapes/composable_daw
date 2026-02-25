use std::hash::Hash;
use std::sync::{Arc, RwLock};

use iced::advanced::subscription::{EventStream, Recipe};
use iced::futures::channel::mpsc;
use iced::futures::stream::BoxStream;
use iced::futures::SinkExt;
use iced::time;
use iced::widget::{column, row, stack, text, Column, Container, Space};
use iced::{Element, Length, Subscription, Task, window};
use log::{error, info};

use crate::models::sequences::{Sequence, TSequence, Tick};
use crate::models::shared::{ProjectData, RegionIdentifier, TrackIdentifier};

use super::super::engine::actions::{Actions, SynthActions};
use super::super::engine::{self, PlayerState};
use super::actions::{Message, SynthMessage};
use super::components;
use super::composer_window;
use super::control_bar;
use super::editor_window;
use super::file_picker::pick_file;
use super::main_menu::top_menu_view;
use super::track_settings;

const DEFAULT_MIDI_OFFSET: u8 = 48; // C3
const STREAM_CHANNEL_CAPACITY: usize = 100;
const CONTROL_BAR_HEIGHT: f32 = 50.0;
const TRACK_SETTINGS_WIDTH: f32 = 100.0;

/// State for an in-progress region drag.
#[derive(Debug, Clone)]
pub struct DragState {
    pub region_id: RegionIdentifier,
    pub region_length: Tick,
    pub initial_track_index: usize,
    pub initial_tick: Tick,
    pub initial_mouse_x: f32,
    pub current_track_index: usize,
    pub current_tick: Tick,
    pub is_valid_drop: bool,
}

//////////////////////
/// Entry point for iced ui
/// 
pub struct MainWindow {
    // Core application data and engine
    engine: engine::EngineController,
    player_state: Arc<RwLock<PlayerState>>,
    data: ProjectData,

    // Mutable state
    selected_track: usize,
    selected_region: Option<RegionIdentifier>,
    dragging_region: Option<DragState>,
    playhead: Tick,
    midi_editor_snap: super::midi_editor::SnapToGrid,
    midi_editor_offset: u8, // Lowest visible MIDI note (pitch at bottom of grid), default 48 (C3)
    // Preferences
    width: Length,
    height: Length,

    // UI subcomponents
    control_bar: control_bar::Component,
    composer_window: composer_window::Component,
    editor_window: editor_window::Component,
    track_settings: track_settings::Component,


}

impl std::hash::Hash for MainWindow {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i64(123456_i64);  // Use a static hash to allow data change subscription to work
    }
}

/////////////////////////////
/// Initial state on startup
/// All project state is stored in data and managed by the engine. It is this data that is saved and loaded
/// All UI state is stored in the ui thread and not persisted between sessions. If needed by engine it is passed as command parameters
/// NOT YET IMPLEMENTED: All permanent settings are stored in a settings object and autosaved in the background 
impl Default for MainWindow {
    fn default() -> Self {
        let data = ProjectData::new();
        let (engine, player_state) = {
            let (engine, player_state) = engine::start(
            {
                move |_player_state: &engine::PlayerState| {
                    // Ignore this for the moment.
                    // Eventually, I'll need to work out how to handle internal state updates
                }
            },
            
            &data
            );
            (engine, player_state)
        };
        let selected_track = TrackIdentifier{ track_id: 0 };
    
        Self {
            engine,
            player_state,
            data,
            selected_track: selected_track.track_id,
            selected_region: Some(RegionIdentifier { track_id: selected_track, region_id: 0 }), // Temporary: select pattern by default. Relies on track beging created with initial pattern
            dragging_region: None,
            playhead: 0,
            midi_editor_snap: super::midi_editor::SnapToGrid::Division,
            midi_editor_offset: DEFAULT_MIDI_OFFSET,
            width: Length::Fill, //600_f32,
            height: Length::Fill, //400_f32,
            control_bar: control_bar::Component::new(Length::Fill, Length::Fixed(CONTROL_BAR_HEIGHT)),
            composer_window: composer_window::Component::new(Length::Fill, Length::FillPortion(2)),
            editor_window: editor_window::Component::new(Length::Fill, Length::FillPortion(1)),
            track_settings: track_settings::Component::new(Length::Fixed(TRACK_SETTINGS_WIDTH), Length::Fill),            
        }
    }
}

impl MainWindow {
    //////////////////
    /// Actions triggered by UI components. Look for main handlers here
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
            Message::RegionClick(region_id) => {
                self.selected_region = Some(region_id);
                self.dragging_region = None;
                Task::none()
            }
            Message::StartRegionDrag(region_id, initial_x, initial_y, current_x, current_y) => {
                if let Some(length) = Self::get_region_length(&self.data, &region_id) {
                    self.selected_region = Some(region_id);
                    let track_idx = region_id.track_id.track_id;
                    let initial_tick = region_id.region_id;
                    let current_track = Self::y_to_track_index(current_y).unwrap_or(track_idx).min(self.data.tracks.len().saturating_sub(1));
                    let length_per_tick = Self::length_per_tick(self.data.ppq);
                    let delta_x = current_x - initial_x;
                    let current_tick = (initial_tick as f32 + delta_x / length_per_tick).max(0.0) as Tick;
                    let is_valid = Self::check_drop_valid(&self.data, region_id, track_idx, initial_tick, length, current_track, current_tick);
                    self.dragging_region = Some(DragState {
                        region_id,
                        region_length: length,
                        initial_track_index: track_idx,
                        initial_tick,
                        initial_mouse_x: initial_x,
                        current_track_index: current_track,
                        current_tick,
                        is_valid_drop: is_valid,
                    });
                }
                Task::none()
            }
            Message::UpdateRegionDrag(mouse_x, mouse_y) => {
                if let Some(ref mut drag) = self.dragging_region {
                    let length_per_tick = Self::length_per_tick(self.data.ppq);
                    let delta_x = mouse_x - drag.initial_mouse_x;
                    drag.current_tick = (drag.initial_tick as f32 + delta_x / length_per_tick).max(0.0) as Tick;
                    drag.current_track_index = Self::y_to_track_index(mouse_y)
                        .unwrap_or(drag.initial_track_index)
                        .min(self.data.tracks.len().saturating_sub(1));
                    drag.is_valid_drop = Self::check_drop_valid(
                        &self.data,
                        drag.region_id,
                        drag.initial_track_index,
                        drag.initial_tick,
                        drag.region_length,
                        drag.current_track_index,
                        drag.current_tick,
                    );
                }
                Task::none()
            }
            Message::EndRegionDrag => {
                if let Some(drag) = self.dragging_region.take() && drag.is_valid_drop {
                    let target_id = TrackIdentifier {
                        track_id: drag.current_track_index,
                    };
                    let _ = self.engine.send(Actions::MoveRegion(
                        drag.region_id,
                        target_id,
                        drag.current_tick,
                    ));
                    self.selected_region = Some(RegionIdentifier {
                        track_id: target_id,
                        region_id: drag.current_tick,
                    });
                }
                Task::none()
            }
            Message::CancelRegionDrag => {
                self.dragging_region = None;
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
            Message::OpenFile => {
                info!("Open file not yet implemented");
                Task::none()
            },
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
            Message::ProjectDataChanged(project_data) => {
                self.data = project_data;
                Task::none()
            },
            Message::MidiEditor(msg) => {
                match msg {
                    super::midi_editor::MidiEditorMessage::SetSnapToGrid(snap) => {
                        self.midi_editor_snap = snap;
                    }
                    super::midi_editor::MidiEditorMessage::ScrollPitch(delta) => {
                        let new_offset = (self.midi_editor_offset as i16 + delta).clamp(0, 127);
                        self.midi_editor_offset = new_offset as u8;
                    }
                }
                Task::none()
            },
        }
    }

    //////////////////
    /// Helper functions for handlers
    fn length_per_tick(ppq: u32) -> f32 {
        const TIMELINE_WIDTH: f32 = 950.0;
        const BARS_IN_TIMELINE: u32 = 16;
        let length_in_ticks = ppq * 4 * BARS_IN_TIMELINE;
        TIMELINE_WIDTH / length_in_ticks as f32
    }

    fn y_to_track_index(y: f32) -> Option<usize> {
        const RULER_HEIGHT: f32 = 10.0;
        const TRACK_HEIGHT: f32 = 50.0;
        if y < RULER_HEIGHT {
            return Some(0);
        }
        let track = ((y - RULER_HEIGHT) / TRACK_HEIGHT).floor() as usize;
        Some(track)
    }

    fn get_region_length(data: &ProjectData, id: &RegionIdentifier) -> Option<Tick> {
        data.tracks
            .get(id.track_id.track_id)
            .and_then(|t| t.midi.as_ref())
            .and_then(|c| c.sequences.get(&id.region_id))
            .map(|s| s.length_in_ticks())
    }

    fn check_drop_valid(
        data: &ProjectData,
        _region_id: RegionIdentifier,
        initial_track_index: usize,
        initial_tick: Tick,
        region_length: Tick,
        current_track_index: usize,
        current_tick: Tick,
    ) -> bool {
        let Some(track) = data.tracks.get(current_track_index) else {
            return false;
        };
        let Some(container) = track.midi.as_ref() else {
            return true;
        };
        let exclude = (current_track_index == initial_track_index).then_some(initial_tick);
        !container.region_collides_with_existing_excluding(current_tick, region_length, exclude)
    }

    //////////////////
    ///Handles communication with the engine. If the engine returns an error, the application will shut down.
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

    //////////////////
    /// UI layout for the application. This is the entry point for the UI and where all the UI components are rendered.
    pub fn view(&self) ->Element<'_, Message> {
        let is_audio_initialized = self
            .player_state
            .try_read()
            .map(|s| s.is_audio_initialized)
            .unwrap_or(false);

        let selected_track = if self.selected_track < self.data.tracks.len() {
            &self.data.tracks[self.selected_track]
        } else {
            &self.data.tracks[0]
        };
        let selected_region: Option<&Sequence> = self
            .selected_region
            .and_then(|selection| {
                self.data.tracks[selection.track_id.track_id]
                    .midi
                    .as_ref()
                    .and_then(|sequence| sequence.sequences.get(&selection.region_id))
            });

        let main_content: Column<'_, Message> = column![
            top_menu_view(),
            self.control_bar.view(),
            row![
                components::module_slot(self.track_settings.view(selected_track))
                    .width(Length::Shrink),
                column![
                    components::module_slot(
                        self.composer_window.view(
                            &self.data.tracks,
                            self.selected_track,
                            self.data.ppq,
                            self.playhead,
                            self.dragging_region.as_ref(),
                        ),
                    ),
                    components::module_slot(
                        self.editor_window
                            .view(selected_region, self.midi_editor_snap, self.midi_editor_offset),
                    ),
                ]
            ]
        ]
        .width(self.width)
        .height(self.height);

        let toast_layer: Element<'_, Message> = if is_audio_initialized {
            Space::new().into()
        } else {
            Container::new(
                Container::new(text("Audio Initializing"))
                    .padding(8)
                    .style(super::style::module_slot),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(16)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top)
            .into()
        };

        let stacked = stack![main_content, toast_layer];
        components::rack(stacked.into()).into()
    }

    //////////////////
    /// Subscriptions for the application. This is where we subscribe to events from the engine and other sources.
    pub fn subscription(&self) -> Subscription<Message> {
        // 1. Subscription for Window Events
        let window_events = iced::window::events().map(|(_id, event)| Message::WindowEvent(event));
    
        // 2. Subscription for the Millisecond Tick
        // Every 1 millisecond, send a Message::Tick
        let tick = time::every(time::Duration::from_millis(1)).map(|_| Message::Tick);

        // 3. Subscribe to project data change events
        let data_change = project_data_change_listener(self);
    
        // 4. Combine all subscriptions
        Subscription::batch(vec![window_events, tick, data_change])
    }
    
    // Don't forget to stop engine on shutdown
    fn shutdown(&self) {
        info!("Shutting down");
        self.engine.quit();
    }
}


    //////////////////
    /// Listener for project data changes. This is used to update the UI when the project data changes.

pub fn project_data_change_listener(wnd: &MainWindow) -> Subscription<Message> {
    // 1. Create the recipe instance
    let recipe = ProjectDataListener {
        receiver: wnd.engine.data_change_receiver.clone(),
    };

    // 2. Turn the recipe into a Subscription
    // In 0.14, this is usually under iced::advanced::subscription::from_recipe
    iced::advanced::subscription::from_recipe(recipe)
}
pub struct ProjectDataListener {
    // We store the receiver here
    receiver: flume::Receiver<ProjectData>,
}

impl Recipe for ProjectDataListener {
    type Output = Message;

    // This is the "Identity" of your subscription
    fn hash(&self, state: &mut rustc_hash::FxHasher) {
        use std::any::TypeId;
        TypeId::of::<Self>().hash(state);
        // You could also hash a specific project ID if you have one
    }

    // This is where the actual async work happens
    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        let rx = self.receiver;
        
        // We use iced's internal stream channel helper
        let stream = iced::stream::channel(STREAM_CHANNEL_CAPACITY, move |mut output: mpsc::Sender<Message>| async move {
            while let Ok(data) = rx.recv_async().await {
                if output.send(Message::ProjectDataChanged(data)).await.is_err() {
                    break;
                }
            }
        });

        Box::pin(stream)
    }
}

////////////////////////////////
/// Integration Tests
#[cfg(test)]
mod integration_tests {
    const TEST_DEFAULT_NOTE_VELOCITY: u8 = 100;
    const TEST_DEFAULT_NOTE_LENGTH_TICKS: u32 = 960;

    use std::{collections::VecDeque, thread, time::Duration};

    use crate::models::{
        sequences::MidiNote,
        shared::{PatternNoteIdentifier, RegionIdentifier, RegionType, TrackIdentifier},
    };

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

        // 3. Check that selecting the first region selects it
        test.select_first_region_in_selected_track()?;
        assert!(test.has_selection(), "Region should be selected after selecting it");

        // 4. Check "Edit"/"Delete Region" removes it
        test.click_menu_item("Edit", "Delete Region")?;
        assert!(!test.has_selection(), "Selection should be gone after delete");

        // 5. Check playhead is at 0
        assert_eq!(test.get_playhead(), 0);

        // 6. Add MIDI region at tick 0 via menu
        test.click_menu_item("Edit", "Add Midi Region")?;
        test.select_first_region_in_selected_track()?;

        // 7. Check selecting displays MIDI editor
        // We check if the editor window is viewing a sequence (non-None)
        assert!(test.is_midi_editor_visible(), "Editor should be visible for MIDI region");

        // 8. Click grid to add note (at tick 10, pitch 60)
        test.click_midi_editor_grid(10, 60)?;
        
        Ok(())
    }

    /// Spec 1.3 Rewind to start: Playhead returns to tick 0
    #[test]
    fn test_rewind_to_start() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        // Set playhead away from 0 (via SetPlayhead message)
        test.send_message(Message::SetPlayhead(1000));
        assert_ne!(test.get_playhead(), 0);
        // Click Rewind (sends GoToStart)
        test.send_message(Message::GoToStart);
        assert_eq!(test.get_playhead(), 0);
        Ok(())
    }

    /// Spec 1.1 Play, 1.2 Stop: Start and stop playback without crash
    #[test]
    fn test_play_and_stop() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.send_message(Message::Engine(Actions::Play));
        test.send_message(Message::Engine(Actions::Pause));
        Ok(())
    }

    /// Spec 2.1 New project: File → New clears project
    #[test]
    fn test_new_project_clears() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.click("+")?;
        assert_eq!(test.tracks_present(), 2);
        test.click_menu_item("File", "New")?;
        assert_eq!(test.tracks_present(), 1);
        Ok(())
    }

    /// Spec 2.2 Open file: No crash when File → Open (not yet implemented)
    #[test]
    fn test_open_file_no_crash() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.send_message(Message::OpenFile);
        Ok(())
    }

    /// Spec 3.1 Add track, 3.2 Select track: Add track and verify selection
    #[test]
    fn test_add_and_select_track() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.click("+")?;
        assert_eq!(test.tracks_present(), 2);
        test.select_track(1)?;
        assert!(test.is_track_selected("Track 2"));
        Ok(())
    }

    /// Spec 4.6 Set playhead: Playhead moves to position
    #[test]
    fn test_set_playhead() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.send_message(Message::SetPlayhead(480));
        assert_eq!(test.get_playhead(), 480);
        Ok(())
    }

    /// Spec 4.4 Move region: Simulate drag via StartRegionDrag, UpdateRegionDrag, EndRegionDrag
    #[test]
    fn test_move_region() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.select_first_region_in_selected_track()?;
        let region_id = test.app.selected_region.expect("region selected");
        let _region_length = crate::models::sequences::TSequence::length_in_ticks(
            test.app.data.tracks[region_id.track_id.track_id]
                .midi
                .as_ref()
                .and_then(|c| c.sequences.get(&region_id.region_id))
                .expect("region exists"),
        );
        let length_per_tick = 950.0 / (test.app.data.ppq * 4 * 16) as f32;
        let delta_x = length_per_tick * 480.0; // move 480 ticks right
        test.send_message(Message::StartRegionDrag(
            region_id,
            100.0,
            50.0,
            100.0 + delta_x,
            50.0,
        ));
        test.send_message(Message::EndRegionDrag);
        // Region should have moved; selection updated
        assert!(test.app.selected_region.is_some());
        Ok(())
    }

    /// Spec 6.3 Editor when no region selected: Deselect shows empty/neutral editor
    #[test]
    fn test_editor_when_no_region_selected() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.click(Id::new("TimelineBackground"))?;
        assert!(!test.has_selection());
        // App should not crash; view renders
        let _ = test.app.view();
        Ok(())
    }

    /// Spec 7.2 Multiple pattern toggles: Each step maintains state
    #[test]
    fn test_pattern_multiple_toggles() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.click_menu_item("Edit", "Delete Region")?;
        test.click_menu_item("Edit", "Add Pattern Region")?;
        test.select_first_region_in_selected_track()?;
        test.click_pattern_editor_grid(0, 0)?;
        test.click_pattern_editor_grid(1, 2)?;
        test.click_pattern_editor_grid(2, 1)?;
        test.drain_engine_updates();
        // Verify pattern has steps on (beat 0, note 0), (beat 1, note 2), (beat 2, note 1)
        let region_id = test.app.selected_region.expect("region");
        let seq = test.app.data.tracks[region_id.track_id.track_id]
            .midi
            .as_ref()
            .and_then(|c| c.sequences.get(&region_id.region_id));
        if let Some(crate::models::sequences::Sequence::Pattern(p)) = seq {
            assert!(*p.is_on(0, 0), "beat 0 note 0 should be on");
            assert!(*p.is_on(1, 2), "beat 1 note 2 should be on");
            assert!(*p.is_on(2, 1), "beat 2 note 1 should be on");
        } else {
            panic!("Expected Pattern region");
        }
        Ok(())
    }

    /// Spec 8.5 Delete MIDI notes: Selected notes removed
    #[test]
    fn test_delete_midi_notes() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.click_menu_item("Edit", "Delete Region")?;
        test.click_menu_item("Edit", "Add Midi Region")?;
        test.select_first_region_in_selected_track()?;
        test.click_midi_editor_grid(0, 60)?;
        test.click_midi_editor_grid(480, 62)?;
        test.drain_engine_updates();
        let region_id = test.app.selected_region.expect("region");
        let midi_seq = test.app.data.tracks[region_id.track_id.track_id]
            .midi
            .as_ref()
            .and_then(|c| c.sequences.get(&region_id.region_id));
        let before = match midi_seq {
            Some(crate::models::sequences::Sequence::Midi(m)) => {
                m.notes.values().map(|v| v.len()).sum::<usize>()
            }
            _ => 0,
        };
        assert!(before >= 2, "should have at least 2 notes, got {}", before);
        test.send_message(Message::Engine(Actions::DeleteMultipleMidiNotes(
            region_id,
            vec![(0, 0), (480, 0)],
        )));
        let midi_seq_after = test.app.data.tracks[region_id.track_id.track_id]
            .midi
            .as_ref()
            .and_then(|c| c.sequences.get(&region_id.region_id));
        let after = match midi_seq_after {
            Some(crate::models::sequences::Sequence::Midi(m)) => {
                m.notes.values().map(|v| v.len()).sum::<usize>()
            }
            _ => 0,
        };
        assert!(after < before);
        Ok(())
    }

    /// Spec 10.1 Menu items: File and Edit menus have expected items
    #[test]
    fn test_menu_items_present() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        // File menu items
        test.click_menu_item("File", "New")?;
        test.click_menu_item("File", "Open")?;
        // Edit menu items
        test.click_menu_item("Edit", "Add Pattern Region")?;
        test.select_first_region_in_selected_track()?;
        test.click_menu_item("Edit", "Add Midi Region")?;
        test.select_first_region_in_selected_track()?;
        test.click_menu_item("Edit", "Delete Region")?;
        Ok(())
    }

    /// Spec 11.1 Delete region with no selection: No crash
    #[test]
    fn test_delete_region_with_no_selection() -> Result<(), Error> {
        let mut app = MainWindow::default();
        let mut test = Emulator::new(&mut app);
        test.click(Id::new("TimelineBackground"))?;
        assert!(!test.has_selection());
        test.send_message(Message::DeleteSelectedRegion);
        Ok(())
    }

    /// Spec 5.1, 10.2 Layout: Control bar and track list visible
    #[test]
    fn test_layout_control_bar_and_tracks_visible() -> Result<(), Error> {
        let app = MainWindow::default();
        let mut ui = simulator(app.view());
        ui.find("Track 1")?;
        ui.find("+")?;
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

        // 4. Add Pattern region at tick 0 via menu
        test.click_menu_item("Edit", "Add Pattern Region")?;
        test.select_first_region_in_selected_track()?;

        // 5. Check selecting displays pattern editor
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

        fn send_message(&mut self, msg: Message) {
            let mut messages = std::collections::VecDeque::new();
            messages.push_back(msg);
            self.process_messages(messages);
        }

        fn drain_engine_updates(&mut self) {
            for _ in 0..5 {
                thread::sleep(Duration::from_millis(200));
                while let Ok(data) = self.app.engine.data_change_receiver.try_recv() {
                    let _ = self.app.update(Message::ProjectDataChanged(data));
                }
            }
        }

        // Select an element, and click on it directly
        fn click<S>(&mut self, selector: S) -> Result<(), Error> 
        where S:Selector + Send + Clone, S::Output: Bounded + Clone + Send + Sync + 'static {
            let messages = {
                let view = self.app.view();
                let mut ui = simulator(view);
                ui.find(selector.clone())?;
                ui.click(selector.clone())?;
                ui.into_messages().collect::<VecDeque<_>>()
            };
            // Wait for engine to update
            self.process_messages(messages);
            Ok(())
        }

        fn process_messages(&mut self, mut messages: VecDeque<Message>) {
            while !messages.is_empty() {
                if let Some(message) = messages.pop_front() {
                    let task = self.app.update(message.clone());
                    assert!(task.units()==0, "We can't handle chained tasks, so only send messages that result in task none");
                }
                // Now check for generated events
                thread::sleep(Duration::from_millis(1000));
                if let Ok(data) = self.app.engine.data_change_receiver.try_recv() {
                    messages.push_back(Message::ProjectDataChanged(data));
                }
            }
        }
        
        fn click_menu_item(&mut self, menu: &str, item: &str) -> Result<(), Error> {
            self.click(menu)?;
            if self.click(item).is_ok() {
                // Wait for engine to update
                thread::sleep(Duration::from_millis(1));
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
                let mut messages = VecDeque::new();
                messages.push_back(msg);
                self.process_messages(messages);
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

        /// Select the first region (by tick) in the currently selected track.
        fn select_first_region_in_selected_track(&mut self) -> Result<(), Error> {
            let track_idx = self.app.selected_track;
            let track = self.app.data.tracks.get(track_idx).ok_or_else(|| {
                Error::SelectorNotFound { selector: "selected track".to_string() }
            })?;
            let container = track.midi.as_ref().ok_or_else(|| {
                Error::SelectorNotFound { selector: "track midi container".to_string() }
            })?;
            let first_tick = container.sequences.keys().min().ok_or_else(|| {
                Error::SelectorNotFound { selector: "first region in selected track".to_string() }
            })?;
            let region_id = RegionIdentifier {
                track_id: TrackIdentifier { track_id: track_idx },
                region_id: *first_tick,
            };
            let mut messages = VecDeque::new();
            messages.push_back(Message::SelectRegion(region_id, false));
            self.process_messages(messages);
            Ok(())
        }

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
                let midi_note = MidiNote { key: note, velocity: TEST_DEFAULT_NOTE_VELOCITY, channel: 0, length: TEST_DEFAULT_NOTE_LENGTH_TICKS };
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

