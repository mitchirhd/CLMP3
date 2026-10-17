use anyhow::Ok;
use crossterm::event::{
    KeyCode::{self},
    KeyModifiers,
};
use ratatui::{
    style::Style,
    symbols::border,
    widgets::{Block, StatefulWidget, Widget},
};
use ratatui_textarea::{Input, Key, TextArea};

use crate::{app_state::AppState, online_helper::OnlineHelper, widgets::traits::InputWidget};

pub struct OnlineWidget {
    textarea: TextArea<'static>,
}

impl OnlineWidget {
    pub fn new() -> OnlineWidget {
        OnlineWidget {
            textarea: TextArea::default(),
        }
    }

    fn start_import(&self, state: &AppState) {
        match self.textarea.lines().first() {
            Some(url) => OnlineHelper::fetch_url(
                state.player.queue.clone(),
                state.metadata_loader.clone(),
                url.clone(),
            ),
            None => {}
        }
    }
}

impl StatefulWidget for &OnlineWidget {
    type State = AppState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let style = if state.all_input_to_widget {
            Style::default().light_blue()
        } else {
            Style::default().white()
        };
        let block = Block::bordered().border_set(border::PLAIN).style(style);
        (&self.textarea).render(block.inner(area), buf);
        block.render(area, buf);
    }
}

impl InputWidget for OnlineWidget {
    fn on_input(
        &mut self,
        modifier: crossterm::event::KeyModifiers,
        key: crossterm::event::KeyCode,
        state: &mut AppState,
    ) -> anyhow::Result<()> {
        match (key, state.all_input_to_widget) {
            (KeyCode::Esc, _) => state.all_input_to_widget = false,
            (KeyCode::Right, true) => {
                self.start_import(state);
                state.all_input_to_widget = false;
                self.textarea.clear();
            }
            (KeyCode::Enter, false) => state.all_input_to_widget = true,
            (_, true) => {
                self.textarea.input(Input {
                    key: Key::from(key),
                    ctrl: modifier.contains(KeyModifiers::CONTROL),
                    alt: modifier.contains(KeyModifiers::ALT),
                    shift: modifier.contains(KeyModifiers::SHIFT),
                });
            }
            (_, false) => {}
        }
        Ok(())
    }
}
