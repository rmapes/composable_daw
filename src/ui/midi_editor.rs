use std::collections::BTreeMap;

use iced::event::Status;
use iced::mouse::Cursor;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme, Vector};
use crate::models::sequences::{MidiNote, MidiSeq, Tick};
use crate::models::shared::RegionIdentifier;

use super::actions::{Message, MidiEditorMessage};

use super::components;


use iced::widget::canvas::{
    self, Canvas, Frame, Geometry, Path, Stroke, Text, LineCap, Style,
};

// 
// --- CONFIGURATION CONSTANTS ---
const BEATS_PER_BAR: u32 = 4;
const SUBDIVISIONS_PER_BEAT: u32 = 4; // Quarters of a beat
const NOTE_COUNT: u8 = 128; // Standard MIDI note range
const VISIBLE_NOTES: u8 = 24; // How many keys are visible at once (e.g., two octaves)

const RULER_HEIGHT: f32 = 25.0;
const KEYBOARD_WIDTH: f32 = 75.0;
const NOTE_HEIGHT: f32 = 18.0; // Height of one row (MIDI Note)
const BEAT_WIDTH: f32 = 100.0; // Width of one beat

const MIDI_BASE: u8 = 48;
const DEFAULT_LENGTH: Tick = 960; // TODO: calculate from PPQ


// --- MODEL & STATE ---
#[derive(Default, Copy, Clone)]
pub struct PendingNote {
    pub start: Tick,
    pub note: MidiNote, // Track the note being dragged
}

#[derive(Default)]
pub struct MidiEditorState {
    pub pending_note: Option<PendingNote>, // Track the note being dragged
}

pub struct MidiEditor {
    cache: canvas::Cache,
    scroll_offset: Vector, // Scroll state (x, y)
    // Add your MIDI note data structure here
    notes: BTreeMap<Tick, Vec<MidiNote>>,
    region_identifier: RegionIdentifier,
}

// Implement the Program trait for the editor
impl canvas::Program<Message, Theme> for MidiEditor {
    type State = MidiEditorState;

    fn draw(&self, state: &Self::State, renderer: &iced::Renderer,
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
            self.draw_midi_notes(frame, state);//, self.notes, &self.scroll_offset); 
        });

        vec![editor_geometry]
    }

    // You would implement update to handle mouse clicks (drawing notes) and scrolling
    // fn update(...) -> (Self::State, Option<Option<Message>>) { ... }
    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (Status, Option<Message>) {
        let cursor_position = if let Some(p) = cursor.position_in(bounds) {
            p
        } else {
            return (Status::Ignored, None);
        };

        // Calculate if the cursor is within the Grid area
        let grid_bounds = Rectangle::new(
            Point::new(KEYBOARD_WIDTH, RULER_HEIGHT),
            Size::new(bounds.width - KEYBOARD_WIDTH, bounds.height - RULER_HEIGHT),
        );

        match event {
            canvas::Event::Mouse(mouse_event) => match mouse_event {
                iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left) => {
                    if grid_bounds.contains(cursor_position) {
                        // info!("State pending note is {}", state.pending_note.is_some());
                        // 1. Calculate Pitch and Tick
                        // Note: scroll_offset.y is usually negative when scrolling down
                        let relative_y = cursor_position.y - RULER_HEIGHT - self.scroll_offset.y;
                        let pitch = (relative_y / NOTE_HEIGHT).floor() as u8;
                        
                        let relative_x = cursor_position.x - KEYBOARD_WIDTH - self.scroll_offset.x;
                        let start_tick = self.x_to_tick(relative_x);

                        // update internal state
                        state.pending_note = Some(PendingNote{ start: start_tick, note: MidiNote { channel: 0, key: pitch + MIDI_BASE, length: DEFAULT_LENGTH, velocity: 100 }});
                        // info!("Pending note set from {start_tick}");
                        // and return message to say all handled
                        return (
                            Status::Captured,
                            None,
                        );
                    }
                }

                iced::mouse::Event::CursorMoved { .. } => {
                    if let Some(pending) = &mut state.pending_note {
                        let relative_x = cursor_position.x - KEYBOARD_WIDTH - self.scroll_offset.x;
                        let current_tick = self.x_to_tick(relative_x);
                        
                        // update internal state
                        if current_tick > pending.start {
                            pending.note.length = current_tick - pending.start;
                        }
                        // and return message to say all handled
                        return (
                            Status::Captured,
                            None
                        );
                    }
                }

                iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left) => {
                    if let Some(pending) = &state.pending_note {
                        let final_note = pending.clone();
                        state.pending_note = None;
                        return (
                            Status::Captured,
                            Some(Message::CreateMidiNote (
                                self.region_identifier,
                                final_note.start,
                                final_note.note,
                            ))
                        );
                    }
                }
                _ => {}
            },
            _ => {}
        }

        (Status::Ignored, None)
    }
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
    fn draw_midi_notes(&self, frame: &mut Frame, state: &MidiEditorState) { //}, notes: BTreeMap<Tick, Vec<MidiNote>>, scroll: &Vector) { 
        if let Some(pending) = &state.pending_note {
            // info!("Drawing pending note at {},{}", pending.start_tick, pending.end_tick);
            self.draw_note(frame, pending.start, &pending.note);
        }
        for (start, notes) in &self.notes {
            for note in notes {
                self.draw_note(frame, *start, &note);
            }
        }
     }

    fn draw_note(&self, frame: &mut Frame, start: Tick, note: &MidiNote) {
        let x = self.tick_to_x(start);
        let width = self.tick_to_x(note.length);
        let y = (note.key - MIDI_BASE) as f32 * NOTE_HEIGHT;
        
        let rect = Path::rectangle(Point::new(x, y), Size::new(width.max(5.0), NOTE_HEIGHT));
        frame.fill(&rect, Color::from_rgba(0.0, 0.8, 1.0, 0.5));
        // Transparent blue ghost
    }
    
    // Helper functions
    const PPQ: u32 = 480;

    /// Converts a horizontal pixel coordinate (relative to grid start) to MIDI ticks
    fn x_to_tick(&self, x: f32) -> u32 {
        let beat = x / BEAT_WIDTH;
        (beat * Self::PPQ as f32) as u32
    }

    /// Converts MIDI ticks back to pixels for drawing
    fn tick_to_x(&self, tick: u32) -> f32 {
        (tick as f32 / Self::PPQ as f32) * BEAT_WIDTH
    }
}

// --- CONTAINER WIDGET ---
// This is what you actually put in your Iced layout.
impl MidiEditor {
    pub fn new(notes: BTreeMap<Tick, Vec<MidiNote>>, region_identifier: RegionIdentifier) -> Self {
        Self {
            cache: canvas::Cache::default(),
            scroll_offset: Vector::default(),
            notes,
            region_identifier
        }
    }

    pub fn view(notes: BTreeMap<Tick, Vec<MidiNote>>, region_identifier: RegionIdentifier) -> Canvas<Self, Message> {
        Canvas::new(MidiEditor::new(notes, region_identifier))
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
        let content = MidiEditor::view(midi_seq.notes.clone(), midi_seq.id);
        components::module(
            content
            .width(self.width)
            .height(self.height).into()
        ).into()
    }

    pub fn update(&self, msg: MidiEditorMessage) {
        match msg {
            crate::ui::actions::MidiEditorMessage::StartPendingNote(pitch, start) => todo!(),
            crate::ui::actions::MidiEditorMessage::UpdatePendingNote(duration) => todo!(),
        }

    }
}

