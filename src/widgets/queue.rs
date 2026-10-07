use std::cmp::Ordering;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

use crate::{
    app_state::AppState,
    enums::{Clmp3Widget, WidgetArea},
    metadata::Metadata,
    source_builder::SourceType,
    widgets::{
        render_helper::{border_title_first_style, border_title_style, thickness},
        traits::InputWidget,
    },
};

const PAGE_STEP: usize = 5;

pub struct QueueWidget {
    queue_state: ListState,
}

impl QueueWidget {
    pub fn new() -> QueueWidget {
        QueueWidget {
            queue_state: ListState::default(),
        }
    }

    fn update_queue_state(&mut self, state: &AppState) {
        if state.selected_widget() != Clmp3Widget::Queue {
            self.queue_state.select(None);
            *self.queue_state.offset_mut() = state.player.queue.prev_len().saturating_sub(5);
        }
    }

    fn select(&mut self, state: &mut AppState, f: impl FnOnce(&mut ListState)) {
        if self.queue_state.selected().is_none() {
            self.queue_state.select(Some(state.player.queue.prev_len()));
            return;
        }
        f(&mut self.queue_state);
    }

    fn move_by(state: &mut ListState, f: impl FnOnce(usize) -> usize) {
        if let Some(current) = state.selected() {
            *state = state.with_selected(Some(f(current)));
        };
    }
}

impl StatefulWidget for &mut QueueWidget {
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
                    Ordering::Equal => Style::default().light_blue(),
                    Ordering::Greater => Style::default().white(),
                };

                ListItem::new(label).style(style)
            })
            .collect();

        let (thickness, style) = thickness(WidgetArea::Queue, state);
        let queue_block = Block::bordered()
            .title(border_title(state, items.len(), prev_len))
            .border_set(thickness)
            .border_style(style);

        let list = List::new(items)
            .highlight_style(Style::default().fg(Color::White).bg(Color::DarkGray))
            .block(queue_block);

        self.update_queue_state(state);
        list.render(area, buf, &mut self.queue_state);
    }
}

fn label(source: &SourceType, metadata: Option<&Metadata>) -> String {
    match source {
        SourceType::SineWave(freq, _) => format!("Sine {freq}"),
        SourceType::File(path) => match metadata {
            Some(m) => format!("{} - {}", m.title, m.artist.as_deref().unwrap_or("unknown")),
            None => crate::source_builder::file_name(path),
        },
    }
}

fn border_title(state: &AppState, item_count: usize, prev_len: usize) -> Line<'static> {
    let mut spans = vec![
        Span::styled(" q", border_title_first_style(Clmp3Widget::Queue, state)),
        Span::styled("ueue ", border_title_style(Clmp3Widget::Queue, state)),
    ];

    if item_count > 0 {
        spans.push(Span::raw(format!(
            "{}/{}",
            (prev_len + 1).clamp(1, item_count),
            item_count
        )));
    }

    Line::from(spans)
}

impl InputWidget for QueueWidget {
    fn on_input(&mut self, _modifier: KeyModifiers, key: KeyCode, state: &mut AppState) {
        match key {
            KeyCode::Down => self.select(state, ListState::select_next),
            KeyCode::Up => self.select(state, ListState::select_previous),
            KeyCode::Left => self.queue_state.select(None),
            KeyCode::Home => self.select(state, ListState::select_first),
            KeyCode::End => self.select(state, ListState::select_last),
            KeyCode::PageUp => {
                self.select(state, |s| Self::move_by(s, |i| i.saturating_sub(PAGE_STEP)))
            }
            KeyCode::PageDown => {
                self.select(state, |s| Self::move_by(s, |i| i.saturating_add(PAGE_STEP)))
            }
            _ => {}
        }
    }
}
