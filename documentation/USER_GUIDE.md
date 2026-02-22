# Composable DAW — User Guide

This document describes each major piece of functionality and **how to access it in the user interface**.

---

## Application layout

- **Top**: Menu bar (File, Edit).
- **Below menu**: **Control bar** with transport buttons (rewind, stop, play).
- **Main area** (left to right):
  - **Left panel**: Track settings (channel strip) for the **selected track**.
  - **Right**: **Composer**
    - **Top**: **Track View** — timeline ruler and track lanes with regions.
    - **Bottom**: **Editor** — either the Pattern editor (step grid) or the MIDI editor (piano roll), depending on which region is selected.

---

## Transport (playback)

| Functionality         | How to access                                                                                                                           |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| **Play**              | Click the **Play** button in the control bar (below the menu).                                                                          |
| **Pause / Stop**      | Click the **Stop** button in the control bar.                                                                                           |
| **Rewind to start**   | Click the **Rewind to start** (rewind) icon in the control bar. The playhead moves to the beginning of the timeline.                    |
| **Playhead position** | The playhead moves automatically during playback. You can also set it by clicking on the timeline ruler in the composer (if supported). |

---

## Project

| Functionality   | How to access                                          |
| --------------- | ------------------------------------------------------ |
| **New project** | **File → New**. Clears the current project (new file). |
| **Open file**   | **File → Open**. Not yet implemented.                  |

---

## Tracks

| Functionality      | How to access                                                                                                                                                     |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Add track**      | In the **composer** area, click the **"+"** button above the track list. A new track is added.                                                                    |
| **Select track**   | Click on a track’s row or label in the composer so that it becomes the selected track. The **track settings** panel on the left shows that track’s channel strip. |
| **Track settings** | After selecting a track, the **left panel** shows that track’s name and instrument settings (e.g. SoundFont, Bank, Program for a synth track).                    |

---

## Regions

| Functionality                      | How to access                                                                                                                                                       |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Add Pattern region at playhead** | Set the playhead where you want the region, then choose **Edit → Add Pattern**. A new Pattern region is added on the **selected track** at the playhead.            |
| **Add Midi region at playhead**    | Set the playhead where you want the region, then choose **Edit → Add Midi**. A new Midi region is added on the **selected track** at the playhead.                  |
| **Select a region**                | **Click** on a region block in the composer. The editor at the bottom switches to the Pattern editor (for a Pattern region) or the MIDI editor (for a Midi region). |
| **Move a region**                  | **Drag** a region block in the composer: press on the region, move to another track or time, then release. The region moves to the new track and/or position.       |
| **Delete a region**                | Select the region by clicking it, then choose **Edit → Delete Region**. The selected region is removed.                                                             |
| **Set playhead**                   | Click on the **timeline ruler** in the composer to set the playhead position (when implemented). Playback and “add region at playhead” use this position.           |
| **Deselect regions**               | Use the deselect-all behaviour if exposed in the UI (e.g. click on empty area); otherwise selection remains until you select another region or use a menu action.   |

---

## Composer (timeline)

| Functionality              | How to access                                                                                                 |
| -------------------------- | ------------------------------------------------------------------------------------------------------------- |
| **Timeline view**          | The center area shows a **ruler** (time in bars/beats) and **track lanes** with region blocks.                |
| **Track list and regions** | Each track is a horizontal lane. Regions appear as blocks; you click to select and drag to move.              |
| **Region drag**            | Click and hold on a region, drag horizontally (time) or vertically (track), then release to confirm the move. |

---

## Editor area (Pattern vs MIDI)

| Functionality          | How to access                                                                                                                                                                                                                   |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Switch editor type** | Select a **Pattern** region → the **Pattern editor** (step grid) is shown. Select a **Midi** region → the **MIDI editor** (piano roll) is shown. If no region or an unsupported type is selected, the editor area may be empty. |

---

## Pattern editor

| Functionality          | How to access                                                                                                                           |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| **Toggle step on/off** | With a **Pattern** region selected, the bottom area shows a **grid of steps** (beats × notes). **Click** a step to toggle it on or off. |

---

## MIDI editor

| Functionality                  | How to access                                                                                                                                                                                  |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Create a MIDI note**         | With a **Midi** region selected, the bottom area shows the **piano roll**. **Click and drag** (or equivalent draw gesture) in the note grid to create a new note at the chosen time and pitch. |
| **Move a MIDI note**           | **Drag** an existing note horizontally (time) or vertically (pitch). The note updates when you release.                                                                                        |
| **Resize / change length**     | If supported, drag the **right edge** of a note to change its length.                                                                                                                          |
| **Select MIDI notes**          | **Click** a note to select it. **Shift+click** to add/remove notes from the selection.                                                                                                         |
| **Delete selected MIDI notes** | Select one or more notes, then press **Backspace**. All selected notes are removed.                                                                                                            |
| **Snap to grid**               | At the **top right** of the MIDI editor there is a **“Snap:”** dropdown. Choose **None**, **Division**, **Beat**, or **Bar** to control how note start/end and movement snap to the grid.      |
| **Scroll pitch (view)**        | Use **Page Up** to shift the view to higher pitches, **Page Down** to lower pitches. **Mouse wheel** over the editor can also scroll the pitch axis (if supported).                            |

---

## Synth / track settings (left panel)

| Functionality        | How to access                                                                                                                                                                           |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Select SoundFont** | With a **track** selected, the left panel shows a button labelled with the current SoundFont (or placeholder). **Click** that button to open a file picker and choose a SoundFont file. |
| **Set Bank**         | In the same track settings panel, use the **Bank** dropdown/picker (0–127) to set the synth bank.                                                                                       |
| **Set Program**      | In the same panel, use the **Program** dropdown/picker (0–127) to set the synth program.                                                                                                |

---

## Menu reference

- **File**
  - **New** — New project (clear current).
  - **Open** — Open file (not yet implemented).
- **Edit**
  - **Add Pattern** — Add a Pattern region at the playhead on the selected track.
  - **Add Midi** — Add a Midi region at the playhead on the selected track.
  - **Delete Region** — Delete the currently selected region.

---

## Summary

- **Transport**: Control bar (Play, Stop, Rewind).
- **Project**: File menu (New, Open).
- **Tracks**: “+” in composer to add; click track to select; left panel for settings.
- **Regions**: Edit menu to add/delete; click to select; drag to move.
- **Pattern editor**: Select a Pattern region, then use the step grid to toggle steps.
- **MIDI editor**: Select a Midi region, then use the piano roll to add, move, resize, select, and delete notes; use Snap dropdown and Page Up/Down (and wheel) for view/snap.
