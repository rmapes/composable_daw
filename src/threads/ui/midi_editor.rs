use std::collections::{BTreeMap, HashSet};

use iced::mouse::Cursor;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme, Vector};
use super::super::engine::actions::Actions;
use crate::models::sequences::{MidiNote, MidiSeq, Tick};
use crate::models::shared::RegionIdentifier;

use super::actions::Message;

use super::components;

use iced::widget::{pick_list, row, text, container};

use iced::widget::canvas::{
    self, Canvas, Frame, Geometry, Path, Stroke, Text, LineCap, Style,
};

// 
// --- CONFIGURATION CONSTANTS ---
const BEATS_PER_BAR: u32 = 4;
const SUBDIVISIONS_PER_BEAT: u32 = 4; // Quarters of a beat

const RULER_HEIGHT: f32 = 25.0;
const KEYBOARD_WIDTH: f32 = 75.0;
const NOTE_HEIGHT: f32 = 18.0; // Height of one row (MIDI Note)
const BEAT_WIDTH: f32 = 100.0; // Width of one beat

const MIDI_BASE: u8 = 48;
const DEFAULT_LENGTH: Tick = 960; // TODO: calculate from PPQ

const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

// --- SNAP TO GRID ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapToGrid {
    None,
    Division,
    Beat,
    Bar,
}

impl std::fmt::Display for SnapToGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl SnapToGrid {
    pub const ALL: [SnapToGrid; 4] = [
        SnapToGrid::None,
        SnapToGrid::Division,
        SnapToGrid::Beat,
        SnapToGrid::Bar,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            SnapToGrid::None => "None",
            SnapToGrid::Division => "Division",
            SnapToGrid::Beat => "Beat",
            SnapToGrid::Bar => "Bar",
        }
    }

    /// Get the snap interval in ticks
    fn snap_interval(&self) -> Tick {
        const PPQ: u32 = 480;
        match self {
            SnapToGrid::None => 1, // No snapping, but use 1 to avoid division by zero
            SnapToGrid::Division => PPQ / SUBDIVISIONS_PER_BEAT, // 120 ticks
            SnapToGrid::Beat => PPQ, // 480 ticks
            SnapToGrid::Bar => PPQ * BEATS_PER_BAR, // 1920 ticks
        }
    }

    /// Snap a tick value to the nearest grid point
    pub fn snap_tick(&self, tick: Tick) -> Tick {
        if *self == SnapToGrid::None {
            return tick;
        }
        let interval = self.snap_interval();
        ((tick as f32 / interval as f32).round() as u32) * interval
    }
}

// --- MODEL & STATE ---
#[derive(Default, Copy, Clone)]
pub struct PendingNote {
    pub start: Tick,
    pub note: MidiNote, // Track the note being dragged
}

#[derive(Clone)]
pub struct DraggedNote {
    pub original_start: Tick,
    pub original_note_index: usize,
    pub current_start: Tick,
    pub note: MidiNote,
    pub is_resizing: bool, // true if resizing, false if moving
    pub click_offset_x: f32, // X offset from note start where user clicked (in ticks)
    pub click_offset_y: f32, // Y offset from note pitch where user clicked (in pitch units)
}

#[derive(Default)]
pub struct MidiEditorState {
    pub pending_note: Option<PendingNote>, // Track the note being created
    pub dragged_note: Option<DraggedNote>, // Track the note being moved or resized
    pub hovered_resize_edge: Option<(Tick, usize)>, // Track which note's right edge is being hovered
    pub selected_notes: HashSet<(Tick, usize)>, // Track selected notes (start_tick, note_index)
    pub shift_pressed: bool, // Track if shift key is currently pressed
}

pub struct MidiEditor {
    cache: canvas::Cache,
    scroll_offset: Vector, // Scroll state (x, y)
    // Add your MIDI note data structure here
    notes: BTreeMap<Tick, Vec<MidiNote>>,
    region_identifier: RegionIdentifier,
    snap_to_grid: SnapToGrid,
}

// Implement the Program trait for the editor
impl canvas::Program<Message, Theme> for MidiEditor {
    type State = MidiEditorState;

    fn draw(&self, state: &Self::State, renderer: &iced::Renderer,
        _theme: &Theme, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry> {
        // Clear cache when dragging/resizing or hovering resize edge to ensure smooth visual updates
        // This forces a redraw every frame during drag/resize
        if state.dragged_note.is_some() || state.hovered_resize_edge.is_some() {
            self.cache.clear();
        }
        
        let editor_geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            // 1. Draw the UI Sections
            let ruler_rect = Rectangle::new(Point::new(KEYBOARD_WIDTH, Point::ORIGIN.y), Size::new(bounds.width - KEYBOARD_WIDTH, RULER_HEIGHT));
            let keyboard_rect = Rectangle::new(
                Point::new(0.0, RULER_HEIGHT),
                Size::new(KEYBOARD_WIDTH, bounds.height - RULER_HEIGHT),
            );
            let grid_rect = Rectangle::new(
                Point::new(KEYBOARD_WIDTH, RULER_HEIGHT),
                Size::new(bounds.width - KEYBOARD_WIDTH, bounds.height - RULER_HEIGHT),
            );

            // Fill in background
            frame.fill(
                &Path::rectangle(Point::ORIGIN, frame.size()),
                Color::BLACK, // Dark background
            );

            // 2. Draw the Piano Keyboard
            frame.with_save(|frame| {
                frame.translate(Vector::new(0.0, RULER_HEIGHT));
                self.draw_keyboard(frame, &keyboard_rect, &self.scroll_offset);
            });

            // 3. Draw the Note Grid and Notes
            // We use a separate sub-frame for the grid to easily apply scroll offset
            frame.with_save(|frame| {
                frame.translate(Vector::new(KEYBOARD_WIDTH, RULER_HEIGHT));
                self.draw_grid(frame, &grid_rect.size(), &self.scroll_offset);
                self.draw_midi_notes(frame, state, &grid_rect);//, self.notes, &self.scroll_offset); 
            });

            // 4. Draw the Ruler
            self.draw_ruler(frame, &ruler_rect, &self.scroll_offset);
        });

        vec![editor_geometry]
    }

    // You would implement update to handle mouse clicks (drawing notes) and scrolling
    // fn update(...) -> (Self::State, Option<Option<Message>>) { ... }
    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<iced::widget::Action<Message>> {
        let cursor_position = cursor.position_in(bounds)?;
        
        // Handle keyboard events to track shift key state
        if let iced::Event::Keyboard(keyboard_event) = event {
            match keyboard_event {
                iced::keyboard::Event::KeyPressed { 
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Shift), 
                    .. 
                } => {
                    state.shift_pressed = true;
                }
                iced::keyboard::Event::KeyReleased { 
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Shift), 
                    .. 
                } => {
                    state.shift_pressed = false;
                }
                _ => {}
            }
        }

        // Calculate if the cursor is within the Grid area
        let grid_bounds = Rectangle::new(
            Point::new(KEYBOARD_WIDTH, RULER_HEIGHT),
            Size::new(bounds.width - KEYBOARD_WIDTH, bounds.height - RULER_HEIGHT),
        );

        if let iced::Event::Mouse(mouse_event) = event {
             match mouse_event {
                iced::mouse::Event::CursorMoved { .. } => {
                    // Only update hover state if we're not currently dragging/resizing
                    if state.dragged_note.is_none() && state.pending_note.is_none() {
                        // Update hover state for resize edge indicator
                        if grid_bounds.contains(cursor_position) {
                            state.hovered_resize_edge = self.find_note_resize_edge(
                                cursor_position.x,
                                cursor_position.y,
                                &grid_bounds
                            );
                            // Clear cache to redraw with hover indicator
                            if state.hovered_resize_edge.is_some() {
                                self.cache.clear();
                            }
                        } else {
                            state.hovered_resize_edge = None;
                        }
                    }
                    
                    // Handle dragging/resizing
                    if let Some(pending) = &mut state.pending_note {
                        // Creating a new note
                        let relative_x = cursor_position.x - KEYBOARD_WIDTH - self.scroll_offset.x;
                        let current_tick = self.snap_to_grid.snap_tick(self.x_to_tick(relative_x));
                        
                        // update internal state
                        if current_tick > pending.start {
                            pending.note.length = current_tick - pending.start;
                        }
                        // and return message to say all handled
                        return Some(iced::widget::Action::capture());
                    } else if let Some(dragged) = &mut state.dragged_note {
                        if dragged.is_resizing {
                            // Resizing an existing note - start position doesn't change
                            let relative_x = cursor_position.x - KEYBOARD_WIDTH - self.scroll_offset.x;
                            let end_tick = self.snap_to_grid.snap_tick(self.x_to_tick(relative_x));
                            
                            // Update the note length (ensure it's at least 1 tick)
                            // Use original_start because the start position shouldn't change when resizing
                            if end_tick > dragged.original_start {
                                dragged.note.length = end_tick - dragged.original_start;
                            } else {
                                dragged.note.length = 1;
                            }
                            
                            // Ensure current_start stays the same as original_start when resizing
                            // (start position should not change during resize)
                            dragged.current_start = dragged.original_start;
                            
                            // Clear cache to force redraw
                            self.cache.clear();
                            return Some(iced::widget::Action::capture());
                        } else {
                            // Moving an existing note - use relative movement from click position
                            let relative_x = cursor_position.x - KEYBOARD_WIDTH - self.scroll_offset.x;
                            let relative_y = cursor_position.y - RULER_HEIGHT - self.scroll_offset.y;
                            
                            // Calculate the new position based on mouse position minus the click offset
                            let mouse_tick = self.x_to_tick(relative_x);
                            let mouse_pitch = y_to_pitch(relative_y, &grid_bounds.size()) + MIDI_BASE;
                            
                            // Apply the offset: new position = mouse position - click offset
                            // This keeps the note aligned with where the user originally clicked
                            let new_start_tick = mouse_tick.saturating_sub(dragged.click_offset_x as u32);
                            let new_pitch_f32 = mouse_pitch as f32 - dragged.click_offset_y;
                            let new_pitch = new_pitch_f32.max(0.0).min(127.0) as u8; // Clamp to valid MIDI range
                            
                            // Snap the new position
                            let snapped_start_tick = self.snap_to_grid.snap_tick(new_start_tick);
                            let snapped_pitch = new_pitch; // Pitch snapping could be added here if needed
                            
                            // Update the dragged note's position
                            dragged.current_start = snapped_start_tick;
                            dragged.note.key = snapped_pitch;
                            
                            // Clear cache to force redraw
                            self.cache.clear();
                            return Some(iced::widget::Action::capture());
                        }
                    }
                }
                iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left) => {
                    if grid_bounds.contains(cursor_position) {
                        let relative_x = cursor_position.x - KEYBOARD_WIDTH - self.scroll_offset.x;
                        let relative_y = cursor_position.y - RULER_HEIGHT - self.scroll_offset.y;
                        
                        // First, check if we clicked on the resize edge of a note
                        if let Some((start_tick, note_index)) = self.find_note_resize_edge(
                            cursor_position.x,
                            cursor_position.y,
                            &grid_bounds
                        ) {
                            // Handle selection for resize edge click
                            let note_key = (start_tick, note_index);
                            if state.shift_pressed {
                                // Shift+click: add to selection (toggle if already selected)
                                if state.selected_notes.contains(&note_key) {
                                    state.selected_notes.remove(&note_key);
                                } else {
                                    state.selected_notes.insert(note_key);
                                }
                            } else {
                                // Regular click: replace selection
                                state.selected_notes.clear();
                                state.selected_notes.insert(note_key);
                            }
                            
                            // Clicked on resize edge - start resizing
                            if let Some(notes_at_tick) = self.notes.get(&start_tick) {
                                if note_index < notes_at_tick.len() {
                                    let note = notes_at_tick[note_index];
                                    state.dragged_note = Some(DraggedNote {
                                        original_start: start_tick,
                                        original_note_index: note_index,
                                        current_start: start_tick,
                                        note,
                                        is_resizing: true,
                                        click_offset_x: 0.0, // Not used for resizing
                                        click_offset_y: 0.0, // Not used for resizing
                                    });
                                    state.pending_note = None; // Clear any pending note creation
                                    state.hovered_resize_edge = None; // Clear hover state
                                    self.cache.clear(); // Redraw to show selection
                                    return Some(iced::widget::Action::capture());
                                }
                            }
                        }
                        // Then check if we clicked on an existing note (for moving)
                        else if let Some((start_tick, note_index)) = self.find_note_at_position(
                            cursor_position.x, 
                            cursor_position.y, 
                            &grid_bounds
                        ) {
                            // Handle selection
                            let note_key = (start_tick, note_index);
                            if state.shift_pressed {
                                // Shift+click: add to selection (toggle if already selected)
                                if state.selected_notes.contains(&note_key) {
                                    state.selected_notes.remove(&note_key);
                                } else {
                                    state.selected_notes.insert(note_key);
                                }
                            } else {
                                // Regular click: replace selection
                                state.selected_notes.clear();
                                state.selected_notes.insert(note_key);
                            }
                            
                            // Clicked on an existing note - start dragging it
                            if let Some(notes_at_tick) = self.notes.get(&start_tick) {
                                if note_index < notes_at_tick.len() {
                                    let note = notes_at_tick[note_index];
                                    
                                    // Calculate the offset from the note's start position where user clicked
                                    // Convert click position to ticks and pitch
                                    let click_tick = self.x_to_tick(relative_x);
                                    let click_pitch = y_to_pitch(relative_y, &grid_bounds.size()) + MIDI_BASE;
                                    
                                    // Calculate offset: how far into the note the user clicked
                                    let click_offset_x_ticks = click_tick.saturating_sub(start_tick);
                                    let click_offset_y_pitch = (click_pitch as i16 - note.key as i16) as f32;
                                    
                                    state.dragged_note = Some(DraggedNote {
                                        original_start: start_tick,
                                        original_note_index: note_index,
                                        current_start: start_tick,
                                        note,
                                        is_resizing: false,
                                        click_offset_x: click_offset_x_ticks as f32,
                                        click_offset_y: click_offset_y_pitch,
                                    });
                                    state.pending_note = None; // Clear any pending note creation
                                    state.hovered_resize_edge = None; // Clear hover state
                                    self.cache.clear(); // Redraw to show selection
                                    return Some(iced::widget::Action::capture());
                                }
                            }
                        } else {
                            // Clicked on empty space - clear selection unless shift is held
                            if !state.shift_pressed {
                                state.selected_notes.clear();
                                self.cache.clear(); // Redraw to update selection
                            }
                            // Not clicking on an existing note - start creating a new one
                            let pitch = y_to_pitch(relative_y, &grid_bounds.size());
                            let start_tick = self.snap_to_grid.snap_tick(self.x_to_tick(relative_x));

                            // update internal state
                            state.pending_note = Some(PendingNote{ start: start_tick, note: MidiNote { channel: 0, key: pitch + MIDI_BASE, length: DEFAULT_LENGTH, velocity: 100 }});
                            state.dragged_note = None; // Clear any dragged note
                            state.hovered_resize_edge = None; // Clear hover state
                            // and return message to say all handled
                            return Some(iced::widget::Action::capture());
                        }
                    }
                }

                iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left) => {
                    if let Some(pending) = &state.pending_note {
                        // Finishing creating a new note
                        let final_note = *pending;
                        state.pending_note = None;
                        return Some(iced::widget::Action::publish(Message::Engine(Actions::CreateMidiNote (
                            self.region_identifier,
                            final_note.start,
                            final_note.note,
                        ))));
                    } else if let Some(dragged) = &state.dragged_note {
                        // Finishing moving or resizing an existing note
                        let dragged_note = dragged.clone();
                        state.dragged_note = None;
                        
                        // Check if anything changed
                        let original_note = self.notes.get(&dragged_note.original_start)
                            .and_then(|notes| notes.get(dragged_note.original_note_index));
                        
                        let position_changed = dragged_note.current_start != dragged_note.original_start || 
                           dragged_note.note.key != original_note.map(|n| n.key).unwrap_or(0);
                        let length_changed = dragged_note.is_resizing && 
                           dragged_note.note.length != original_note.map(|n| n.length).unwrap_or(0);
                        
                        // If position or length changed, send update action
                        if position_changed || length_changed {
                            return Some(iced::widget::Action::publish(Message::Engine(Actions::UpdateMidiNote (
                                self.region_identifier,
                                dragged_note.original_start,
                                dragged_note.original_note_index,
                                dragged_note.current_start,
                                dragged_note.note,
                            ))));
                        }
                    }
                }
                _ => {}
            }
        }

        None
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
            let x = beat as f32 * BEAT_WIDTH + scroll.x + bounds.position().x;

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
        // frame.with_clip(*bounds, |frame| {

            let visible_notes = (bounds.size().height / NOTE_HEIGHT).floor() as u8;
            let start_note_index = (-scroll.y / NOTE_HEIGHT).floor() as u8;
            let end_note_index = start_note_index + visible_notes + 1;

            for i in start_note_index..end_note_index {
                let pitch = i + MIDI_BASE; 
                let y = pitch_to_y(&pitch, &bounds.size());
                let is_sharp = match pitch % 12 {
                    1 | 3 | 6 | 8 | 10 => true, // C#, D#, F#, G#, A#
                    _ => false,
                };
                let is_octave_c = pitch.is_multiple_of(12); // C notes

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
                if !is_sharp {
                    let pitch_index = pitch % 12;
                    let octave = (pitch / 12) as i32 - 1; // MIDI note 12 is C0
                    let label = format!("{}{}", NOTE_NAMES[pitch_index as usize], octave);
                    
                    let text = Text {
                        content: label,
                        position: Point::new(
                            bounds.x + 3.0 * KEYBOARD_WIDTH / 4.0, // Right align on the key
                            y + (NOTE_HEIGHT / 2.0) - 4.0        // Centered vertically
                        ),
                        color: Color::from_rgb(0.5, 0.5, 0.5),
                        size: iced::Pixels::from(10.0),
                        ..Text::default()
                    };
        
                    frame.fill_text(text);
                }
            }
        // })
        // });
    }

    // 3. Draw the Note Grid
    fn draw_grid(&self, frame: &mut Frame, bounds: &Size, scroll: &Vector) {
        // Apply scroll offset to the grid drawing
        // frame.translate(*scroll);

        let total_time_span = 16_f32 * BEAT_WIDTH; // Example: 16 beats

        let visible_notes = (bounds.height / NOTE_HEIGHT).floor() as u8;
        let num_rows = visible_notes + 1;

        // Draw horizontal note lines
        for i in 0..num_rows {
            let y = pitch_to_y(&(i + MIDI_BASE), bounds);
            let line = Path::line(Point::new(0.0, y), Point::new(total_time_span, y));

            let is_octave_c_row = (i + (-scroll.y / NOTE_HEIGHT).round() as u8).is_multiple_of(12);

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
    fn draw_midi_notes(&self, frame: &mut Frame, state: &MidiEditorState, bounds: &Rectangle) { //}, notes: BTreeMap<Tick, Vec<MidiNote>>, scroll: &Vector) { 
        // Draw existing notes, but skip the one being dragged/resized
        if let Some(dragged) = &state.dragged_note {
            for (start, notes) in &self.notes {
                for (note_index, note) in notes.iter().enumerate() {
                    // Skip drawing the note that's being dragged/resized
                    if *start == dragged.original_start && note_index == dragged.original_note_index {
                        continue;
                    }
                    // Draw selected notes with brighter color
                    if state.selected_notes.contains(&(*start, note_index)) {
                        self.draw_note_selected(frame, *start, note, bounds);
                    } else {
                        self.draw_note(frame, *start, note, bounds);
                    }
                }
            }
            // Draw the dragged/resized note at its current position
            // If it was selected, keep it bright
            let was_selected = state.selected_notes.contains(&(dragged.original_start, dragged.original_note_index));
            if was_selected {
                self.draw_note_with_color(frame, dragged.current_start, &dragged.note, bounds, Color::from_rgba(0.2, 1.0, 1.0, 0.9));
            } else {
                self.draw_note_with_color(frame, dragged.current_start, &dragged.note, bounds, Color::from_rgba(0.0, 0.8, 1.0, 0.7));
            }
        } else {
            // No note being dragged, draw all notes normally
            for (start, notes) in &self.notes {
                for (note_index, note) in notes.iter().enumerate() {
                    // Draw selected notes with brighter color
                    if state.selected_notes.contains(&(*start, note_index)) {
                        self.draw_note_selected(frame, *start, note, bounds);
                    } else {
                        self.draw_note(frame, *start, note, bounds);
                    }
                    
                    // Draw resize indicator on right edge if hovering
                    if let Some((hovered_start, hovered_index)) = state.hovered_resize_edge {
                        if *start == hovered_start && note_index == hovered_index {
                            self.draw_resize_indicator(frame, *start, note, bounds);
                        }
                    }
                }
            }
        }
        
        // Draw pending note (being created)
        if let Some(pending) = &state.pending_note {
            self.draw_note(frame, pending.start, &pending.note, bounds);
        }
     }

    /// Draw a resize indicator (vertical line) on the right edge of a note
    fn draw_resize_indicator(&self, frame: &mut Frame, start: Tick, note: &MidiNote, bounds: &Rectangle) {
        let x = self.tick_to_x(start) + self.tick_to_x(note.length);
        let y = pitch_to_y(&note.key, &bounds.size());
        
        // Draw a vertical line on the right edge
        let line = Path::line(
            Point::new(x, y),
            Point::new(x, y + NOTE_HEIGHT)
        );
        frame.stroke(
            &line,
            Stroke {
                style: Style::Solid(Color::from_rgba(1.0, 1.0, 1.0, 0.8)),
                width: 2.0,
                line_cap: LineCap::Square,
                ..Stroke::default()
            }
        );
    }

    fn draw_note(&self, frame: &mut Frame, start: Tick, note: &MidiNote, bounds: &Rectangle) {
        self.draw_note_with_color(frame, start, note, bounds, Color::from_rgba(0.0, 0.8, 1.0, 0.5));
    }

    fn draw_note_selected(&self, frame: &mut Frame, start: Tick, note: &MidiNote, bounds: &Rectangle) {
        // Brighter color for selected notes
        self.draw_note_with_color(frame, start, note, bounds, Color::from_rgba(0.2, 1.0, 1.0, 0.8));
    }

    fn draw_note_with_color(&self, frame: &mut Frame, start: Tick, note: &MidiNote, bounds: &Rectangle, color: Color) {
        let x = self.tick_to_x(start);
        let width = self.tick_to_x(note.length);
        let y = pitch_to_y(&note.key, &bounds.size());
        
        let rect = Path::rectangle(Point::new(x, y), Size::new(width.max(5.0), NOTE_HEIGHT));
        frame.fill(&rect, color);
    }

    const RESIZE_EDGE_THRESHOLD: f32 = 8.0; // Pixels from right edge to trigger resize

    /// Find which note (if any) is at the given cursor position
    /// Returns (start_tick, note_index) if found
    /// Excludes the resize edge area to avoid conflicts with resize detection
    fn find_note_at_position(&self, cursor_x: f32, cursor_y: f32, bounds: &Rectangle) -> Option<(Tick, usize)> {
        let relative_x = cursor_x - KEYBOARD_WIDTH - self.scroll_offset.x;
        let relative_y = cursor_y - RULER_HEIGHT - self.scroll_offset.y;
        
        let pitch = y_to_pitch(relative_y, &bounds.size()) + MIDI_BASE;

        // Check all notes to find one that contains this position
        for (start_tick, notes) in &self.notes {
            for (note_index, note) in notes.iter().enumerate() {
                if note.key == pitch {
                    let note_start_x = self.tick_to_x(*start_tick);
                    let note_end_x = note_start_x + self.tick_to_x(note.length);
                    let note_y = pitch_to_y(&note.key, &bounds.size());
                    
                    // Check if cursor is within note bounds, but exclude the resize edge area
                    // This prevents conflicts with resize detection
                    let resize_edge_start = note_end_x - Self::RESIZE_EDGE_THRESHOLD;
                    if relative_x >= note_start_x && relative_x < resize_edge_start &&
                       relative_y >= note_y && relative_y <= note_y + NOTE_HEIGHT {
                        return Some((*start_tick, note_index));
                    }
                }
            }
        }
        None
    }

    /// Check if cursor is over the right edge of a note (for resizing)
    /// Returns (start_tick, note_index) if over resize edge
    fn find_note_resize_edge(&self, cursor_x: f32, cursor_y: f32, bounds: &Rectangle) -> Option<(Tick, usize)> {
        let relative_x = cursor_x - KEYBOARD_WIDTH - self.scroll_offset.x;
        let relative_y = cursor_y - RULER_HEIGHT - self.scroll_offset.y;
        
        let pitch = y_to_pitch(relative_y, &bounds.size()) + MIDI_BASE;

        // Check all notes to find one whose right edge is being hovered
        for (start_tick, notes) in &self.notes {
            for (note_index, note) in notes.iter().enumerate() {
                if note.key == pitch {
                    let note_start_x = self.tick_to_x(*start_tick);
                    let note_end_x = note_start_x + self.tick_to_x(note.length);
                    let note_y = pitch_to_y(&note.key, &bounds.size());
                    
                    // Check if cursor is near the right edge and within vertical bounds
                    let distance_from_edge = note_end_x - relative_x;
                    if distance_from_edge >= 0.0 && distance_from_edge <= Self::RESIZE_EDGE_THRESHOLD &&
                       relative_y >= note_y && relative_y <= note_y + NOTE_HEIGHT {
                        return Some((*start_tick, note_index));
                    }
                }
            }
        }
        None
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

fn pitch_to_y(key: &u8, bounds: &Size) -> f32 {
    let y = (key - MIDI_BASE) as f32 * NOTE_HEIGHT;
    bounds.height - y
}

fn y_to_pitch(relative_y: f32, bounds: &Size) -> u8 {
    ((bounds.height - relative_y) / NOTE_HEIGHT).ceil() as u8
}


// --- CONTAINER WIDGET ---
// This is what you actually put in your Iced layout.
impl MidiEditor {
    pub fn new(notes: BTreeMap<Tick, Vec<MidiNote>>, region_identifier: RegionIdentifier, snap_to_grid: SnapToGrid) -> Self {
        Self {
            cache: canvas::Cache::default(),
            scroll_offset: Vector::default(),
            notes,
            region_identifier,
            snap_to_grid,
        }
    }

    pub fn view(notes: BTreeMap<Tick, Vec<MidiNote>>, region_identifier: RegionIdentifier, snap_to_grid: SnapToGrid) -> Canvas<Self, Message> {
        Canvas::new(MidiEditor::new(notes, region_identifier, snap_to_grid))
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

    pub fn view(&self, midi_seq: &MidiSeq, snap_to_grid: SnapToGrid) -> Element<'_, Message> {
        let canvas = MidiEditor::view(midi_seq.notes.clone(), midi_seq.id, snap_to_grid);
        
        // Create snap-to-grid selector in top right
        let snap_selector = row![
            text("Snap:").size(12),
            pick_list(
                SnapToGrid::ALL,
                Some(snap_to_grid),
                move |snap| Message::MidiEditor(MidiEditorMessage::SetSnapToGrid(snap))
            )
        ]
        .spacing(5);

        // Use a column with the selector at the top right, then the canvas
        let content = iced::widget::column![
            container(snap_selector)
                .padding(5)
                .align_x(iced::alignment::Horizontal::Right)
                .width(Length::Fill),
            components::module(
                canvas
                    .width(self.width)
                    .height(self.height)
                    .into()
            ).id("MidiEditor")
        ]
        .width(self.width)
        .height(self.height);

        content.into()
    }

}

#[derive(Debug, Clone)]
pub enum MidiEditorMessage {
    SetSnapToGrid(SnapToGrid),
}

