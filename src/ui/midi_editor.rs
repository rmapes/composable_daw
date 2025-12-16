use iced::widget::Column;
use iced::mouse::Cursor;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme, Vector};
use crate::models::sequences::MidiSeq;

use super::actions::Message;

use super::components;


use iced::widget::canvas::{
    self, Canvas, Frame, Geometry, Path, Stroke, Text, LineCap, Style,
};

// --- CONFIGURATION CONSTANTS ---
const BEATS_PER_BAR: u32 = 4;
const SUBDIVISIONS_PER_BEAT: u32 = 4; // Quarters of a beat
const NOTE_COUNT: u8 = 128; // Standard MIDI note range
const VISIBLE_NOTES: u8 = 24; // How many keys are visible at once (e.g., two octaves)

const RULER_HEIGHT: f32 = 25.0;
const KEYBOARD_WIDTH: f32 = 75.0;
const NOTE_HEIGHT: f32 = 18.0; // Height of one row (MIDI Note)
const BEAT_WIDTH: f32 = 100.0; // Width of one beat

// --- MODEL & STATE ---
#[derive(Default)]
pub struct MidiEditor {
    cache: canvas::Cache,
    scroll_offset: Vector, // Scroll state (x, y)
    // Add your MIDI note data structure here
    // e.g., notes: Vec<MidiNote>
}

// Implement the Program trait for the editor
impl canvas::Program<Message, Theme> for MidiEditor {
    type State = ();

    fn draw(&self, _state: &Self::State, renderer: &iced::Renderer,
        _theme: &Theme, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry> {
        let editor_geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            // 1. Draw the UI Sections
            let ruler_rect = Rectangle::new(Point::ORIGIN, Size::new(bounds.width, RULER_HEIGHT));
            let keyboard_rect = Rectangle::new(
                Point::new(0.0, RULER_HEIGHT),
                Size::new(KEYBOARD_WIDTH, bounds.height - RULER_HEIGHT),
            );
            let grid_rect = Rectangle::new(
                Point::new(KEYBOARD_WIDTH, RULER_HEIGHT),
                Size::new(bounds.width - KEYBOARD_WIDTH, bounds.height - RULER_HEIGHT),
            );

            // 2. Draw the Ruler
            self.draw_ruler(frame, &ruler_rect, &self.scroll_offset);

            // 3. Draw the Piano Keyboard
            self.draw_keyboard(frame, &keyboard_rect, &self.scroll_offset);

            // 4. Draw the Note Grid and Notes
            // We use a separate sub-frame for the grid to easily apply scroll offset
            frame.translate(Vector::new(KEYBOARD_WIDTH, RULER_HEIGHT));
            self.draw_grid(frame, &grid_rect.size(), &self.scroll_offset);
            // self.draw_midi_notes(frame, &self.notes, &self.scroll_offset); 
        });

        vec![editor_geometry]
    }

    // You would implement update to handle mouse clicks (drawing notes) and scrolling
    // fn update(...) -> (Self::State, Option<Option<Message>>) { ... }
}

// --- DRAWING HELPER METHODS ---

impl MidiEditor {
    // 1. Draw the Ruler (Time Axis)
    fn draw_ruler(&self, frame: &mut Frame, bounds: &Rectangle, scroll: &Vector) {
        frame.fill(
            &Path::rectangle(bounds.position(), bounds.size()),
            Color::from_rgb(0.1, 0.1, 0.1), // Dark background
        );

        let total_beats_visible = bounds.width / BEAT_WIDTH;
        let start_beat = (-scroll.x / BEAT_WIDTH).floor() as u32;
        let end_beat = start_beat + total_beats_visible.ceil() as u32 + 1;

        for beat in start_beat..end_beat {
            let x = beat as f32 * BEAT_WIDTH + scroll.x;

            // Bar Lines (Longest lines)
            if beat % BEATS_PER_BAR == 0 {
                let bar_text = Text {
                    content: (beat / BEATS_PER_BAR + 1).to_string(),
                    position: Point::new(x + 5.0, bounds.center_y()),
                    color: Color::WHITE,
                    size: iced::Pixels::from(14.0),
                    ..Text::default()
                };
                frame.fill_text(bar_text);
            }

            // Beat Lines (Longer lines)
            if beat % BEATS_PER_BAR != 0 {
                let beat_line = Path::line(
                    Point::new(x, bounds.height * 0.5),
                    Point::new(x, bounds.height),
                );
                frame.stroke(
                    &beat_line,
                    Stroke {
                        style: Style::from(Color::from_rgb(0.5, 0.5, 0.5)),
                        width: 1.0,
                        ..Stroke::default()
                    },
                );
            }

            // Subdivisions (Quarters of a beat - Shortest lines)
            for sub in 1..SUBDIVISIONS_PER_BEAT {
                let sub_x = x + (sub as f32 * (BEAT_WIDTH / SUBDIVISIONS_PER_BEAT as f32));
                let sub_line = Path::line(
                    Point::new(sub_x, bounds.height * 0.75),
                    Point::new(sub_x, bounds.height),
                );
                frame.stroke(
                    &sub_line,
                    Stroke {
                        style: Style::from(Color::from_rgb(0.3, 0.3, 0.3)),
                        width: 0.5,
                        ..Stroke::default()
                    },
                );
            }
        }
    }

    // 2. Draw the Piano Keyboard
    fn draw_keyboard(&self, frame: &mut Frame, bounds: &Rectangle, scroll: &Vector) {
        frame.fill(
            &Path::rectangle(bounds.position(), bounds.size()),
            Color::from_rgb(0.2, 0.2, 0.2), // Keyboard background
        );

        // Clip to the keyboard area
        // frame.with_save(|frame| {
        //     frame.with_clip(*bounds, |frame| {

            let start_note_index = (-scroll.y / NOTE_HEIGHT).floor() as u8;
            let end_note_index = start_note_index + VISIBLE_NOTES + 1;

            for i in start_note_index..end_note_index {
                let y = i as f32 * NOTE_HEIGHT + scroll.y + bounds.y;
                let is_sharp = match i % 12 {
                    1 | 3 | 6 | 8 | 10 => true, // C#, D#, F#, G#, A#
                    _ => false,
                };
                let is_octave_c = i % 12 == 0; // C notes

                // Draw the key background
                let key_color = if is_sharp {
                    Color::BLACK
                } else {
                    Color::WHITE
                };
                let key_rect = Path::rectangle(
                    Point::new(bounds.x, y),
                    Size::new(KEYBOARD_WIDTH, NOTE_HEIGHT),
                );
                frame.fill(&key_rect, key_color);

                // Draw outlines and note label
                let outline_color = if is_octave_c {
                    Color::from_rgb(0.5, 0.0, 0.0) // Red for Octave C
                } else {
                    Color::from_rgb(0.5, 0.5, 0.5)
                };
                frame.stroke(
                    &key_rect,
                    Stroke {
                        style: Style::from(outline_color),
                        width: 1.0,
                        ..Stroke::default()
                    },
                );

                // You would add note names here (e.g., C4, F#5)
            }
        // })
        // });
    }

    // 3. Draw the Note Grid
    fn draw_grid(&self, frame: &mut Frame, bounds: &Size, scroll: &Vector) {
        // Apply scroll offset to the grid drawing
        frame.translate(*scroll);

        let total_time_span = 16 as f32 * BEAT_WIDTH; // Example: 16 beats

        let num_rows = VISIBLE_NOTES + 1;

        // Draw horizontal note lines
        for i in 0..num_rows {
            let y = i as f32 * NOTE_HEIGHT;
            let line = Path::line(Point::new(0.0, y), Point::new(total_time_span, y));

            let is_octave_c_row = (i as u8 + (-scroll.y / NOTE_HEIGHT).round() as u8) % 12 == 0;

            let stroke_style = if is_octave_c_row {
                Color::from_rgb(0.4, 0.0, 0.0) // Stronger line for C notes
            } else {
                Color::from_rgb(0.3, 0.3, 0.3)
            };

            frame.stroke(
                &line,
                Stroke {
                    style: Style::from(stroke_style),
                    width: 0.5,
                    ..Stroke::default()
                },
            );
        }

        // Draw vertical time lines (Bar, Beat, Subdivisions)
        let total_beats = (total_time_span / BEAT_WIDTH).round() as u32;

        for beat in 0..total_beats {
            let x = beat as f32 * BEAT_WIDTH;

            // Bar Lines
            if beat % BEATS_PER_BAR == 0 {
                let bar_line = Path::line(Point::new(x, 0.0), Point::new(x, bounds.height));
                frame.stroke(
                    &bar_line,
                    Stroke {
                        style: Style::from(Color::from_rgb(0.5, 0.5, 0.5)),
                        width: 2.0, // Thicker for bar
                        line_cap: LineCap::Butt,
                        ..Stroke::default()
                    },
                );
            }

            // Beat Lines
            if beat % BEATS_PER_BAR != 0 {
                let beat_line = Path::line(Point::new(x, 0.0), Point::new(x, bounds.height));
                frame.stroke(
                    &beat_line,
                    Stroke {
                        style: Style::from(Color::from_rgb(0.3, 0.3, 0.3)),
                        width: 1.0,
                        line_cap: LineCap::Butt,
                        ..Stroke::default()
                    },
                );
            }

            // Subdivision Lines
            for sub in 1..SUBDIVISIONS_PER_BEAT {
                let sub_x = x + (sub as f32 * (BEAT_WIDTH / SUBDIVISIONS_PER_BEAT as f32));
                let sub_line = Path::line(Point::new(sub_x, 0.0), Point::new(sub_x, bounds.height));
                frame.stroke(
                    &sub_line,
                    Stroke {
                        style: Style::from(Color::from_rgb(0.1, 0.1, 0.1)),
                        width: 0.5,
                        line_cap: LineCap::Butt,
                        ..Stroke::default()
                    },
                );
            }
        }
    }
    // 4. draw_midi_notes (Not implemented here, but this is where you draw your note blocks)
    // fn draw_midi_notes(...) { ... }
}

// --- CONTAINER WIDGET ---
// This is what you actually put in your Iced layout.
impl MidiEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view() -> Canvas<Self, Message> {
        Canvas::new(MidiEditor::default())
            .width(Length::Fill)
            .height(Length::Fill)
    }
}

// Example usage in your main iced::Element:
/*
let midi_editor = MidiEditor::view();

// Use a Scrollable wrapper if you want native scrollbars,
// but the canvas itself handles the internal drawing scroll.
// scrollable::Scrollable::new(midi_editor)
*/

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

    pub fn view(&self, midi_seq: &MidiSeq) -> Element<'_, Message> {
        let content = MidiEditor::view();
        components::module(
            content
            .width(self.width)
            .height(self.height).into()
        ).into()
    }
}

