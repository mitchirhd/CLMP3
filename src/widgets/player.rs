use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, SystemTime},
};

use crossterm::event::{Event::Key, KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{LineGauge, Paragraph, StatefulWidget, Widget},
};

use crate::{
    app_state::AppState, audio::AudioControls, source_builder::Metadata,
    widgets::traits::InputWidget,
};

pub struct PlayerWidget {}

impl PlayerWidget {
    pub fn new() -> PlayerWidget {
        PlayerWidget {}
    }

    fn change_volume(&self, state: &mut AppState, f: Rc<dyn Fn(&mut AppState, f64)>) {
        f(state, 1.0);
        state
            .audio_controls
            .set_volume(state.volume_state.value() as f32);
        state.set_volume_changed_time(SystemTime::now());
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

        render_title_bar(title_area, buf, state.audio_controls.get_metadata());
        render_progress_bar(progress_area, buf, &state.audio_controls);
        render_buttons(button_area, buf, &state.audio_controls);
        render_volume(volume_area, buf, state);
    }
}

fn render_title_bar(area: Rect, buf: &mut Buffer, metadata: Option<Metadata>) {
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
        Span::from(metadata.title),
        delimiter1,
        Span::from(metadata.artist.unwrap_or(String::from(""))),
        delimiter2,
        Span::from(metadata.album.unwrap_or(String::from(""))),
    ]);
    line.render(area.inner(Margin::new(1, 0)), buf);
}

fn render_progress_bar(area: Rect, buf: &mut Buffer, controls: &Arc<AudioControls>) {
    let (ratio, text) = get_gauge_data(controls);

    let gauge = LineGauge::default()
        .ratio(ratio)
        .label(text)
        .filled_style(Style::new().gray().on_white())
        .unfilled_style(Style::new().white());
    gauge.render(area.inner(Margin::new(1, 0)), buf);
}

fn get_gauge_data(controls: &Arc<AudioControls>) -> (f64, String) {
    match controls.get_total_duration() {
        Some(total) => {
            let total_secs = total.as_secs_f64();
            if total_secs != 0.0 {
                let ratio = controls.get_position().as_secs_f64() / total_secs;
                let text = format!(
                    "{}/{}",
                    calculate_time_str(&controls.get_position()),
                    calculate_time_str(&total)
                );
                (ratio, text)
            } else {
                (0.0, String::from("00:00/00:00"))
            }
        }
        None => (0.0, String::from("00:00/00:00")),
    }
}

fn calculate_time_str(duration: &Duration) -> String {
    let total_secs = duration.as_secs();
    let min = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", min, secs)
}

fn render_buttons(area: Rect, buf: &mut Buffer, controls: &Arc<AudioControls>) {
    let button_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .split(area.centered_horizontally(Constraint::Ratio(1, 2)));

    let stopped = controls.get_is_stopped();
    let paused = controls.get_is_paused();
    let play_pause_symbol = if stopped || paused { "▶" } else { "⏸" };

    render_button("⏮", button_area[0], buf);
    render_button(play_pause_symbol, button_area[1], buf);
    render_button("⏭", button_area[2], buf);
}

fn render_button(label: &str, area: Rect, buf: &mut Buffer) {
    Paragraph::new(label).centered().render(area, buf);
}

fn render_volume(area: Rect, buf: &mut Buffer, state: &mut AppState) {
    let mut icon = "🔈";
    if state.volume_state.value() > 66.6 {
        icon = "🔊";
    } else if state.volume_state.value() > 33.3 {
        icon = "🔉"
    }

    let text = format!("{icon} {}%", state.volume_state.value());

    Paragraph::new(text)
        .right_aligned()
        .render(area.inner(Margin::new(3, 0)), buf);
}

impl InputWidget for PlayerWidget {
    fn on_input(&mut self, modifier: KeyModifiers, key: KeyCode, state: &mut AppState) {
        match key {
            KeyCode::Up => {
                self.change_volume(state, Rc::new(|s, step| s.volume_state.increase(step)));
            }
            KeyCode::Down => {
                self.change_volume(state, Rc::new(|s, step| s.volume_state.decrease(step)));
            }
            KeyCode::Right => {
                state.audio_controls.set_skip_current(true);
            }
            _ => {}
        }
    }
}
