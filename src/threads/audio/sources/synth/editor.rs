use iced::widget::{button, column, pick_list, row, text};
use iced::{Element, Task};

use crate::models::components::Track;
use crate::models::instrument::InstrumentActions;

use super::config::{SimpleSynth, SynthMessage};

use crate::threads::engine::actions::Actions;
use crate::threads::ui::actions::Message;
use crate::threads::ui::file_picker;
use crate::threads::ui::instrument_editor_event::Event;

const SOUNDFONTS_DIR: &str = "./soundfonts/";

/// Handles instrument editor events for the synth. Returns `Some(task, action)` when the event
/// is a synth message, `None` otherwise.
pub fn handle_event(evt: Event) -> Option<(Task<Message>, Option<Actions>)> {
    let sm = match evt {
        Event::Synth(sm) => sm,
    };
    let out = match sm {
        SynthMessage::SelectSoundFont(track_id) => (
            Task::perform(
                file_picker::pick_file(track_id, SOUNDFONTS_DIR),
                |(track_id, path)| Message::InstrumentEditor(Event::Synth(SynthMessage::SetSoundFont(track_id, path))),
            ),
            None,
        ),
        SynthMessage::SetSoundFont(track_id, path) => (
            Task::none(),
            Some(Actions::Instrument(track_id, InstrumentActions::SetSoundFont(path))),
        ),
    };
    Some(out)
}

/// Builds the synth instrument editor UI. Produces `Message::InstrumentEditor(Event::Synth(...))` for synth actions.
pub fn synth_editor_ui(track: &Track, synth: &SimpleSynth) -> Element<'static, Message> {
    column![
        text("Instrument Settings"),
        row![
            text("Soundfont:").size(12),
            button(text(synth.soundfont.clone()).size(12)).on_press(
                Message::InstrumentEditor(Event::Synth(SynthMessage::SelectSoundFont(track.id)))
            )
        ]
        .spacing(8),
        text("Bank"),
        number_selector(0, 127, synth.bank as u8, {
            let track_id = track.id;
            move |val: u8| {
                Message::Engine(Actions::Instrument(
                    track_id,
                    InstrumentActions::SetBank(val as u32),
                ))
            }
        }),
        text("Program"),
        number_selector(0, 127, synth.program, {
            let track_id = track.id;
            move |val: u8| {
                Message::Engine(Actions::Instrument(
                    track_id,
                    InstrumentActions::SetProgram(val),
                ))
            }
        }),
        row![button(text("Done")).on_press(Message::CloseInstrumentEditor)]
            .spacing(8),
    ]
    .spacing(4)
    .into()
}

fn number_selector<F>(min: u8, max: u8, current: u8, on_set: F) -> Element<'static, Message>
where
    F: Fn(u8) -> Message + 'static,
{
    let options: Vec<u8> = (min..=max).collect();
    pick_list(options, Some(current), on_set).into()
}