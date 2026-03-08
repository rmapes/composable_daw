use std::sync::Arc;

use iced::{Element, Length, Task};

use crate::models::components::Track;
use crate::threads::audio::sources::synth::InstrumentRegistry;
use crate::threads::engine::actions::Actions;

use super::actions::Message;
use super::components;
use super::instrument_editor_event::Event;

#[derive(Debug, Clone)]
pub struct Component {
    width: Length,
    height: Length,
    registry: Arc<InstrumentRegistry>,
}

impl Component {
    pub fn new(width: Length, height: Length, registry: Arc<InstrumentRegistry>) -> Self {
        Self {
            width,
            height,
            registry,
        }
    }

    /// Handle an instrument-editor event by dispatching to the active instrument's editor. Returns a task and an optional engine action.
    pub fn update(&self, evt: Event) -> (Task<Message>, Option<Actions>) {
        if let Some(result) = self.registry.handle_editor_event(evt) {
            result
        } else {
            (Task::none(), None)
        }
    }

    pub fn view(&self, track: &Track) -> Element<'_, Message> {
        let default_cfg = track.instrument.config.is_none().then(|| {
            self.registry.default_config(&track.instrument.kind)
        });
        let config = track
            .instrument
            .config
            .as_deref()
            .or_else(|| default_cfg.as_ref().and_then(|b| b.as_deref()));
        let content = config
            .and_then(|c| {
                self.registry
                    .view_editor(&track.instrument.kind, track, c)
            })
            .unwrap_or_else(|| iced::widget::column![].into());

        components::module(content)
            .width(self.width)
            .height(self.height)
            .into()
    }
}
