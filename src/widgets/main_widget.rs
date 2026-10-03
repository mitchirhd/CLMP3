use std::io;

use crate::{
    app_state::AppState,
    enums::{Clmp3Widget, WidgetArea},
    widgets::{
        display::DisplayWidget, load::LoadWidget, player::PlayerWidget, queue::QueueWidget,
        traits::InputWidget,
    },
};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    symbols::border,
    text::Line,
    widgets::{Block, StatefulWidget, Widget},
};
use tui_slider::{Slider, SliderOrientation};

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

    pub fn on_navigation_key(
        &mut self,
        modifier: KeyModifiers,
        key: KeyCode,
        state: &mut AppState,
    ) {
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
        terminal.draw(|frame| frame.render_stateful_widget(&*self, frame.area(), state))?;
        Ok(())
    }

    fn render_display_area(
        &self,
        display_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut AppState,
    ) {
        let (thickness, style) = self.thickness(WidgetArea::Display, state);
        let display_block = Block::bordered()
            .title(Line::from(" Display Area "))
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
        &self,
        queue_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut AppState,
    ) {
        let (thickness, style) = self.thickness(WidgetArea::Queue, state);
        let queue_block = Block::bordered()
            .title(Line::from(" Queue Area "))
            .border_set(thickness)
            .border_style(style);

        self.queue_widget
            .render(queue_block.inner(queue_area), buf, state);

        queue_block.render(queue_area, buf);
    }

    fn render_player_area(
        &self,
        player_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut AppState,
    ) {
        let (thickness, style) = self.thickness(WidgetArea::Player, state);
        let player_block = Block::bordered()
            .title(Line::from(" Player Area "))
            .border_set(thickness)
            .border_style(style);

        self.player_widget
            .render(player_block.inner(player_area), buf, state);

        player_block.render(player_area, buf);
    }

    fn render_volume_slider(&self, area: Rect, buf: &mut Buffer, state: &mut AppState) {
        if state.draw_volume_slider() {
            let vol_block = Block::bordered()
                .title(Line::from("Vol"))
                .border_set(border::PLAIN);

            let vol_slider = Slider::from_state(&state.volume_state)
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

    fn thickness(&self, check: WidgetArea, state: &AppState) -> (border::Set<'static>, Style) {
        if state.selected_widget().area() == check {
            (border::THICK, Style::default().blue())
        } else {
            (border::PLAIN, Style::default().white())
        }
    }
}

impl StatefulWidget for &MainWidget {
    type State = AppState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) where
        Self: Sized,
    {
        // let player_height = ((area.height as f32 * 0.15f32) as u16).clamp(10, 50);

        let [display_area, player_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Max(8)])
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
