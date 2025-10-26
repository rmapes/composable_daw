mod control_bar;
mod composer_window;
mod pattern_editor;
mod track_settings;
mod style;
mod components;

use iced::widget::{column, Column, row, text};
use iced::Length::Fixed;
use iced::Element;
use crate::models::shared::SongData;
use crate::engine;

use std::sync::{Arc, Mutex};
use std::rc::Rc;

//////////////////////
/// Entry point for iced ui
/// 

const APP_TITLE: &str = "Composable: Pluggable DAW";

struct MainWindow {
    // Core application data and engine
    engine: Rc<engine::EngineController>,
    data: Arc<Mutex<SongData>>,

    // Mutable state
    selected_track: usize,
    // Preferences
    width: f32,
    height: f32,

    // UI subcomponents
    control_bar: control_bar::Component,
    composer_window: composer_window::Component,
    pattern_editor: pattern_editor::Component,
    track_settings: track_settings::Component,


}

impl Default for MainWindow {
    fn default() -> Self {
        let data = Arc::new(Mutex::new(SongData::new()));
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
            width: 600_f32,
            height: 400_f32,
            control_bar: control_bar::Component::new(50_f32),
            composer_window: composer_window::Component::new(500_f32, 200_f32),
            pattern_editor: pattern_editor::Component::new(500_f32, 150_f32),
            track_settings: track_settings::Component::new(100_f32, 500_f32),            
        }
    }
}
#[derive(Debug, Clone, Copy)]
enum Message {

}
impl MainWindow {
    pub fn update(&mut self, _msg: Message) {

    }
    pub fn view(&self) ->Element<'_, Message> {
        let content: Column<'_, Message> = {
            if let Ok(song) = self.data.try_lock() {
                let selected_track =  &song.tracks[self.selected_track];       
                column![
                    self.control_bar.view(),
                    // Replace the following row and column layout with https://github.com/iced-rs/iced/blob/master/examples/pane_grid/README.md
                    row![
                        components::module_slot(
                            self.track_settings.view(selected_track)
                        ),
                        column![
                            components::module_slot(
                                self.composer_window.view(&song.tracks, self.selected_track),
                            ),
                            components::module_slot(
                                self.pattern_editor.view(),
                            )
                        ]
                    ]
                ].width(Fixed(self.width)).height(Fixed(self.height))
                } else {
                column![] // TODO: store a local copy of the song data to deal with try_lock failing
            }
        };
        components::rack(content.into()).into()
    }

    // Don't forget to stop engine on shutdown
    //     engine.quit();
}

pub fn run() -> Result<(), iced::Error> {
    iced::application(APP_TITLE, MainWindow::update, MainWindow::view).run()
}
