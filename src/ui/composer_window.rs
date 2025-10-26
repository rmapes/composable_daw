use iced::widget::{ row, column, Column, scrollable, container, text, horizontal_space};
use iced::{Color, Element, Length, Theme};
use iced::widget::container::Style; 
use crate::models::components::Track;
use super::Message;

// Define styling
pub fn track_style(is_selected: bool) -> impl Fn(&Theme) -> Style {
    let background_colour = if is_selected {
        Color::from_rgb(0.0, 0.0, 0.5) // Note: Using 0.5 for a visible dark blue
    } else {
        Color::BLACK
    };
    
    // The closure signature is correct: |_theme: &Theme| -> container::Style
    move |_theme: &Theme| Style {
        // Set the background color
        background: Some(background_colour.into()),
        text_color: Some(Color::from_rgb(0.0, 0.8, 0.2)),
        ..Default::default()
    }
}


// Define components
pub struct Component {
    width: f32,
    height: f32,
}

impl Component {
    pub fn new(width: f32, height: f32) -> Self {
        Self { 
            width, 
            height,
        }
    } 

    pub fn update(&mut self, _msg: Message) {

    }
    pub fn view(&self, tracks: &[Track], selected_track: usize) -> Column<'_, Message> {
        column![
            self.track_list(tracks, selected_track),
        ]
        .width(Length::Fixed(self.width))
        .height(Length::Fixed(self.height))
    }
    fn track_list(&self, tracks: &[Track], selected_track: usize) -> Element<'_, Message> {
        let mut track_list = column![].spacing(10);

        // Iterate over our tasks and create a widget for each one
        for (id, track) in tracks.iter().enumerate() {
            // `id` is the index (0, 1, 2, ...)
            // `task` is a &Task

            let selected = id == selected_track;
            let track_view = self.track(track, selected);

            // Add the track
            track_list = track_list.push(track_view);
        }

        // Wrap the Column in a Scrollable
        let scrollable_list = scrollable(track_list);

        // Put the scrollable list inside a container to give it some padding
        // and limit its height.
        container(scrollable_list)
            .center_x(Length::Fill)
            .center_y(Length::Fill) // You can also set a fixed height, e.g., `Length::Fixed(400.0)`
            .padding(20)
            .into()       
    }

    fn track(&self, track: &Track, is_selected: bool) -> Element<'_, Message> {
        container(row![
            // self.track_settings()
            column![
                text(track.name.clone())
            ],
            // Timeline view
            horizontal_space(),
        ])
        .style(track_style(is_selected))
        .into()

    }
}

