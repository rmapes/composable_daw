use iced::widget::{button, column, pick_list, row, text};
use iced::Element;

use crate::models::components::Track;
use crate::models::instuments::{InstrumentActions, SynthMessage};

use crate::threads::engine::actions::Actions;
use crate::threads::ui::actions::Message;

/// Builds the synth instrument editor UI. `to_message` maps synth-specific messages into the app `Message` type.
pub fn synth_editor_ui<F>(track: &Track, synth: &crate::models::instuments::SimpleSynth, to_message: F) -> Element<'static, Message>
where
    F: Fn(SynthMessage) -> Message + 'static,
{
    column![
        text("Instrument Settings"),
        row![
            text("Soundfont:").size(12),
            button(text(synth.soundfont.clone()).size(12)).on_press(
                to_message(SynthMessage::SelectSoundFont(track.id))
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