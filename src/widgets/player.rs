use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{LineGauge, Paragraph, StatefulWidget, Widget},
};

use crate::{
    app_state::AppState, audio::AudioPlayer, metadata::Metadata, widgets::traits::InputWidget,
};

const VOL_STEP: f32 = 1.0;

pub struct PlayerWidget {}

impl PlayerWidget {
    pub fn new() -> PlayerWidget {
        PlayerWidget {}
    }

    fn change_volume(&mut self, state: &mut AppState, f: impl FnOnce(f32, f32) -> f32) {
        state.player.set_volume(f(state.player.volume(), VOL_STEP));
        state.set_volume_changed_time(Instant::now());
    }

    fn render_volume(&self, area: Rect, buf: &mut Buffer, state: &AppState) {
        let mut icon = "🔈";
        if state.player.volume() > 66.6 {
            icon = "🔊";
        } else if state.player.volume() > 33.3 {
            icon = "🔉"
        }

        let text = format!("{icon} {}%", state.player.volume());

        Paragraph::new(text)
            .right_aligned()
            .render(area.inner(Margin::new(3, 0)), buf);
    }
}

impl StatefulWidget for &PlayerWidget {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let [title_area, progress_area, button_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(1), Constraint::Max(2), Constraint::Max(3)])
            .areas(area);

        let [_, button_area, volume_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ])
            .areas(button_area);

        render_title_bar(title_area, buf, state.current_metadata());
        render_progress_bar(progress_area, buf, state);
        render_buttons(button_area, buf, &state.player);
        self.render_volume(volume_area, buf, state);
    }
}

fn render_title_bar(area: Rect, buf: &mut Buffer, metadata: Option<Arc<Metadata>>) {
    let Some(metadata) = metadata else { return };

    let mut delimiter1 = Span::default();
    if metadata.artist.is_some() || metadata.album.is_some() {
        delimiter1 = Span::from(" - ");
    }

    let mut delimiter2 = Span::default();
    if metadata.artist.is_some() && metadata.album.is_some() {
        delimiter2 = Span::from(", ");
    }

    let line = Line::from_iter([
        Span::from(&metadata.title),
        delimiter1,
        Span::from(metadata.artist.as_deref().unwrap_or("")),
        delimiter2,
        Span::from(metadata.album.as_deref().unwrap_or("")),
    ]);
    line.render(area.inner(Margin::new(1, 0)), buf);
}

fn render_progress_bar(area: Rect, buf: &mut Buffer, state: &mut AppState) {
    let (ratio, text) = gauge_data(state);

    let gauge = LineGauge::default()
        .ratio(ratio)
        .label(text)
        .filled_style(Style::new().gray().on_white())
        .unfilled_style(Style::new().white());
    gauge.render(area.inner(Margin::new(1, 0)), buf);
}

fn gauge_data(state: &mut AppState) -> (f64, String) {
    if let Some(total) = state.player.total_duration {
        let total_secs = total.as_secs_f64();
        if total_secs != 0.0 {
            let ratio = state.player.position().as_secs_f64() / total_secs;
            let text = format!(
                "{}/{}",
                calculate_time_str(&state.player.position()),
                calculate_time_str(&total)
            );
            (ratio, text)
        } else {
            (0.0, String::from("00:00/00:00"))
        }
    } else {
        (0.0, String::from("00:00/00:00"))
    }
}

fn calculate_time_str(duration: &Duration) -> String {
    let total_secs = duration.as_secs();
    let min = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", min, secs)
}

fn render_buttons(area: Rect, buf: &mut Buffer, player: &AudioPlayer) {
    let button_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .split(area.centered_horizontally(Constraint::Ratio(1, 2)));

    let stopped = player.stopped();
    let paused = player.paused();
    let play_pause_symbol = if stopped || paused { "▶" } else { "⏸" };

    render_button("⏮", button_area[0], buf);
    render_button(play_pause_symbol, button_area[1], buf);
    render_button("⏭", button_area[2], buf);
}

fn render_button(label: &str, area: Rect, buf: &mut Buffer) {
    Paragraph::new(label).centered().render(area, buf);
}

impl InputWidget for PlayerWidget {
    fn on_input(&mut self, _modifier: KeyModifiers, key: KeyCode, state: &mut AppState) {
        match key {
            KeyCode::Up => self.change_volume(state, |vol, step| vol + step),
            KeyCode::Down => self.change_volume(state, |vol, step| vol - step),
            KeyCode::Right => state.player.skip(),
            KeyCode::Left => state.player.previous(),
            _ => {}
        }
    }
}
