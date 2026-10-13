use anyhow::Ok;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    symbols::border,
    text::{Line, Span},
    widgets::{Block, LineGauge, Paragraph, StatefulWidget, Widget},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    app_state::AppState,
    enums::{Clmp3Error, Clmp3Widget, WidgetArea},
    metadata::Metadata,
    widgets::{
        render_helper::{
            border_button_style, border_text_style, border_title_first_style, border_title_style,
            thickness,
        },
        traits::InputWidget,
    },
};

const VOL_STEP: f32 = 1.0;

pub struct PlayerWidget {}

impl PlayerWidget {
    pub fn new() -> PlayerWidget {
        PlayerWidget {}
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
        let (thickness, style) = thickness(WidgetArea::Player, state);
        let (bottom_title_left, bottom_title_right) = border_title_bottom(state, thickness);
        let player_block = Block::bordered()
            .title(border_title(state))
            .title_bottom(bottom_title_left)
            .title_bottom(bottom_title_right)
            .border_set(thickness)
            .border_style(style);

        let [progress_area, rest_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(3)])
            .areas(player_block.inner(area));

        let [title_area, volume_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(2), Constraint::Fill(1)])
            .areas(rest_area);

        render_song_infos(title_area, buf, state.current_metadata());
        render_progress_bar(progress_area, buf, state);
        self.render_volume(volume_area, buf, state);
        player_block.render(area, buf);
    }
}

fn border_title(state: &AppState) -> Line<'static> {
    let spans = vec![
        Span::styled(" p", border_title_first_style(Clmp3Widget::Player, state)),
        Span::styled("layer ", border_title_style(Clmp3Widget::Player, state)),
    ];
    Line::from(spans)
}

fn border_title_bottom(
    state: &mut AppState,
    border: border::Set<'static>,
) -> (Line<'static>, Line<'static>) {
    let play_pause_symbol = if state.player.stopped() || state.player.paused() {
        "▶"
    } else {
        "⏸"
    };

    let button_style = border_button_style(Clmp3Widget::Player, state);
    let text_style = border_text_style(Clmp3Widget::Player, state);

    let left_spans = vec![
        Span::styled(" ⏮ ", text_style),
        Span::styled("🢀 ", button_style),
        Span::raw(border.horizontal_top),
        Span::raw(border.horizontal_top),
        Span::styled(format!(" {play_pause_symbol} "), Style::default().white()),
        Span::styled("space ", Style::default().light_blue()),
        Span::raw(border.horizontal_top),
        Span::raw(border.horizontal_top),
        Span::styled(" ⏭ ", text_style),
        Span::styled("🢂 ", button_style),
        Span::raw(border.horizontal_top),
        Span::raw(border.horizontal_top),
        Span::styled(" scrub ", text_style),
        Span::styled("shift + 🢀 🢂 ", button_style),
    ];

    let right_spans = vec![
        Span::styled(" 🢁 ", button_style),
        Span::styled("vol ", text_style),
        Span::styled("🢃 ", button_style),
    ];

    (
        Line::from(left_spans).left_aligned(),
        Line::from(right_spans).right_aligned(),
    )
}

fn render_song_infos(area: Rect, buf: &mut Buffer, metadata: Option<Arc<Metadata>>) {
    let Some(metadata) = metadata else { return };

    let [title_area, artist_area, album_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .horizontal_margin(1)
    .areas(area);

    Line::raw(&metadata.title).render(title_area, buf);
    Line::raw(metadata.artist.as_deref().unwrap_or(""))
        .style(Style::default().dark_gray())
        .render(artist_area, buf);
    Line::raw(metadata.album.as_deref().unwrap_or(""))
        .style(Style::default().dark_gray())
        .render(album_area, buf);
}

fn render_progress_bar(area: Rect, buf: &mut Buffer, state: &AppState) {
    let (ratio, text) = gauge_data(state);

    let gauge = LineGauge::default()
        .ratio(ratio)
        .label(text)
        .filled_style(Style::new().blue().on_light_blue())
        .unfilled_style(Style::new().white());
    gauge.render(area.inner(Margin::new(1, 0)), buf);
}

fn gauge_data(state: &AppState) -> (f64, String) {
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

fn change_volume(state: &mut AppState, f: impl FnOnce(f32, f32) -> f32) {
    state.player.set_volume(f(state.player.volume(), VOL_STEP));
    state.set_volume_changed_time(Instant::now());
}

fn on_left(state: &mut AppState, modifier: KeyModifiers) {
    let result = if modifier == KeyModifiers::SHIFT {
        state.player.scrub(true)
    } else {
        state.player.previous()
    };

    if let Err(e) = result {
        state.last_error = Some((Clmp3Error::AudioError, vec![format!("{e:#}")]));
    }
}

fn on_right(state: &mut AppState, modifier: KeyModifiers) {
    if modifier == KeyModifiers::SHIFT {
        if let Err(e) = state.player.scrub(false) {
            state.last_error = Some((Clmp3Error::AudioError, vec![format!("{e:#}")]));
        }
    } else {
        state.player.skip();
    }
}

impl InputWidget for PlayerWidget {
    fn on_input(
        &mut self,
        modifier: KeyModifiers,
        key: KeyCode,
        state: &mut AppState,
    ) -> anyhow::Result<()> {
        match key {
            KeyCode::Up => change_volume(state, |vol, step| vol + step),
            KeyCode::Down => change_volume(state, |vol, step| vol - step),
            KeyCode::Right => on_right(state, modifier),
            KeyCode::Left => on_left(state, modifier),
            _ => {}
        };
        Ok(())
    }
}
