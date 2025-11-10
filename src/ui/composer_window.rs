use iced::mouse::Cursor;
use iced::widget::{ Container, MouseArea, button, column, container, row, scrollable, stack, text};
use iced::widget::canvas::{self, Frame, Geometry, LineCap, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Theme};
use iced::widget::container::Style;
use log::debug; 
use crate::models::components::Track;
use crate::models::sequences::Tick;
use super::components;
use super::actions::Message;


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
    width: Length,
    height: Length,
}

impl Component {
    pub fn new(width: Length, height: Length) -> Self {
        Self { 
            width, 
            height,
        }
    } 

    pub fn view(&self, tracks: &[Track], selected_track: usize) -> Element<'_, Message> {
        components::module(
            column![
                self.controls(),
                self.track_list(tracks, selected_track),
            ]
            .width(self.width)
            .height(self.height).into()
        ).into()
    }
    fn controls(&self) -> Element<'_, Message> {
        row![
            button("+").on_press(Message::AddTrack),
        ].into()
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
            .into()       
    }

    fn track(&self, track: &Track, is_selected: bool) -> Element<'_, Message> {
        let track_bar = container(row![
            // self.track_settings()
            column![
                text(track.name.clone())
            ].width(Length::Fixed(100.0)),
            // Timeline view
            self.timeline_view(track).width(Length::Fill),
        ]).height(Length::Fixed(50.0))
        .style(track_style(is_selected));
        MouseArea::new(track_bar).on_press(Message::SelectTrack(track.id)).into()
    }

    fn timeline_view(&self, track: &Track) -> Container<'_, Message> {
        components::display(
            // stack![
                iced::widget::canvas(timeline(track.ppq * 4 * 16, track.ppq, 4)).width(Length::Fixed(950.0)).into(),
            // ].into()
        )    
    }
}

pub struct TrackTimeline {
    length_in_ticks: Tick,
    ppq: u32,
    beats_per_bar: u8,
    cache: canvas::Cache,
}

impl TrackTimeline {
    pub fn new(length_in_ticks: Tick, ppq: u32, beats_per_bar: u8) -> Self {
        Self {length_in_ticks, ppq, beats_per_bar, cache: canvas::Cache::new()}
    }
}

pub fn timeline(length_in_ticks: Tick, ppq: u32, beats_per_bar: u8) -> TrackTimeline {
    TrackTimeline::new(length_in_ticks, ppq, beats_per_bar)
}

impl canvas::Program<Message, Theme> for TrackTimeline {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let total_beats = self.length_in_ticks / self.ppq;
        let total_bars = total_beats / self.beats_per_bar as u32;        
        // Use the cache; if the canvas size hasn't changed, this avoids re-drawing.
        let geometry = self.cache.draw(renderer, bounds.size(), |frame: &mut Frame| {
            
            // --- Custom Drawing Logic ---
            
            // Background color
            frame.fill(&Path::rectangle(Point::ORIGIN, bounds.size()), Color::from_rgb8(0x00, 0x00, 0x00));

            let width = frame.width();
            let height = frame.height();

            println!("Width: {width}");

            let length_per_bar = width / total_bars as f32;
            println!("length_per_bar: {length_per_bar}");

            // Define stroke style for the wave
            let bar_line_stroke = Stroke {
                style: canvas::Style::Solid(Color::from_rgb8(0x30, 0x90, 0xF0)),
                width: 1.0,
                line_cap: LineCap::Square,
                ..Stroke::default()
            };

            // Draw the wave
            for bar in 0..total_bars {
                let xpos = bar as f32 * length_per_bar;
                frame.stroke(&Path::line(
                    Point { x: xpos, y: 0.0 },
                    Point { x: xpos, y: height },
                ), bar_line_stroke);
            }

            // --- End Drawing Logic ---
        });

        vec![geometry]
    }
}



