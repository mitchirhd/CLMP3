use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    style::Style,
    widgets::{List, StatefulWidget},
};

use crate::{app_state::AppState, widgets::traits::InputWidget};

pub struct QueueWidget {}

impl QueueWidget {
    pub fn new() -> QueueWidget {
        QueueWidget {}
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
        let items = state
            .queue
            .prev_as_vec()
            .into_iter()
            .chain(state.queue.as_vec().into_iter())
            .map(|s| match s {
                crate::source_builder::SourceType::SineWave(freq, _) => format!("Sine {freq}"),
                crate::source_builder::SourceType::File(file) => format!("{}", file.name),
            });

        let list = List::new(items)
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true);

        StatefulWidget::render(list, area, buf, &mut state.queue_state);
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
            KeyCode::Down => state.queue_state.select_next(),
            KeyCode::Up => state.queue_state.select_previous(),
            KeyCode::Left => state.queue_state.select(None),
            _ => {}
        }
    }
}
