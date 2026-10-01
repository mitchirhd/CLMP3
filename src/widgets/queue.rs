use std::{cmp::Ordering, rc::Rc};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    style::Style,
    widgets::{List, ListItem, StatefulWidget},
};

use crate::{app_state::AppState, widgets::traits::InputWidget};

pub struct QueueWidget {}

impl QueueWidget {
    pub fn new() -> QueueWidget {
        QueueWidget {}
    }

    fn select(&self, state: &mut AppState, f: Rc<dyn Fn(&mut AppState)>) {
        if state.queue_state.selected().is_none() {
            state.queue_state.select(Some(state.queue.prev_len()));
            return;
        }
        f(state);
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
        let prev_len = state.queue.prev_len();

        let items: Vec<ListItem> = {
            // Lock nur einmal für die ganze Liste
            let map = state.metadata_map.read().unwrap();

            state
                .queue
                .prev_as_vec()
                .into_iter()
                .chain(state.queue.as_vec())
                .enumerate()
                .map(|(i, source)| {
                    let metadata = source.id().and_then(|id| map.get(&id));
                    let label = source.label(metadata.map(|m| m.as_ref()));

                    let style = match i.cmp(&prev_len) {
                        Ordering::Less => Style::default().dark_gray(),
                        Ordering::Equal => Style::default().blue(),
                        Ordering::Greater => Style::default().white(),
                    };

                    ListItem::new(label).style(style)
                })
                .collect()
        }; // Guard und Borrow von state enden hier

        let list = List::new(items)
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true);

        list.render(area, buf, &mut state.queue_state);
    }
}

impl InputWidget for QueueWidget {
    fn on_input(
        &mut self,
        _modifier: KeyModifiers,
        key: crossterm::event::KeyCode,
        state: &mut AppState,
    ) {
        match key {
            KeyCode::Down => self.select(state, Rc::new(|s| s.queue_state.select_next())),
            KeyCode::Up => self.select(state, Rc::new(|s| s.queue_state.select_previous())),
            KeyCode::Left => state.queue_state.select(None),
            _ => {}
        }
    }
}
