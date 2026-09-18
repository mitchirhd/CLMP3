use std::sync::{Arc, atomic::Ordering};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Gauge, Paragraph, Widget},
};

use crate::audio::AudioControls;

pub struct PlayerWidget {
    audio_controls: Arc<AudioControls>,
}

impl PlayerWidget {
    pub fn new(controls: Arc<AudioControls>) -> PlayerWidget {
        PlayerWidget {
            audio_controls: controls,
        }
    }
}

impl Widget for &PlayerWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let [progress_area, button_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(area);

        let progress_bar = Gauge::default().percent(50);

        let button_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ])
            .split(button_area.centered_horizontally(Constraint::Ratio(1, 2)));

        let stopped = self.audio_controls.is_stopped.load(Ordering::Relaxed);
        let paused = self.audio_controls.is_paused.load(Ordering::Relaxed);
        let play_pause_symbol = if stopped || paused { "▶" } else { "⏸" };

        render_button("⏮", button_area[0], buf);
        render_button(play_pause_symbol, button_area[1], buf);
        render_button("⏭", button_area[2], buf);
    }
}

fn render_button(label: &str, area: Rect, buf: &mut Buffer) {
    Paragraph::new(label).centered().render(area, buf);
}
