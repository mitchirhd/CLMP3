use std::cmp::Ordering;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    style::Style,
    widgets::{List, ListItem, ListState, StatefulWidget},
};

use crate::{
    app_state::AppState,
    source_builder::{Metadata, SourceType},
    widgets::traits::InputWidget,
};

pub struct QueueWidget {}

impl QueueWidget {
    pub fn new() -> QueueWidget {
        QueueWidget {}
    }

    fn select(state: &mut AppState, f: impl FnOnce(&mut ListState)) {
        if state.queue_state.selected().is_none() {
            state
                .queue_state
                .select(Some(state.player.queue.prev_len()));
            return;
        }
        f(&mut state.queue_state);
    }

    fn move_by(state: &mut ListState, f: impl FnOnce(usize) -> usize) {
        if let Some(current) = state.selected() {
            *state = state.with_selected(Some(f(current)));
        };
    }
}

impl StatefulWidget for &QueueWidget {
    type State = AppState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut AppState,
    ) {
        let map = state.metadata_map.read().unwrap();
        let (prev_len, sources) = state.player.queue.snapshot();

        let items: Vec<ListItem> = sources
            .into_iter()
            .enumerate()
            .map(|(i, source)| {
                let metadata = source.id().and_then(|id| map.get(&id));
                let label = label(&source, metadata.map(|m| m.as_ref()));

                let style = match i.cmp(&prev_len) {
                    Ordering::Less => Style::default().dark_gray(),
                    Ordering::Equal => Style::default().blue(),
                    Ordering::Greater => Style::default().white(),
                };

                ListItem::new(label).style(style)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true);

        list.render(area, buf, &mut state.queue_state);
    }
}

fn label(source: &SourceType, metadata: Option<&Metadata>) -> String {
    match source {
        SourceType::SineWave(freq, _) => format!("Sine {freq}"),
        SourceType::File(path) => match metadata {
            Some(m) => format!("{} - {}", m.title, m.artist.as_deref().unwrap_or("unknown")),
            None => crate::source_builder::file_name(path)
        },
    }
}

impl InputWidget for QueueWidget {
    fn on_input(&mut self, _modifier: KeyModifiers, key: KeyCode, state: &mut AppState) {
        let page_step: usize = 5;

        match key {
            KeyCode::Down => Self::select(state, ListState::select_next),
            KeyCode::Up => Self::select(state, ListState::select_previous),
            KeyCode::Left => state.queue_state.select(None),
            KeyCode::Home => Self::select(state, ListState::select_first),
            KeyCode::End => Self::select(state, ListState::select_last),
            KeyCode::PageUp => {
                Self::select(state, |s| Self::move_by(s, |i| i.saturating_sub(page_step)))
            }
            KeyCode::PageDown => {
                Self::select(state, |s| Self::move_by(s, |i| i.saturating_add(page_step)))
            }
            _ => {}
        }
    }
}
