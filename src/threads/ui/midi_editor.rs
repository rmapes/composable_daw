use std::collections::{BTreeMap, HashSet};

use iced::mouse::{Cursor, ScrollDelta};
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

const DEFAULT_LENGTH: Tick = 960; // TODO: calculate from PPQ
const DRAG_EDGE_SCROLL_THRESHOLD: f32 = 2.0 * NOTE_HEIGHT; // Pixels from top/bottom to trigger auto-scroll
const DRAG_EDGE_SCROLL_THROTTLE: u8 = 3; // Only scroll every Nth CursorMoved at edge

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
    pub pending_update: Option<(Tick, usize, Tick, MidiNote)>, // Track pending note update (old_start, note_index, new_start, note) to show at new position
    pub click_start_position: Option<(f32, f32)>, // Track initial click position for debounce (x, y)
    pub click_start_note: Option<(Tick, usize)>, // Track which note was clicked for debounce
    pub drag_edge_scroll_counter: u8, // Throttle for auto-scroll when dragging note to edge
}

pub struct MidiEditor {
    cache: canvas::Cache,
    scroll_offset: Vector, // Scroll state (x, y) for horizontal
    midi_offset: u8,      // Lowest visible MIDI note at bottom of grid (replaces MIDI_BASE)
    notes: BTreeMap<Tick, Vec<MidiNote>>,
    region_identifier: RegionIdentifier,
    snap_to_grid: SnapToGrid,
}

// Implement the Program trait for the editor
impl canvas::Program<Message, Theme> for MidiEditor {
    type State = MidiEditorState;

    fn draw(&self, state: &Self::State, renderer: &iced::Renderer,
        _theme: &Theme, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry> {
        // Clear cache when dragging/resizing, hovering resize edge, or pending update to ensure smooth visual updates
        // This forces a redraw every frame during drag/resize and while waiting for update
        if state.dragged_note.is_some() || state.hovered_resize_edge.is_some() || state.pending_update.is_some() {
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
                self.draw_keyboard(frame, &keyboard_rect, &self.scroll_offset, self.midi_offset);
            });

            // 3. Draw the Note Grid and Notes
            // We use a separate sub-frame for the grid to easily apply scroll offset
            frame.with_save(|frame| {
                frame.translate(Vector::new(KEYBOARD_WIDTH, RULER_HEIGHT));
                self.draw_grid(frame, &grid_rect.size(), &self.scroll_offset, self.midi_offset);
                self.draw_midi_notes(frame, state, &grid_rect, self.midi_offset);
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
        // Handle keyboard events first (don't need cursor position)
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
                iced::keyboard::Event::KeyPressed { 
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::PageUp), 
                    .. 
                } => {
                    // Logic Pro: Page Up = scroll to see higher notes
                    self.cache.clear();
                    return Some(iced::widget::Action::publish(Message::MidiEditor(
                        MidiEditorMessage::ScrollPitch(12), // One octave up
                    )));
                }
                iced::keyboard::Event::KeyPressed { 
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::PageDown), 
                    .. 
                } => {
                    // Logic Pro: Page Down = scroll to see lower notes
                    self.cache.clear();
                    return Some(iced::widget::Action::publish(Message::MidiEditor(
                        MidiEditorMessage::ScrollPitch(-12), // One octave down
                    )));
                }
                iced::keyboard::Event::KeyPressed { 
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace), 
                    .. 
                } => {
                    // Delete all selected notes
                    if !state.selected_notes.is_empty() {
                        // Collect selected notes into a vector (we need to clone because we'll be modifying the HashSet)
                        let selected_notes: Vec<(Tick, usize)> = state.selected_notes.iter().cloned().collect();
                        
                        // Clear selection
                        state.selected_notes.clear();
                        
                        // Clear cache to update display
                        self.cache.clear();
                        
                        // Send batch delete action
                        return Some(iced::widget::Action::publish(Message::Engine(
                            Actions::DeleteMultipleMidiNotes(
                                self.region_identifier,
                                selected_notes,
                            )
                        )));
                    }
                }
                _ => {}
            }
            return None; // Keyboard event handled (or no action needed)
        }

        // Mouse events require cursor position
        let cursor_position = match cursor.position_in(bounds) {
            Some(p) => p,
            None => return None,
        };

        // Calculate if the cursor is within the Grid area
        let grid_bounds = Rectangle::new(
            Point::new(KEYBOARD_WIDTH, RULER_HEIGHT),
            Size::new(bounds.width - KEYBOARD_WIDTH, bounds.height - RULER_HEIGHT),
        );

        if let iced::Event::Mouse(mouse_event) = event {
             match mouse_event {
                iced::mouse::Event::CursorMoved { .. } => {
                    // Check if pending update has been applied and clear it
                    // We check on every cursor move to detect updates quickly
                    if let Some((old_start, note_index, new_start, note)) = &state.pending_update {
                        // Check if the note at the old position is gone or changed
                        // (indicating update has been applied)
                        let old_note_matches = self.notes.get(old_start)
                            .and_then(|notes| notes.get(*note_index))
                            .map(|n| n.key == note.key && n.length == note.length && n.velocity == note.velocity)
                            .unwrap_or(false);
                        
                        // If the old note doesn't match (or is gone), the update has likely been applied
                        // We can clear the pending update - the note should now be in self.notes at the new position
                        if !old_note_matches {
                            // Update selection to the actual note position if it was selected
                            let old_key = (*old_start, *note_index);
                            if state.selected_notes.remove(&old_key) {
                                // Find the note at the new position by matching key and length
                                if let Some(notes_at_new_tick) = self.notes.get(new_start) {
                                    if let Some((actual_index, _)) = notes_at_new_tick.iter()
                                        .enumerate()
                                        .find(|(_, n)| n.key == note.key && n.length == note.length && n.velocity == note.velocity) {
                                        state.selected_notes.insert((*new_start, actual_index));
                                    }
                                }
                            }
                            state.pending_update = None;
                            self.cache.clear();
                        }
                    }
                    
                    // Handle debounce: check if mouse has moved enough to start dragging
                    if let (Some((click_x, click_y)), Some((click_start_tick, click_note_index))) = 
                        (state.click_start_position, state.click_start_note) {
                        if state.dragged_note.is_none() && state.pending_note.is_none() {
                            // Calculate distance moved
                            let dx = cursor_position.x - click_x;
                            let dy = cursor_position.y - click_y;
                            let distance = (dx * dx + dy * dy).sqrt();
                            
                            const DRAG_THRESHOLD: f32 = 3.0; // Pixels - minimum movement to start drag
                            
                            if distance > DRAG_THRESHOLD {
                                // Mouse moved enough - start dragging
                                if let Some(notes_at_tick) = self.notes.get(&click_start_tick) {
                                    if click_note_index < notes_at_tick.len() {
                                        let note = notes_at_tick[click_note_index];
                        let relative_x = cursor_position.x - KEYBOARD_WIDTH - self.scroll_offset.x;
                                        let relative_y = cursor_position.y - RULER_HEIGHT - self.scroll_offset.y;
                                        
                                        // Calculate the offset from the note's start position where user clicked
                                        let click_tick = self.x_to_tick(relative_x);
                                        let click_pitch = y_to_pitch(relative_y, &grid_bounds.size(), self.midi_offset);
                                        
                                        // Calculate offset: how far into the note the user clicked
                                        let click_offset_x_ticks = click_tick.saturating_sub(click_start_tick);
                                        let click_offset_y_pitch = (click_pitch as i16 - note.key as i16) as f32;
                                        
                                        state.dragged_note = Some(DraggedNote {
                                            original_start: click_start_tick,
                                            original_note_index: click_note_index,
                                            current_start: click_start_tick,
                                            note,
                                            is_resizing: false,
                                            click_offset_x: click_offset_x_ticks as f32,
                                            click_offset_y: click_offset_y_pitch,
                                        });
                                        // Clear click tracking now that we're dragging
                                        state.click_start_position = None;
                                        state.click_start_note = None;
                                        self.cache.clear();
                        return Some(iced::widget::Action::capture());
                                    }
                                }
                            }
                        }
                    }
                    
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
                            let grid_height = grid_bounds.size().height;
                            
                            // Calculate the new position based on mouse position minus the click offset
                            let mouse_tick = self.x_to_tick(relative_x);
                            let mouse_pitch = y_to_pitch(relative_y, &grid_bounds.size(), self.midi_offset);
                            
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
                            
                            // Auto-scroll when note is dragged to top or bottom of visible range
                            let at_top = relative_y < DRAG_EDGE_SCROLL_THRESHOLD;
                            let at_bottom = relative_y > grid_height - DRAG_EDGE_SCROLL_THRESHOLD;
                            let scroll_delta = if at_top && self.midi_offset < 127 {
                                state.drag_edge_scroll_counter = state.drag_edge_scroll_counter.saturating_add(1);
                                if state.drag_edge_scroll_counter >= DRAG_EDGE_SCROLL_THROTTLE {
                                    state.drag_edge_scroll_counter = 0;
                                    Some(1i16) // Scroll up to see higher notes
                                } else {
                                    None
                                }
                            } else if at_bottom && self.midi_offset > 0 {
                                state.drag_edge_scroll_counter = state.drag_edge_scroll_counter.saturating_add(1);
                                if state.drag_edge_scroll_counter >= DRAG_EDGE_SCROLL_THROTTLE {
                                    state.drag_edge_scroll_counter = 0;
                                    Some(-1i16) // Scroll down to see lower notes
                                } else {
                                    None
                                }
                            } else {
                                state.drag_edge_scroll_counter = 0;
                                None
                            };
                            
                            // Clear cache to force redraw
                            self.cache.clear();
                            return Some(if let Some(delta) = scroll_delta {
                                iced::widget::Action::publish(Message::MidiEditor(
                                    MidiEditorMessage::ScrollPitch(delta),
                                ))
                                .and_capture()
                            } else {
                                iced::widget::Action::capture()
                            });
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
                            // Handle selection immediately on click
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
                            
                            // Store click position for debounce - don't start dragging immediately
                            // Dragging will start in CursorMoved if mouse moves beyond threshold
                            state.click_start_position = Some((cursor_position.x, cursor_position.y));
                            state.click_start_note = Some((start_tick, note_index));
                            state.pending_note = None; // Clear any pending note creation
                            state.hovered_resize_edge = None; // Clear hover state
                            self.cache.clear(); // Redraw to show selection
                            return Some(iced::widget::Action::capture());
                        } else {
                            // Clicked on empty space - clear selection unless shift is held
                            if !state.shift_pressed {
                                state.selected_notes.clear();
                                self.cache.clear(); // Redraw to update selection
                            }
                            // Not clicking on an existing note - start creating a new one
                            let pitch = y_to_pitch(relative_y, &grid_bounds.size(), self.midi_offset);
                            let start_tick = self.snap_to_grid.snap_tick(self.x_to_tick(relative_x));

                            // update internal state
                            state.pending_note = Some(PendingNote{ start: start_tick, note: MidiNote { channel: 0, key: pitch, length: DEFAULT_LENGTH, velocity: 100 }});
                            state.dragged_note = None; // Clear any dragged note
                            state.hovered_resize_edge = None; // Clear hover state
                            // and return message to say all handled
                            return Some(iced::widget::Action::capture());
                        }
                    }
                }

                iced::mouse::Event::WheelScrolled { delta } => {
                    // Logic Pro: mouse wheel (no modifier) = vertical scroll of pitch range
                    // Positive y = scroll up (see lower notes), negative y = scroll down (see higher notes)
                    let delta_y = match delta {
                        ScrollDelta::Lines { y, .. } => -*y as i16, // 1 line ≈ 1 semitone
                        ScrollDelta::Pixels { y, .. } => -((*y / NOTE_HEIGHT).round() as i16),
                    };
                    if delta_y != 0 {
                        self.cache.clear();
                        return Some(iced::widget::Action::publish(Message::MidiEditor(
                            MidiEditorMessage::ScrollPitch(delta_y),
                        )));
                    }
                }
                iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left) => {
                    // Handle click without drag (selection only)
                    if let (Some((click_x, click_y)), Some(_click_note)) = 
                        (state.click_start_position, state.click_start_note) {
                        if state.dragged_note.is_none() {
                            // Check if mouse moved significantly
                            let dx = cursor_position.x - click_x;
                            let dy = cursor_position.y - click_y;
                            let distance = (dx * dx + dy * dy).sqrt();
                            
                            const DRAG_THRESHOLD: f32 = 3.0; // Same threshold as in CursorMoved
                            
                            if distance <= DRAG_THRESHOLD {
                                // Click without drag - selection already handled in ButtonPressed
                                // Just clear the click tracking
                                state.click_start_position = None;
                                state.click_start_note = None;
                                return None; // Let selection update be handled
                            }
                        }
                    }
                    
                    // Clear click tracking on release
                    state.click_start_position = None;
                    state.click_start_note = None;
                    
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
                        
                        // Check if anything changed
                        let original_note = self.notes.get(&dragged_note.original_start)
                            .and_then(|notes| notes.get(dragged_note.original_note_index));
                        
                        let position_changed = dragged_note.current_start != dragged_note.original_start || 
                           dragged_note.note.key != original_note.map(|n| n.key).unwrap_or(0);
                        let length_changed = dragged_note.is_resizing && 
                           dragged_note.note.length != original_note.map(|n| n.length).unwrap_or(0);
                        
                        // If position or length changed, store pending update and send action
                        if position_changed || length_changed {
                            // Update selection to new position if note was selected
                            let old_note_key = (dragged_note.original_start, dragged_note.original_note_index);
                            if state.selected_notes.remove(&old_note_key) {
                                // Note was selected - find the note at the new position and update selection
                                // Since notes are stored by start_tick, we need to find the matching note
                                if let Some(notes_at_new_tick) = self.notes.get(&dragged_note.current_start) {
                                    // Try to find the note by matching key and length
                                    if let Some((new_index, _)) = notes_at_new_tick.iter()
                                        .enumerate()
                                        .find(|(_, n)| n.key == dragged_note.note.key && n.length == dragged_note.note.length) {
                                        state.selected_notes.insert((dragged_note.current_start, new_index));
                                    } else {
                                        // Fallback: use original index if available
                                        if dragged_note.original_note_index < notes_at_new_tick.len() {
                                            state.selected_notes.insert((dragged_note.current_start, dragged_note.original_note_index));
                                        }
                                    }
                                } else {
                                    // New position doesn't exist yet - will be created by the update
                                    // Store selection for new position (will be validated when update is applied)
                                    state.selected_notes.insert((dragged_note.current_start, dragged_note.original_note_index));
                                }
                            }
                            
                            // Store the pending update BEFORE clearing dragged_note to avoid visual gap
                            state.pending_update = Some((
                                dragged_note.original_start,
                                dragged_note.original_note_index,
                                dragged_note.current_start,
                                dragged_note.note,
                            ));
                            
                            // Clear cache to redraw with pending update (before clearing dragged_note)
                            self.cache.clear();
                            
                            // Clear dragged state now that we have pending update
                            state.dragged_note = None;
                            state.drag_edge_scroll_counter = 0;
                            
                            return Some(iced::widget::Action::publish(Message::Engine(Actions::UpdateMidiNote (
                                self.region_identifier,
                                dragged_note.original_start,
                                dragged_note.original_note_index,
                                dragged_note.current_start,
                                dragged_note.note,
                            ))));
                        } else {
                            // Nothing changed, just clear dragged state
                            state.dragged_note = None;
                            state.drag_edge_scroll_counter = 0;
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

    // 2. Draw the Piano Keyboard (scrolls with grid via midi_offset)
    fn draw_keyboard(&self, frame: &mut Frame, bounds: &Rectangle, _scroll: &Vector, midi_offset: u8) {
        frame.fill(
            &Path::rectangle(bounds.position(), bounds.size()),
            Color::from_rgb(0.2, 0.2, 0.2), // Keyboard background
        );

            let visible_notes = (bounds.size().height / NOTE_HEIGHT).floor() as u8;
            let end_pitch = (midi_offset as u16 + visible_notes as u16 + 1).min(128) as u8;

            for pitch in midi_offset..end_pitch {
                let y = pitch_to_y(pitch, &bounds.size(), midi_offset);
                let is_sharp = match pitch % 12 {
                    1 | 3 | 6 | 8 | 10 => true, // C#, D#, F#, G#, A#
                    _ => false,
                };
                let is_octave_c = pitch % 12 == 0; // C notes

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

    // 3. Draw the Note Grid (scrolls with midi_offset)
    fn draw_grid(&self, frame: &mut Frame, bounds: &Size, _scroll: &Vector, midi_offset: u8) {
        let total_time_span = 16_f32 * BEAT_WIDTH; // Example: 16 beats

        let visible_notes = (bounds.height / NOTE_HEIGHT).floor() as u8;
        let num_rows = visible_notes + 1;

        // Draw horizontal note lines (from midi_offset upward)
        for i in 0..num_rows {
            let pitch = midi_offset + i;
            if pitch > 127 {
                break;
            }
            let y = pitch_to_y(pitch, bounds, midi_offset);
            let line = Path::line(Point::new(0.0, y), Point::new(total_time_span, y));

            let is_octave_c_row = pitch % 12 == 0;

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
    // 4. draw_midi_notes
    fn draw_midi_notes(&self, frame: &mut Frame, state: &MidiEditorState, bounds: &Rectangle, midi_offset: u8) { 
        // Determine which note to skip (being dragged or pending update)
        let skip_note = if let Some(dragged) = &state.dragged_note {
            Some((dragged.original_start, dragged.original_note_index))
        } else if let Some((old_start, note_index, _, _)) = &state.pending_update {
            Some((*old_start, *note_index))
        } else {
            None
        };
        
        // Draw all notes except the one being dragged or pending update
        for (start, notes) in &self.notes {
            for (note_index, note) in notes.iter().enumerate() {
                // Skip drawing the note that's being dragged/resized or has pending update
                if let Some((skip_start, skip_idx)) = skip_note {
                    if *start == skip_start && note_index == skip_idx {
                        continue;
                    }
                }
                // Draw selected notes with brighter color
                if state.selected_notes.contains(&(*start, note_index)) {
                    self.draw_note_selected(frame, *start, note, bounds, midi_offset);
                } else {
                    self.draw_note(frame, *start, note, bounds, midi_offset);
                }
            }
        }
        
        // Draw the dragged/resized note at its current position
        if let Some(dragged) = &state.dragged_note {
            let was_selected = state.selected_notes.contains(&(dragged.original_start, dragged.original_note_index));
            if was_selected {
                self.draw_note_with_color(frame, dragged.current_start, &dragged.note, bounds, Color::from_rgba(0.2, 1.0, 1.0, 0.9), midi_offset);
            } else {
                self.draw_note_with_color(frame, dragged.current_start, &dragged.note, bounds, Color::from_rgba(0.0, 0.8, 1.0, 0.7), midi_offset);
            }
        }
        // Draw pending update note at its new position (until actual update comes through)
        // Always draw it to avoid visual gaps - it will be cleared when update is confirmed
        else if let Some((_old_start, note_index, new_start, note)) = &state.pending_update {
            // Check if the note is selected at the new position (selection was updated in ButtonReleased)
            let is_selected = state.selected_notes.contains(&(*new_start, *note_index));
            if is_selected {
                self.draw_note_with_color(frame, *new_start, note, bounds, Color::from_rgba(0.2, 1.0, 1.0, 0.9), midi_offset);
            } else {
                self.draw_note_with_color(frame, *new_start, note, bounds, Color::from_rgba(0.0, 0.8, 1.0, 0.7), midi_offset);
            }
        }
        
        // Draw resize indicator on right edge if hovering (only when not dragging)
        if state.dragged_note.is_none() && state.pending_update.is_none() {
            for (start, notes) in &self.notes {
                for (note_index, note) in notes.iter().enumerate() {
                    if let Some((hovered_start, hovered_index)) = state.hovered_resize_edge {
                        if *start == hovered_start && note_index == hovered_index {
                            self.draw_resize_indicator(frame, *start, note, bounds, midi_offset);
                        }
                    }
                }
            }
        }
        
        // Draw pending note (being created)
        if let Some(pending) = &state.pending_note {
            self.draw_note(frame, pending.start, &pending.note, bounds, midi_offset);
        }
     }

    /// Draw a resize indicator (vertical line) on the right edge of a note
    fn draw_resize_indicator(&self, frame: &mut Frame, start: Tick, note: &MidiNote, bounds: &Rectangle, midi_offset: u8) {
        let x = self.tick_to_x(start) + self.tick_to_x(note.length);
        let y = pitch_to_y(note.key, &bounds.size(), midi_offset);
        
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

    fn draw_note(&self, frame: &mut Frame, start: Tick, note: &MidiNote, bounds: &Rectangle, midi_offset: u8) {
        self.draw_note_with_color(frame, start, note, bounds, Color::from_rgba(0.0, 0.8, 1.0, 0.5), midi_offset);
    }

    fn draw_note_selected(&self, frame: &mut Frame, start: Tick, note: &MidiNote, bounds: &Rectangle, midi_offset: u8) {
        // Brighter color for selected notes
        self.draw_note_with_color(frame, start, note, bounds, Color::from_rgba(0.2, 1.0, 1.0, 0.8), midi_offset);
    }

    fn draw_note_with_color(&self, frame: &mut Frame, start: Tick, note: &MidiNote, bounds: &Rectangle, color: Color, midi_offset: u8) {
        // Skip if note is out of visible range
        if note.key < midi_offset {
            return;
        }
        let x = self.tick_to_x(start);
        let width = self.tick_to_x(note.length);
        let y = pitch_to_y(note.key, &bounds.size(), midi_offset);
        
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
        
        let pitch = y_to_pitch(relative_y, &bounds.size(), self.midi_offset);

        // Check all notes to find one that contains this position
        for (start_tick, notes) in &self.notes {
            for (note_index, note) in notes.iter().enumerate() {
                if note.key == pitch {
                    let note_start_x = self.tick_to_x(*start_tick);
                    let note_end_x = note_start_x + self.tick_to_x(note.length);
                    let note_y = pitch_to_y(note.key, &bounds.size(), self.midi_offset);
                    
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
        
        let pitch = y_to_pitch(relative_y, &bounds.size(), self.midi_offset);

        // Check all notes to find one whose right edge is being hovered
        for (start_tick, notes) in &self.notes {
            for (note_index, note) in notes.iter().enumerate() {
                if note.key == pitch {
                    let note_start_x = self.tick_to_x(*start_tick);
                    let note_end_x = note_start_x + self.tick_to_x(note.length);
                    let note_y = pitch_to_y(note.key, &bounds.size(), self.midi_offset);
                    
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

fn pitch_to_y(key: u8, bounds: &Size, midi_offset: u8) -> f32 {
    let y = (key - midi_offset) as f32 * NOTE_HEIGHT;
    bounds.height - y
}

/// Convert Y position (relative to grid) to MIDI pitch. Uses midi_offset as the lowest visible note.
fn y_to_pitch(relative_y: f32, bounds: &Size, midi_offset: u8) -> u8 {
    let row = ((bounds.height - relative_y) / NOTE_HEIGHT).ceil() as u8;
    (midi_offset as u16 + row as u16).min(127) as u8
}


// --- CONTAINER WIDGET ---
// This is what you actually put in your Iced layout.
impl MidiEditor {
    pub fn new(notes: BTreeMap<Tick, Vec<MidiNote>>, region_identifier: RegionIdentifier, snap_to_grid: SnapToGrid, midi_offset: u8) -> Self {
        Self {
            cache: canvas::Cache::default(),
            scroll_offset: Vector::default(),
            midi_offset,
            notes,
            region_identifier,
            snap_to_grid,
        }
    }

    pub fn view(notes: BTreeMap<Tick, Vec<MidiNote>>, region_identifier: RegionIdentifier, snap_to_grid: SnapToGrid, midi_offset: u8) -> Canvas<Self, Message> {
        Canvas::new(MidiEditor::new(notes, region_identifier, snap_to_grid, midi_offset))
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

    pub fn view(&self, midi_seq: &MidiSeq, snap_to_grid: SnapToGrid, midi_offset: u8) -> Element<'_, Message> {
        let canvas = MidiEditor::view(midi_seq.notes.clone(), midi_seq.id, snap_to_grid, midi_offset);
        
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
    /// Scroll pitch view: positive = see higher notes, negative = see lower notes (Logic Pro: wheel, Page Up/Down)
    ScrollPitch(i16),
}

