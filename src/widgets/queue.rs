use std::cmp::Ordering;

use anyhow::Ok;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    style::Style,
    symbols::border,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

use crate::{
    app_state::AppState,
    enums::{Clmp3Widget, WidgetArea},
    metadata::Metadata,
    source_builder::SourceType,
    widgets::{
        render_helper::{
            block_thickness, border_button_style, border_text_style, border_title_first_style,
            border_title_style,
        },
        traits::InputWidget,
    },
};

const PAGE_STEP: usize = 5;
const PREV_ENTRIES_SHOWN: usize = 5;

pub struct QueueWidget {
    queue_state: ListState,
    selected_indexes: Option<Vec<usize>>,
}

impl QueueWidget {
    pub fn new() -> QueueWidget {
        QueueWidget {
            queue_state: ListState::default(),
            selected_indexes: None,
        }
    }

    fn update_queue_state(&mut self, state: &AppState) {
        if state.selected_widget() != Clmp3Widget::Queue {
            self.queue_state.select(None);
            *self.queue_state.offset_mut() = state
                .player
                .queue
                .prev_len()
                .saturating_sub(PREV_ENTRIES_SHOWN);
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

    fn on_left(&mut self) {
        self.queue_state.select(None);
        self.selected_indexes = None
    }

    fn on_select(&mut self, state: &AppState) {
        if let Some(index) = self.queue_state.selected() {
            if index == state.player.queue.prev_len() {
                return;
            }

            let selected_vec = self.selected_indexes.get_or_insert_with(Vec::new);
            if let Some(pos) = selected_vec.iter().position(|&i| i == index) {
                selected_vec.remove(pos);
            } else {
                selected_vec.push(index);
            }
        }
    }

    fn on_move(&mut self, state: &AppState) {
        if let Some(selected_indexes) = &self.selected_indexes
            && let Some(current_index) = self.queue_state.selected()
            && current_index != state.player.queue.prev_len()
        {
            state.player.queue.swap(current_index, selected_indexes);
            self.selected_indexes = None;
        }
    }

    fn delete_selected(&self, state: &AppState) {
        if let Some(selected) = self.queue_state.selected()
            && (selected != state.player.queue.prev_len() || state.player.stopped())
        {
            state.player.queue.delete_at(selected);
        }
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
        let sources = state.player.queue.snapshot();
        let prev_len = state.player.queue.prev_len();

        let list_height = area.height as usize;
        let min_index = self
            .queue_state
            .selected()
            .map_or(prev_len.saturating_sub(PREV_ENTRIES_SHOWN), |s| {
                s.saturating_sub(list_height)
            });
        let max_index = self
            .queue_state
            .selected()
            .map_or(list_height, |s| s.saturating_add(list_height));

        let items: Vec<ListItem> = sources
            .into_iter()
            .enumerate()
            .map(|(i, source)| {
                if i < min_index || max_index < i {
                    return ListItem::new("");
                }

                let metadata = map.get(&source.id());
                let label = label(&source, metadata.map(|m| m.as_ref()));

                let style = if let Some(selected) = &self.selected_indexes
                    && selected.contains(&i)
                {
                    Style::default().white().on_dark_gray()
                } else {
                    match i.cmp(&prev_len) {
                        Ordering::Less => Style::default().dark_gray(),
                        Ordering::Equal => Style::default().light_blue(),
                        Ordering::Greater => Style::default().white(),
                    }
                };

                ListItem::new(label).style(style)
            })
            .collect();

        let (thickness, style) = block_thickness(WidgetArea::Queue, state);
        let (top_title_left, top_title_right) = border_title(state, items.len(), prev_len);
        let queue_block = Block::bordered()
            .title(top_title_left)
            .title(top_title_right)
            .title_bottom(border_title_bottom(state, thickness))
            .border_set(thickness)
            .border_style(style);

        let list = List::new(items)
            .highlight_style(Style::default().white().on_blue())
            .block(queue_block);

        self.update_queue_state(state);
        list.render(area, buf, &mut self.queue_state);
    }
}

fn label(source: &SourceType, metadata: Option<&Metadata>) -> String {
    match metadata {
        Some(m) => {
            let mut label_vec = vec![m.title.to_string()];
            if let Some(artist) = m.artist.as_deref() {
                label_vec.push(artist.to_string());
            }
            label_vec.join(" - ")
        }
        None => source.fallback_label(),
    }
}

fn border_title(
    state: &AppState,
    item_count: usize,
    prev_len: usize,
) -> (Line<'static>, Line<'static>) {
    let title_style = border_title_style(Clmp3Widget::Queue, state);
    let title_first_style = border_title_first_style(Clmp3Widget::Queue, state);
    let mut left_spans = vec![
        Span::styled(" q", title_style),
        Span::styled("u", title_first_style),
        Span::styled("eue ", title_style),
    ];

    if item_count > 0 {
        left_spans.push(Span::raw(format!(
            "{}/{}",
            (prev_len + 1).clamp(1, item_count),
            item_count
        )));
    }

    let button_style = border_button_style(Clmp3Widget::Queue, state);
    let text_style = border_text_style(Clmp3Widget::Queue, state);
    let right_spans = vec![
        Span::styled(" 🢁 ", button_style),
        Span::styled("select ", text_style),
        Span::styled("⮠ 🢃 ", button_style),
    ];

    (
        Line::from(left_spans).left_aligned(),
        Line::from(right_spans).right_aligned(),
    )
}

fn border_title_bottom(state: &AppState, border: border::Set<'static>) -> Line<'static> {
    let button_style = border_button_style(Clmp3Widget::Queue, state);
    let text_style = border_text_style(Clmp3Widget::Queue, state);

    let spans = vec![
        Span::styled(" shuffle ", text_style),
        Span::styled("s ", button_style),
        Span::raw(border.horizontal_top),
        Span::raw(border.horizontal_top),
        Span::styled(" clear ", text_style),
        Span::styled("c ", button_style),
        Span::raw(border.horizontal_top),
        Span::raw(border.horizontal_top),
        Span::styled(" swap ", text_style),
        Span::styled("🢂 ", button_style),
        Span::raw(border.horizontal_top),
        Span::raw(border.horizontal_top),
        Span::styled(" remove ", text_style),
        Span::styled("del ", button_style),
    ];

    Line::from(spans)
}

impl InputWidget for QueueWidget {
    fn on_input(
        &mut self,
        _modifier: KeyModifiers,
        key: KeyCode,
        state: &mut AppState,
    ) -> anyhow::Result<()> {
        match key {
            KeyCode::Down => self.select(state, ListState::select_next),
            KeyCode::Up => self.select(state, ListState::select_previous),
            KeyCode::Left => self.on_left(),
            KeyCode::Right => self.on_move(state),
            KeyCode::Enter => self.on_select(state),
            KeyCode::Home => self.select(state, ListState::select_first),
            KeyCode::End => self.select(state, ListState::select_last),
            KeyCode::PageUp => {
                self.select(state, |s| Self::move_by(s, |i| i.saturating_sub(PAGE_STEP)))
            }
            KeyCode::PageDown => {
                self.select(state, |s| Self::move_by(s, |i| i.saturating_add(PAGE_STEP)))
            }
            KeyCode::Delete => self.delete_selected(state),
            KeyCode::Char('s') => state.player.shuffle(),
            KeyCode::Char('c') => {
                state.clear_all_metadata();
                state.player.clear();
            }
            _ => {}
        };
        Ok(())
    }
}
