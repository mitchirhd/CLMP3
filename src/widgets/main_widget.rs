use std::io;

use crate::{
    app_state::AppState, enums::{Clmp3Widget, WidgetArea}, widgets::{
        display::DisplayWidget, load::LoadWidget, player::PlayerWidget, queue::QueueWidget, render_helper::{
            border_button_style, border_text_style, border_title_first_style, border_title_style, messages_widget, thickness,
        }, traits::InputWidget,
    },
};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, StatefulWidget, Widget},
};
use tui_slider::{Slider, SliderOrientation};

const MIN_TERMINAL_HEIGHT: u16 = 20;
const MIN_TERMINAL_WIDTH: u16 = 80;

pub struct MainWidget {
    player_widget: PlayerWidget,
    display_widget: DisplayWidget,
    queue_widget: QueueWidget,
    load_widget: LoadWidget,
}

impl MainWidget {
    pub fn new() -> MainWidget {
        MainWidget {
            player_widget: PlayerWidget::new(),
            display_widget: DisplayWidget::new(),
            queue_widget: QueueWidget::new(),
            load_widget: LoadWidget::new(),
        }
    }

    pub fn widget_controls(&mut self, modifier: KeyModifiers, key: KeyCode, state: &mut AppState) {
        let input_widget: Option<&mut dyn InputWidget> = match state.selected_widget() {
            Clmp3Widget::Player => Some(&mut self.player_widget),
            Clmp3Widget::Display => None,
            Clmp3Widget::Load => Some(&mut self.load_widget),
            Clmp3Widget::Queue => Some(&mut self.queue_widget),
        };
        if let Some(input_widget) = input_widget {
            input_widget.on_input(modifier, key, state);
        }
    }

    pub fn draw(&mut self, terminal: &mut DefaultTerminal, state: &mut AppState) -> io::Result<()> {
        if terminal.size().unwrap().width < MIN_TERMINAL_WIDTH
            || terminal.size().unwrap().height < MIN_TERMINAL_HEIGHT
        {
            let msg = messages_widget(&[
                String::from("Terminal too small"),
                format!("Minimal width: {MIN_TERMINAL_WIDTH} lines"),
                format!("Minimal height: {MIN_TERMINAL_HEIGHT} lines"),
            ]);
            terminal.draw(|frame| frame.render_widget(&msg, frame.area()))?;
        } else {
            terminal.draw(|frame| frame.render_stateful_widget(&mut *self, frame.area(), state))?;
        }
        Ok(())
    }

    fn render_display_area(
        &self,
        display_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut AppState,
    ) {
        let (thickness, style) = thickness(WidgetArea::Display, state);
        let display_block = Block::bordered()
            .title(display_border_title(state, thickness))
            .title_bottom(display_border_bottom_title(state, thickness).unwrap_or_default())
            .border_set(thickness)
            .border_style(style);

        let inner = display_block.inner(display_area);

        match state.last_display_widget() {
            Clmp3Widget::Display => self.display_widget.render(inner, buf),
            Clmp3Widget::Load => self.load_widget.render(inner, buf),
            _ => {}
        }

        display_block.render(display_area, buf);
    }

    fn render_queue_area(
        &mut self,
        queue_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut AppState,
    ) {
        self.queue_widget.render(queue_area, buf, state);
    }

    fn render_player_area(
        &self,
        player_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut AppState,
    ) {
        self.player_widget.render(player_area, buf, state);
    }

    fn render_volume_slider(&self, area: Rect, buf: &mut Buffer, state: &mut AppState) {
        if state.draw_volume_slider() {
            let vol_block = Block::bordered()
                .title(Line::from("Vol"))
                .border_set(border::PLAIN);

            let vol_slider = Slider::new(state.player.volume() as f64, 0.0, 100.0)
                .orientation(SliderOrientation::Vertical)
                .filled_symbol("█")
                .empty_symbol("░")
                .handle_symbol("█")
                .block(vol_block);

            let [_, vol_area, _] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Fill(1), Constraint::Max(5), Constraint::Fill(1)])
                .areas(area);

            vol_slider.render(
                vol_area.inner(Margin {
                    horizontal: 0,
                    vertical: 3,
                }),
                buf,
            );
        }
    }
}

impl StatefulWidget for &mut MainWidget {
    type State = AppState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) where
        Self: Sized,
    {
        let [display_area, player_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Max(7)])
            .areas(area);

        let queue_width = ((display_area.width as f32 * 0.2f32) as u16).clamp(30, 100);

        let [display_area, queue_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Length(queue_width)])
            .areas(display_area);

        self.render_display_area(display_area, buf, state);
        self.render_queue_area(queue_area, buf, state);
        self.render_player_area(player_area, buf, state);
        self.render_volume_slider(area, buf, state);
    }
}

fn display_border_title(state: &AppState, border: border::Set<'static>) -> Line<'static> {
    let spans = vec![
        Span::styled(" d", border_title_first_style(Clmp3Widget::Display, state)),
        Span::styled("isplay ", border_title_style(Clmp3Widget::Display, state)),
        Span::raw(border.horizontal_top),
        Span::styled(" l", border_title_first_style(Clmp3Widget::Load, state)),
        Span::styled("oad ", border_title_style(Clmp3Widget::Load, state)),
    ];
    Line::from(spans)
}

fn display_border_bottom_title(
    state: &AppState,
    border: border::Set<'static>,
) -> Option<Line<'static>> {
    match state.last_display_widget() {
        Clmp3Widget::Display => None,
        Clmp3Widget::Load => {
            let button_style = border_button_style(Clmp3Widget::Load, state);
            let text_style = border_text_style(Clmp3Widget::Load, state);
            Some(Line::from(vec![
                Span::styled(" up ", text_style),
                Span::styled("🢀 ", button_style),
                Span::raw(border.horizontal_top),
                Span::raw(border.horizontal_top),
                Span::styled(" 🢁 ", button_style),
                Span::styled("select ", text_style),
                Span::styled("🢃 ", button_style),
                Span::raw(border.horizontal_top),
                Span::raw(border.horizontal_top),
                Span::styled(" load ", text_style),
                Span::styled("🢂 ", button_style),
                Span::raw(border.horizontal_top),
                Span::raw(border.horizontal_top),
                Span::styled(" load dir ", text_style),
                Span::styled("🢂 + shift ", button_style),
            ]))
        }
        _ => None,
    }
}
