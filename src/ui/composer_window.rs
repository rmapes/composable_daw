use iced::mouse::Cursor;
use iced::widget::{ Container, MouseArea, Stack, button, column, container, horizontal_space, row, scrollable, text};
use iced::widget::canvas::{self, Frame, Geometry, LineCap, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Theme, border};
use iced::widget::container::Style;
use crate::models::components::Track;
use crate::models::sequences::{TSequence, Tick};
use crate::models::shared::PatternIdentifier;
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

const TIMELINE_WIDTH: f32 = 950.0;

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

    pub fn view(&self, tracks: &[Track], selected_track: usize, ppq: u32) -> Element<'_, Message> {
        const BARS_IN_TIMELINE: u32 = 16;
        let length_in_ticks = ppq * 4 * BARS_IN_TIMELINE;
        let length_per_tick = TIMELINE_WIDTH / length_in_ticks as f32;

        let ruler_layer = row![
            horizontal_space().width(Length::Fixed(100.0)),
            iced::widget::canvas(tick_ruler(length_per_tick, ppq, 4, BARS_IN_TIMELINE)).width(Length::Fixed(TIMELINE_WIDTH))                
        ].height(Length::Fixed(10.0));

        components::module(
            column![
                self.controls(),
                ruler_layer,
                self.track_list(tracks, selected_track, length_per_tick, ppq, 4, BARS_IN_TIMELINE),
            ]
            .spacing(0)
            .width(self.width)
            .height(self.height)
            .into()
        ).into()
    }
    fn controls(&self) -> Element<'_, Message> {
        row![
            button("+").on_press(Message::AddTrack),
        ].into()
    }
    fn track_list(&self, tracks: &[Track], selected_track: usize, length_per_tick: f32, ppq: u32,  beats_per_bar: u8, bars_in_timeline: u32) -> Element<'_, Message> {
        let mut track_list = column![].spacing(1);

        // Iterate over our tasks and create a widget for each one
        for (id, track) in tracks.iter().enumerate() {
            // `id` is the index (0, 1, 2, ...)
            // `task` is a &Task

            let selected = id == selected_track;
            let track_view = self.track(track, selected, length_per_tick, ppq, beats_per_bar, bars_in_timeline);

            // Add the track
            track_list = track_list.push(track_view);
        }

        // Wrap the Column in a Scrollable
        let scrollable_list = scrollable(track_list);

        // Put the scrollable list inside a container to give it some padding
        // and limit its height.
        container(scrollable_list)
            .center_x(Length::Fill)
            .align_y(iced::alignment::Vertical::Top) // You can also set a fixed height, e.g., `Length::Fixed(400.0)`
            .padding(0)
            .into()       
    }

    fn track(&self, track: &Track, is_selected: bool, length_per_tick: f32, ppq: u32,  beats_per_bar: u8, bars_in_timeline: u32) -> Element<'_, Message> {
        container(row![
            self.track_settings(track),
            // Timeline view
            self.timeline_view(track, length_per_tick, ppq, beats_per_bar, bars_in_timeline).width(Length::Fill),
        ]).height(Length::Fixed(50.0))
        .align_y(iced::alignment::Vertical::Top) // You can also set a fixed height, e.g., `Length::Fixed(400.0)`
        .style(track_style(is_selected))
        .into()
    }

    fn track_settings(&self, track: &Track) -> Element<'_, Message> {
        let content = column![
                text(track.name.clone())
            ]
            .width(Length::Fixed(100.0))
            .height(Length::Fill)
            ;
        MouseArea::new(content).on_press(Message::SelectTrack(track.id)).into()
    }

    fn timeline_view(&self, track: &Track, length_per_tick: f32, ppq: u32,  beats_per_bar: u8, bars_in_timeline: u32) -> Container<'_, Message> {
        // Total width of the stack
        // 1. Timeline Canvas (Background Layer)
        let timeline_layer = iced::widget::canvas(timeline(length_per_tick, ppq, beats_per_bar, bars_in_timeline))
        .width(Length::Fixed(TIMELINE_WIDTH))
        .height(Length::Fill);

        // 2. Interactive Markers (Foreground Layer)
        let regions: Vec<Element<'_, Message>> = self.get_regions(track).into_iter()
        .filter(|(_, _, id)| {id.is_some()} )
        .map(|(start, length, id)| {
        // Assuming 16 bars in the timeline, convert to length per tick
        let x = start as f32 * length_per_tick;
        let w = length as f32 * length_per_tick;
 
        // Create the button widget
        let button = self.region(w, id.unwrap());

        row![
            horizontal_space().width(x),
            button,
        ].into()

        }).collect();

        // 3. Combine Layers
        let mut layers: Vec<Element<'_, Message>> = Vec::new();
        layers.push(timeline_layer.into());
        layers.extend(regions);

        let content = Stack::with_children(layers)
            .width(Length::Fixed(TIMELINE_WIDTH))
            .height(Length::Fixed(50.0)) // Assuming track height is 50px
            ;

        let clickable_content: MouseArea<'_, Message, _, _> = MouseArea::new(
            content
        ).on_press(Message::DeselectAllPatterns());

        components::display(clickable_content.into())
    }

    fn get_regions(&self, track: &Track) -> Vec<(Tick, Tick, Option<PatternIdentifier>)> {
        track.midi.as_ref().map(|m| {
            m.sequences.iter().map(|(tick, region)| {
                let identifier = match region {
                    crate::models::sequences::Sequence::Pattern(pattern_seq) => Some(pattern_seq.id),
                    crate::models::sequences::Sequence::SequenceContainer(_sequence_container) => None,
                };
                (*tick, region.length_in_ticks(), identifier)
            }).collect()
        }).unwrap_or_default()
    }

    fn region<'a>(&self, width: f32, region_id: PatternIdentifier) -> Element<'a, Message> {
    
        // 1. Visual Element (The thin, tall "rectangle")
        let region_marker = container(text(""))
            .width(iced::Length::Fixed(width)) 
            .height(iced::Length::Fill)    
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgb(0.0, 0.4, 0.6).into()), 
                border: border::rounded(5),
                ..Default::default()
            });
    
        // 2. Interactive Element
        // Wrap it in a button and handle the press action
        button(region_marker)
            .on_press(Message::SelectPattern(region_id, false)) // Your custom message
            .padding(0) // Remove padding so the clickable area matches the 2px width
            .into()
        // marker_line.into()
    }
}

pub struct TrackTimeline {
    length_per_tick: f32,
    ppq: u32,
    beats_per_bar: u8,
    total_bars: u32,
    cache: canvas::Cache,
}

impl TrackTimeline {
    pub fn new(length_per_tick: f32, ppq: u32, beats_per_bar: u8, total_bars: u32) -> Self {
        Self {length_per_tick, ppq, beats_per_bar, total_bars, cache: canvas::Cache::new()}
    }
}

pub fn timeline(length_per_tick: f32, ppq: u32, beats_per_bar: u8, total_bars: u32) -> TrackTimeline {
    TrackTimeline::new(length_per_tick, ppq, beats_per_bar, total_bars)
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
        // Use the cache; if the canvas size hasn't changed, this avoids re-drawing.
        let geometry = self.cache.draw(renderer, bounds.size(), |frame: &mut Frame| {
            
            // --- Custom Drawing Logic ---
            
            // Background color
            frame.fill(&Path::rectangle(Point::ORIGIN, bounds.size()), Color::from_rgb8(0x00, 0x00, 0x00));

            let height = bounds.height;


            let length_per_bar = self.length_per_tick * self.beats_per_bar as f32 * self.ppq as f32;

            // Define stroke style for the wave
            let bar_line_stroke = Stroke {
                style: canvas::Style::Solid(Color::from_rgb8(0x30, 0x30, 0x30)),
                width: 1.0,
                line_cap: LineCap::Square,
                ..Stroke::default()
            };

            // Draw the wave
            for bar in 0..self.total_bars {
                let xpos = bar as f32 * length_per_bar;
                if xpos < bounds.width {
                    frame.stroke(&Path::line(
                        Point { x: xpos, y: 0.0 },
                        Point { x: xpos, y: height },
                    ), bar_line_stroke);
                }
            }

            // --- End Drawing Logic ---
        });

        vec![geometry]
    }
}

pub struct TickRuler {
    length_per_tick: f32,
    ppq: u32,
    beats_per_bar: u8,
    total_bars: u32,
    cache: canvas::Cache,
}

impl TickRuler {
    pub fn new(length_per_tick: f32, ppq: u32, beats_per_bar: u8, total_bars: u32) -> Self {
        Self {length_per_tick, ppq, beats_per_bar, total_bars, cache: canvas::Cache::new()}
    }
}

pub fn tick_ruler(length_per_tick: f32, ppq: u32, beats_per_bar: u8, total_bars: u32) -> TickRuler {
    TickRuler::new(length_per_tick, ppq, beats_per_bar, total_bars)
}

impl canvas::Program<Message, Theme> for TickRuler {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        // Use the cache; if the canvas size hasn't changed, this avoids re-drawing.
        let geometry = self.cache.draw(renderer, bounds.size(), |frame: &mut Frame| {
            
            // --- Custom Drawing Logic ---
            
            // Background color
            frame.fill(&Path::rectangle(Point::ORIGIN, bounds.size()), Color::from_rgb8(0x00, 0x00, 0x00));

            let length_per_division = self.length_per_tick * self.ppq as f32;

            // Define stroke style for the wave
            let bar_line_stroke = Stroke {
                style: canvas::Style::Solid(Color::from_rgb8(0x90, 0x90, 0x90)),
                width: 1.0,
                line_cap: LineCap::Square,
                ..Stroke::default()
            };

            // Define stroke style for the wave
            let division_line_stroke = Stroke {
                style: canvas::Style::Solid(Color::from_rgb8(0x60, 0x60, 0x60)),
                width: 1.0,
                line_cap: LineCap::Square,
                ..Stroke::default()
            };

            // Draw the wave
            let total_divisions = self.total_bars * self.beats_per_bar as u32;
            for division in 0..total_divisions {
                let xpos = division as f32 * length_per_division;
                if xpos < bounds.width {
                    let stroke = {if division % self.beats_per_bar as u32 == 0 {bar_line_stroke} else {division_line_stroke}};
                    let height = {if division % self.beats_per_bar as u32 == 0 {bounds.height} else {bounds.height / 2.0}};
                    frame.stroke(&Path::line(
                        Point { x: xpos, y: 0.0 },
                        Point { x: xpos, y: height },
                    ), stroke);
                }
            }

            // --- End Drawing Logic ---
        });

        vec![geometry]
    }
}





