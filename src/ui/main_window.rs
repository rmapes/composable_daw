use iced::widget::{column, Column, row};
use iced::Length;
use iced::Element;
use iced::{Subscription, window, Task};
use log::info;
use crate::models::sequences::Sequence;
use crate::models::shared::ProjectData;
use crate::engine;

use super::components;
use super::composer_window;
use super::pattern_editor;
use super::track_settings;
use super::control_bar;
use super::actions::Message;

use std::sync::{Arc, RwLock};
use std::rc::Rc;

//////////////////////
/// Entry point for iced ui
/// 

pub struct MainWindow {
    // Core application data and engine
    engine: Rc<engine::EngineController>,
    data: Arc<RwLock<ProjectData>>,

    // Mutable state
    selected_track: usize,
    selected_pattern: Option<u32>,
    is_playing: bool,
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
        let engine = Rc::new(engine::start(
            {
                move |_player_state: &engine::PlayerState| {
                    // Ignore this for the moment.
                    // Eventually, I'll need to work out how to handle internal state updates
                }
            },
            Arc::clone(&data),)
            
        );
    
        Self {
            engine,
            data,
            selected_track: 0,
            selected_pattern: Some(0), // Temporary: select pattern by default. Relies on track beging created with initial pattern
            is_playing: false,
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
                    song.get_track_by_id(&note_identifier.pattern_id.track_id)
                    .get_pattern_by_id(&note_identifier.pattern_id)
                    .toggle_on(note_identifier.beat_num, note_identifier.note_num);
                }               
                // No further task to do
                Task::none()
            },
            Message::Play => {
                self.is_playing = true;                
                self.engine.play_midi();
                Task::done(Message::PlayStopped)
            },
            Message::PlayStopped => {
                self.is_playing = false;
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
        }
    }
    pub fn view(&self) ->Element<'_, Message> {
        let content: Column<'_, Message> = {
            if let Ok(song) = self.data.try_read() {
                let selected_track =  &song.tracks[self.selected_track]; 
                let selected_region: Option<&Sequence> = self.selected_pattern
                    .and_then(|selection| selected_track.midi.as_ref()
                        .and_then(|sequence| sequence.sequences.get(&selection)));
                let selected_pattern = match selected_region {
                    Some(Sequence::Pattern(p)) => Some(p),
                    _ => None, // Fix this when we support other region types
                };
                column![
                    self.control_bar.view(),
                    // Replace the following row and column layout with https://github.com/iced-rs/iced/blob/master/examples/pane_grid/README.md
                    row![
                        components::module_slot(
                            self.track_settings.view(selected_track)
                        ).width(Length::Shrink), // Shrink to fit channel strips
                        column![
                            components::module_slot(
                                self.composer_window.view(&song.tracks, self.selected_track),
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
        // Subscribe to all window events
        window::events().map(|(_id, event)| Message::WindowEvent(event))
    }
    // Don't forget to stop engine on shutdown
    fn shutdown(&self) {
        info!("Shutting down");
        self.engine.quit();
    }
}
