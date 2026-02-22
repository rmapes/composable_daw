# Composable DAW — Test Specification

This document describes, in human language, how to test each major piece of functionality. It can be used for manual testing or as a basis for automated integration tests.

---

## 1. Transport (playback)

### 1.1 Play

- **Steps**: Start the application, ensure at least one track exists. Click the Play button in the control bar.
- **Expected**: Playback starts; the playhead (if visible) moves along the timeline. Audio is heard if there are playable regions and a working audio output. Play stops when the end of the last playable region is reached.
- **Variants**: With no regions; with Pattern region; with Midi region and SoundFont.

### 1.2 Pause / Stop

- **Steps**: Start playback, then click the Stop button.
- **Expected**: Playback stops; the playhead stops moving. Clicking Play again resumes from the current playhead position.

### 1.3 Rewind to start

- **Steps**: Move playhead (e.g. by playback) away from the start, then click the Rewind-to-start button.
- **Expected**: Playhead returns to the beginning of the timeline (tick 0). Playback, if running, continues from the start.

### 1.4 Playhead position during playback

- **Steps**: Start playback and observe the playhead (ruler or indicator).
- **Expected**: Playhead position updates over time and reflects current playback position.

---

## 2. Project

### 2.1 New project

- **Steps**: Create a project with at least one track and one region. Use **File → New**.
- **Expected**: Project is cleared (e.g. no tracks or default empty state). No crash; UI is responsive.

### 2.2 Open file

- **Steps**: Use **File → Open**.
- **Expected**: Currently not implemented; expected behaviour (e.g. file dialog or “not implemented” message) should be documented and consistent (no silent failure or crash).

---

## 3. Tracks

### 3.1 Add track

- **Steps**: Click the “+” button in the composer area (above the track list).
- **Expected**: A new track appears in the track list. Track count increases by one.

### 3.2 Select track

- **Steps**: Click on different tracks in the composer (track row or label).
- **Expected**: The clicked track becomes selected (e.g. visual highlight). The left panel (track settings) shows the selected track’s name and controls.

### 3.3 Track settings visibility

- **Steps**: Select a track. Observe the left panel.
- **Expected**: Panel shows that track’s name and instrument settings (e.g. SoundFont button, Bank, Program for a synth track).

---

## 4. Regions

### 4.1 Add Pattern region at playhead

- **Steps**: Ensure a track is selected and playhead is at a known position (e.g. start). Use **Edit → Add Pattern**.
- **Expected**: A new Pattern region appears on the selected track at the playhead. Selecting it shows the Pattern editor below.

### 4.2 Add Midi region at playhead

- **Steps**: Same as 4.1 but use **Edit → Add Midi**.
- **Expected**: A new Midi region appears on the selected track at the playhead. Selecting it shows the MIDI editor below.

### 4.3 Select region

- **Steps**: Click on a region block in the composer.
- **Expected**: Region becomes selected (e.g. visual feedback). Editor area updates: Pattern region → Pattern editor; Midi region → MIDI editor.

### 4.4 Move region (drag)

- **Steps**: Click and hold on a region, drag to another track and/or to a different time, then release.
- **Expected**: Region moves to the new track and/or time. No overlap/collision if the application disallows it; otherwise behaviour is as designed. Selection updates to the new position.

### 4.5 Delete region

- **Steps**: Select a region. Use **Edit → Delete Region**.
- **Expected**: The selected region is removed from the project. Editor area may show empty or another region if selected.

### 4.6 Set playhead (if supported)

- **Steps**: Click on the timeline ruler at a given time.
- **Expected**: Playhead moves to that position. “Add region at playhead” and playback use this position.

---

## 5. Composer (timeline)

### 5.1 Timeline and ruler

- **Steps**: Open the application and look at the composer.
- **Expected**: A time ruler is visible; track lanes are visible; existing regions are shown as blocks.

### 5.2 Region drag feedback

- **Steps**: Start dragging a region; move mouse; release or cancel (e.g. Escape if supported).
- **Expected**: During drag, feedback shows proposed position (and validity if applicable). On valid drop, region moves; on cancel, region returns to original position.

---

## 6. Editor switching (Pattern vs MIDI)

### 6.1 Pattern editor when Pattern region selected

- **Steps**: Select a Pattern region.
- **Expected**: The editor area shows the Pattern editor (step grid). No MIDI piano roll.

### 6.2 MIDI editor when Midi region selected

- **Steps**: Select a Midi region.
- **Expected**: The editor area shows the MIDI editor (piano roll, keyboard, ruler). No Pattern step grid.

### 6.3 Editor when no/invalid region selected

- **Steps**: Deselect all regions or select a type that has no editor.
- **Expected**: Editor area is empty or shows a neutral state; no crash.

---

## 7. Pattern editor

### 7.1 Toggle step on/off

- **Steps**: Select a Pattern region. Click a step in the grid.
- **Expected**: Step toggles between on and off; state is persisted (e.g. after switching region and back). Playback reflects the pattern.

### 7.2 Multiple toggles

- **Steps**: Toggle several steps on and off in different positions.
- **Expected**: Each step maintains its state; pattern content matches user actions.

---

## 8. MIDI editor

### 8.1 Create MIDI note

- **Steps**: Select a Midi region. In the piano roll, use the create-note gesture (e.g. click-drag or click).
- **Expected**: A new note appears at the chosen time and pitch. It can be played back if the track has a SoundFont and the region is in range.

### 8.2 Move MIDI note

- **Steps**: Drag an existing note to a new time and/or pitch.
- **Expected**: Note moves to the new position; playback reflects the change.

### 8.3 Resize MIDI note (if supported)

- **Steps**: Drag the end edge of a note to change length.
- **Expected**: Note length updates; playback reflects the new duration.

### 8.4 Select MIDI notes

- **Steps**: Click a note; then Shift+click another note.
- **Expected**: First note is selected; second click adds/removes from selection. Multiple notes can be selected.

### 8.5 Delete selected MIDI notes

- **Steps**: Select one or more notes. Press Backspace.
- **Expected**: Selected notes are removed. Other notes unchanged.

### 8.6 Snap to grid

- **Steps**: Set Snap to “Division”, “Beat”, or “Bar”. Create or move a note.
- **Expected**: Note start (and end if applicable) snap to the chosen grid. With “None”, no snapping.

### 8.7 Scroll pitch (Page Up / Page Down)

- **Steps**: In the MIDI editor, press Page Up repeatedly, then Page Down.
- **Expected**: View scrolls to show higher pitches (Page Up) and lower pitches (Page Down). Notes remain correct; no data loss.

### 8.8 Scroll pitch (mouse wheel)

- **Steps**: If supported, scroll the mouse wheel over the MIDI editor.
- **Expected**: Pitch view scrolls similarly to Page Up/Down; behaviour is consistent.

---

## 9. Synth / track settings

### 9.1 Select SoundFont

- **Steps**: Select a track. In the left panel, click the SoundFont button (or label). Choose a valid SoundFont file in the file picker.
- **Expected**: Track’s SoundFont updates; label/button reflects the chosen file. MIDI playback on that track uses the new SoundFont (if playback is tested).

### 9.2 Set Bank

- **Steps**: In the track settings, change the Bank control (0–127).
- **Expected**: Bank value updates; playback uses the selected bank when applicable.

### 9.3 Set Program

- **Steps**: In the track settings, change the Program control (0–127).
- **Expected**: Program value updates; playback uses the selected program when applicable.

---

## 10. Menu and layout

### 10.1 Menu items present

- **Steps**: Open File and Edit menus.
- **Expected**: File: New, Open. Edit: Add Pattern, Add Midi, Delete Region. Items are enabled/disabled as designed (e.g. Delete Region only when a region is selected).

### 10.2 Control bar and layout

- **Steps**: Check that control bar, track list, composer, and editor are visible and correctly arranged.
- **Expected**: No overlapping or missing panels; layout matches the user guide.

---

## 11. Error and edge cases

### 11.1 Delete region with no selection

- **Steps**: Deselect all regions (if possible). Use **Edit → Delete Region**.
- **Expected**: No crash; either no change or a clear feedback (e.g. disabled menu or message).

### 11.2 Add region with no track

- **Steps**: If the project can be in a state with no tracks, try **Edit → Add Pattern** or **Add Midi**.
- **Expected**: No crash; either operation is disabled or a new track is created, as designed.

### 11.3 Audio failure at startup

- **Steps**: Run in an environment where audio init fails (e.g. no device or driver issue).
- **Expected**: Application starts with fallback (e.g. dummy audio); user is informed; no silent failure or panic.

### 11.4 Window close

- **Steps**: Close the main window (e.g. window close button).
- **Expected**: Application shuts down cleanly; engine stops; no crash or hang.

---

## Summary

- **Transport**: Verify Play, Stop, Rewind, and playhead behaviour.
- **Project**: New file; Open (current behaviour).
- **Tracks**: Add track, select track, track settings visibility.
- **Regions**: Add (Pattern/Midi at playhead), select, move (drag), delete.
- **Composer**: Timeline and ruler; drag feedback.
- **Editors**: Correct editor for region type; Pattern toggle; MIDI create/move/delete/select, snap, pitch scroll.
- **Synth**: SoundFont, Bank, Program.
- **Menus and layout**: Items present and enabled correctly; layout as described.
- **Edge cases**: No selection, no track, audio failure, window close.

These tests can be automated (e.g. with iced_test and the existing emulator pattern) where the UI is scriptable; others remain manual.
