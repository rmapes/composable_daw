pub mod actions;
mod components;
mod composer_window;
mod control_bar;
mod editor_window;
mod instrument_editor;
mod file_picker;
mod main_menu;
mod main_window;
mod midi_editor;
mod pattern_editor;
mod style;
mod track_settings;

use main_window::MainWindow;

const APP_TITLE: &str = "Composable: Pluggable DAW";

pub fn run() -> Result<(), iced::Error> {
    iced::application(
        || (MainWindow::default(), iced::Task::none()),
        MainWindow::update,
        MainWindow::view,
    )
    .title(APP_TITLE)
    .subscription(MainWindow::subscription)
    .run()
}
