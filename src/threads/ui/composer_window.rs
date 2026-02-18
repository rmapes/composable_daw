use iced::mouse::{Button, Cursor, Event};
use iced::widget::{Container, MouseArea, stack, button, column, container, Space, row, scrollable, text};
use iced::widget::canvas::{self, Frame, Geometry, LineCap, Path, Stroke, Fill};
use iced::{Color, Element, Length, Point, Rectangle, Theme, border};
use iced::widget::container::Style;
use super::super::engine::actions::Actions;
use crate::models::components::Track;
use crate::models::sequences::{TSequence, Tick};
use crate::models::shared::RegionIdentifier;
use super::components;
use super::actions::Message;
use super::main_window::DragState;


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

    pub fn view(
        &self,
        tracks: &[Track],
        selected_track: usize,
        ppq: u32,
        playhead: Tick,
        dragging_region: Option<&DragState>,
    ) -> Element<'_, Message> {
        const BARS_IN_TIMELINE: u32 = 16;
        let length_in_ticks = ppq * 4 * BARS_IN_TIMELINE;
        let length_per_tick = TIMELINE_WIDTH / length_in_ticks as f32;
        const RULER_HEIGHT: f32 = 10.0;

        let ruler_layer = row![
            Space::new().width(Length::Fixed(100.0)),
            iced::widget::canvas(tick_ruler(length_per_tick, ppq, 4, BARS_IN_TIMELINE)).width(Length::Fixed(TIMELINE_WIDTH))                
        ].height(Length::Fixed(RULER_HEIGHT));

        components::module(
            column![
                self.controls(),
                stack![
                    column![
                    ruler_layer,
                    self.track_list(tracks, selected_track, length_per_tick, ppq, 4, BARS_IN_TIMELINE, RULER_HEIGHT, dragging_region),
                    ],
                    row![
                        Space::new().width(Length::Fixed(100.0)),
                        iced::widget::canvas(playhead_marker(playhead, length_per_tick, RULER_HEIGHT)).height(Length::Fill).width(Length::Fill)
                    ].width(Length::Fill)         
                ]
                
            ]
            .spacing(0)
            .width(self.width)
            .height(self.height)
            .into()
        ).into()
    }
    fn controls(&self) -> Element<'_, Message> {
        row![
            button("+").on_press(Message::Engine(Actions::AddTrack)),
        ].into()
    }
    fn track_list(
        &self,
        tracks: &[Track],
        selected_track: usize,
        length_per_tick: f32,
        ppq: u32,
        beats_per_bar: u8,
        bars_in_timeline: u32,
        ruler_height: f32,
        dragging_region: Option<&DragState>,
    ) -> Element<'_, Message> {
        const TRACK_HEIGHT: f32 = 50.0;
        let mut track_list = column![].spacing(1);

        for (id, track) in tracks.iter().enumerate() {
            let selected = id == selected_track;
            let track_view = self.track(
                track,
                selected,
                length_per_tick,
                ppq,
                beats_per_bar,
                bars_in_timeline,
                ruler_height,
                TRACK_HEIGHT,
                id,
                dragging_region,
            );

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

    fn track(
        &self,
        track: &Track,
        is_selected: bool,
        length_per_tick: f32,
        ppq: u32,
        beats_per_bar: u8,
        bars_in_timeline: u32,
        ruler_height: f32,
        track_height: f32,
        track_index: usize,
        dragging_region: Option<&DragState>,
    ) -> Element<'_, Message> {
        container(row![
            self.track_settings(track),
            self.timeline_view(
                track,
                length_per_tick,
                ppq,
                beats_per_bar,
                bars_in_timeline,
                ruler_height,
                track_height,
                track_index,
                dragging_region,
            )
            .width(Length::Fill),
        ])
        .height(Length::Fixed(track_height))
        .align_y(iced::alignment::Vertical::Top)
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

    fn timeline_view(
        &self,
        track: &Track,
        length_per_tick: f32,
        ppq: u32,
        beats_per_bar: u8,
        bars_in_timeline: u32,
        ruler_height: f32,
        track_height: f32,
        track_index: usize,
        dragging_region: Option<&DragState>,
    ) -> Container<'_, Message> {
        let regions: Vec<(Tick, Tick, RegionIdentifier)> = self
            .get_regions(track)
            .into_iter()
            .filter_map(|(start, length, id)| id.map(|id| (start, length, id)))
            .collect();
        let program = InteractiveTimelineCanvas {
            regions,
            length_per_tick,
            ppq,
            beats_per_bar,
            bars_in_timeline,
            track_index,
            ruler_height,
            track_height,
            drag_state: dragging_region.cloned(),
        };
        let content = iced::widget::canvas(program)
            .width(Length::Fixed(TIMELINE_WIDTH))
            .height(Length::Fixed(track_height));
        components::display(content.into()).id("TimelineBackground")
    }

    fn get_regions(&self, track: &Track) -> Vec<(Tick, Tick, Option<RegionIdentifier>)> {
        track.midi.as_ref().map(|m| {
            m.sequences.iter().map(|(tick, region)| {
                let identifier = match region {
                    crate::models::sequences::Sequence::Pattern(pattern_seq) => Some(pattern_seq.id),
                    crate::models::sequences::Sequence::SequenceContainer(_sequence_container) => None,
                    crate::models::sequences::Sequence::Midi(midi_seq) =>  Some(midi_seq.id),
                };
                (*tick, region.length_in_ticks(), identifier)
            }).collect()
        }).unwrap_or_default()
    }

    fn region<'a>(&self, width: f32, region_id: RegionIdentifier, debug_id: String) -> Element<'a, Message> {
        let region_marker = container(text(""))
            .width(iced::Length::Fixed(width))
            .height(iced::Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgb(0.0, 0.4, 0.6).into()),
                border: border::rounded(5),
                ..Default::default()
            })
            .id(debug_id);
        button(region_marker)
            .on_press(Message::SelectRegion(region_id, false))
            .padding(0)
            .into()
    }
}

const DRAG_THRESHOLD_PX: f32 = 5.0;
const REGION_COLOR: Color = Color::from_rgb(0.0, 0.4, 0.6);
const REGION_VALID_DROP: Color = Color::from_rgb(0.4, 0.7, 0.9);
const REGION_INVALID_DROP: Color = Color::from_rgb(0.9, 0.2, 0.2);

#[derive(Clone)]
pub struct InteractiveTimelineCanvas {
    pub regions: Vec<(Tick, Tick, RegionIdentifier)>,
    pub length_per_tick: f32,
    pub ppq: u32,
    pub beats_per_bar: u8,
    pub bars_in_timeline: u32,
    pub track_index: usize,
    pub ruler_height: f32,
    pub track_height: f32,
    pub drag_state: Option<DragState>,
}

/// Pending drag start: (region_id, press_x, press_y) in canvas coords.
type PendingDrag = (RegionIdentifier, f32, f32);

impl canvas::Program<Message, Theme> for InteractiveTimelineCanvas {
    type State = Option<PendingDrag>;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<iced::widget::Action<Message>> {
        let cursor_position = cursor.position_in(bounds)?;
        let (x, y) = (cursor_position.x, cursor_position.y);
        let timeline_y = self.ruler_height + self.track_index as f32 * self.track_height + y;

        match event {
            iced::Event::Mouse(mouse_event) => match mouse_event {
                Event::ButtonPressed(Button::Left) => {
                    if let Some(region_id) = self.region_at_x(x) {
                        *state = Some((region_id, x, y));
                    } else {
                        return Some(iced::widget::Action::publish(Message::DeselectAllRegions()));
                    }
                }
                Event::ButtonReleased(Button::Left) => {
                    if let Some((region_id, _, _)) = state.take() {
                        return Some(iced::widget::Action::publish(Message::RegionClick(region_id)));
                    }
                    return Some(iced::widget::Action::publish(Message::EndRegionDrag));
                }
                Event::CursorMoved { .. } => {
                    if let Some(pending) = state.take() {
                        let (region_id, px, py) = pending;
                        let dist = ((x - px).powi(2) + (y - py).powi(2)).sqrt();
                        if dist >= DRAG_THRESHOLD_PX {
                            let press_timeline_y = self.ruler_height + self.track_index as f32 * self.track_height + py;
                            return Some(iced::widget::Action::publish(Message::StartRegionDrag(
                                region_id, px, press_timeline_y, x, timeline_y,
                            )));
                        }
                        *state = Some((region_id, px, py));
                    } else {
                        return Some(iced::widget::Action::publish(Message::UpdateRegionDrag(x, timeline_y)));
                    }
                }
                _ => {}
            },
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let geometry = canvas::Cache::new().draw(renderer, bounds.size(), |frame: &mut Frame| {
            frame.fill(
                &Path::rectangle(Point::ORIGIN, bounds.size()),
                Color::from_rgb8(0x00, 0x00, 0x00),
            );
            let height = bounds.height;
            let length_per_bar =
                self.length_per_tick * self.beats_per_bar as f32 * self.ppq as f32;
            let bar_stroke = Stroke {
                style: canvas::Style::Solid(Color::from_rgb8(0x30, 0x30, 0x30)),
                width: 1.0,
                line_cap: LineCap::Square,
                ..Stroke::default()
            };
            for bar in 0..self.bars_in_timeline {
                let xpos = bar as f32 * length_per_bar;
                if xpos < bounds.width {
                    frame.stroke(
                        &Path::line(Point::new(xpos, 0.0), Point::new(xpos, height)),
                        bar_stroke,
                    );
                }
            }
            let dragging_id = self.drag_state.as_ref().map(|d| d.region_id);
            for (start, length, region_id) in &self.regions {
                if Some(*region_id) == dragging_id {
                    continue;
                }
                let x = *start as f32 * self.length_per_tick;
                let w = *length as f32 * self.length_per_tick;
                let rect = Path::rectangle(Point::new(x, 0.0), iced::Size::new(w, height));
                frame.fill(&rect, Fill::from(REGION_COLOR));
            }
            if let Some(ref drag) = self.drag_state && drag.current_track_index == self.track_index {
                    let x = drag.current_tick as f32 * self.length_per_tick;
                    let w = drag.region_length as f32 * self.length_per_tick;
                    let color = if drag.is_valid_drop {
                        REGION_VALID_DROP
                    } else {
                        REGION_INVALID_DROP
                    };
                    let rect = Path::rectangle(Point::new(x, 0.0), iced::Size::new(w, height));
                    frame.fill(&rect, Fill::from(color));
            }
        });
        vec![geometry]
    }
}

impl InteractiveTimelineCanvas {
    fn region_at_x(&self, x: f32) -> Option<RegionIdentifier> {
        for (start, length, id) in &self.regions {
            let rx = *start as f32 * self.length_per_tick;
            let rw = *length as f32 * self.length_per_tick;
            if x >= rx && x < rx + rw {
                return Some(*id);
            }
        }
        None
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

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<iced::widget::Action<Message>> {
        if let iced::Event::Mouse(mouse_event) = event 
            && let Some(cursor_position) = cursor.position_in(bounds) {
                // Check for a mouse button press event (e.g., left click)
                if matches!(mouse_event, Event::ButtonPressed(Button::Left)) {
                    // convert cursor position to tick
                    let tick_position = cursor_position.x / self.length_per_tick;
                    // cursor_position is relative to the canvas bounds
                    return Some(iced::widget::Action::publish(Message::SetPlayhead(tick_position as u32)));
                }
        }
        None
    }

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

pub struct PlayheadMarker {
    length_per_tick: f32,
    playhead: Tick,
    rule_height: f32,
    cache: canvas::Cache,
}

impl PlayheadMarker {
    pub fn new(playhead: Tick, length_per_tick: f32, rule_height: f32) -> Self {
        Self {length_per_tick, playhead, rule_height, cache: canvas::Cache::new()}
    }
}

pub fn playhead_marker(playhead: Tick, length_per_tick: f32, rule_height: f32) -> PlayheadMarker {
    PlayheadMarker::new(playhead, length_per_tick, rule_height)
}

impl canvas::Program<Message, Theme> for PlayheadMarker {
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
            frame.fill(&Path::rectangle(Point::ORIGIN, bounds.size()), Color::TRANSPARENT);

            // Draw the marker
            let xpos = self.playhead as f32 * self.length_per_tick;
            // Draw head
            draw_playhead(xpos, 0.0, bounds, self.rule_height, frame);

            // --- End Drawing Logic ---
        });

        vec![geometry]
    }
}
fn draw_playhead(x: f32, y_top: f32, bounds: Rectangle, head_height: f32, frame: &mut Frame) {
    let head_width = head_height;
    // The point where the rectangle part stops and the triangle part begins
    let shoulder_height = head_height * 0.5; 
    
    // Logic Pro-ish Color (Light Gray/Whiteish)
    let playhead_color = Color::from_rgb8(220, 220, 220); 

    // 2. DRAW THE VERTICAL LINE (The "String")
    // We draw this first so it appears behind the head if they overlap slightly
    let line_path = Path::line(
        Point::new(x, y_top + head_height),
        Point::new(x, bounds.height),
    );

    frame.stroke(
        &line_path,
        Stroke::default()
            .with_color(playhead_color)
            .with_width(1.0),
    );

    // 3. DRAW THE HEAD (The "Cap")
    // Shape: Inverted House / Pentagon
    let head_path = Path::new(|p| {
        // Start at top-left corner
        p.move_to(Point::new(x - head_width / 2.0, y_top));
        
        // Draw to top-right corner
        p.line_to(Point::new(x + head_width / 2.0, y_top));
        
        // Draw down to the "shoulder" (right side)
        p.line_to(Point::new(x + head_width / 2.0, y_top + shoulder_height));
        
        // Draw to the tip (center bottom)
        p.line_to(Point::new(x, y_top + head_height));
        
        // Draw up to the "shoulder" (left side)
        p.line_to(Point::new(x - head_width / 2.0, y_top + shoulder_height));
        
        // Close the shape back to start
        p.close();
    });

    // Fill the head
    frame.fill(&head_path, Fill::from(playhead_color));
    
    // Optional: Add a slight darker stroke around the head for contrast
    frame.stroke(
        &head_path, 
        Stroke::default().with_color(Color::BLACK).with_width(1.0)
    );
}




