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
use ratatui_textarea::{CursorMove, Input, Key, TextArea};

use crate::{app_state::AppState, online_helper::OnlineHelper, widgets::traits::InputWidget};

const TEXTAREA_PLACEHOLDER: [&str; 2] = ["links to content supported by yt-dlp", "one per line"];

pub struct OnlineWidget {
    textarea: TextArea<'static>,
}

impl OnlineWidget {
    pub fn new() -> OnlineWidget {
        let mut val = OnlineWidget {
            textarea: TextArea::default(),
        };
        val.reset_textarea();
        val
    }

    fn start_import(&self, state: &AppState) {
        OnlineHelper::fetch_urls(
            state.player.queue.clone(),
            state.metadata_loader.clone(),
            self.textarea.lines().to_vec(),
        )
    }

    fn reset_textarea(&mut self) {
        self.textarea.clear();
        self.textarea.insert_str(TEXTAREA_PLACEHOLDER[0]);
        self.textarea.insert_newline();
        self.textarea.insert_str(TEXTAREA_PLACEHOLDER[1]);
        self.textarea.insert_newline();
        self.textarea.move_cursor(CursorMove::Bottom);
        self.textarea.move_cursor(CursorMove::End);
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
            (KeyCode::Esc, _) => {
                state.all_input_to_widget = false;
                self.reset_textarea();
            }
            (KeyCode::Right, true) => {
                self.start_import(state);
                state.all_input_to_widget = false;
                self.reset_textarea();
            }
            (KeyCode::Enter, false) => {
                state.all_input_to_widget = true;
                self.textarea.clear();
            }
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
