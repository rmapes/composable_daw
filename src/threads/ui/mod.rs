mod control_bar;
mod composer_window;
mod editor_window;
mod pattern_editor;
mod midi_editor;
mod track_settings;
mod style;
mod components;
mod main_window;
mod main_menu;
mod actions;
mod file_picker;

use main_window::MainWindow;

const APP_TITLE: &str = "Composable: Pluggable DAW";

pub fn run() -> Result<(), iced::Error> {
    iced::application(
        || (MainWindow::default(), iced::Task::none()),
        MainWindow::update,
        MainWindow::view
    )
    .title(APP_TITLE)
    .subscription(MainWindow::subscription)
    .run()
}
