use std::{io, sync::Arc};

use crate::{
    audio::AudioControls,
    enums::CLMP3Widgets,
    widgets::{display::DisplayWidget, load::LoadWidget, player::PlayerWidget, queue::QueueWidget},
};

use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout, Rect},
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};

pub struct RenderSettings {
    is_loading: bool,
    selected_widget: CLMP3Widgets,
}

pub struct MainWidget {
    render_settings: RenderSettings,
    player_widget: PlayerWidget,
    display_widget: DisplayWidget,
    queue_widget: QueueWidget,
    load_widget: LoadWidget,
}

impl MainWidget {
    pub fn new(controls: Arc<AudioControls>) -> MainWidget {
        MainWidget {
            render_settings: RenderSettings {
                is_loading: false,
                selected_widget: CLMP3Widgets::Player,
            },
            player_widget: PlayerWidget::new(controls),
            display_widget: DisplayWidget::new(),
            queue_widget: QueueWidget::new(),
            load_widget: LoadWidget::new(),
        }
    }

    pub fn set_is_loading(&mut self, val: bool) {
        self.render_settings.is_loading = val;
    }

    pub fn set_selected_widget(&mut self, val: CLMP3Widgets) {
        self.render_settings.selected_widget = val;
    }

    pub fn draw(&self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        terminal.draw(|frame| frame.render_widget(self, frame.area()))?;
        Ok(())
    }

    fn render_display_area(&self, display_area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let display_block = Block::bordered()
            .title(Line::from(" Display Area "))
            .border_set(self.get_thickness(CLMP3Widgets::Display));

        if self.render_settings.is_loading {
            self.load_widget
                .render(display_block.inner(display_area), buf);
        } else {
            self.display_widget
                .render(display_block.inner(display_area), buf);
        }

        display_block.render(display_area, buf);
    }

    fn render_queue_area(&self, queue_area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let queue_block = Block::bordered()
            .title(Line::from(" Queue Area "))
            .border_set(self.get_thickness(CLMP3Widgets::Queue));

        self.queue_widget.render(queue_block.inner(queue_area), buf);

        queue_block.render(queue_area, buf);
    }

    fn render_player_area(&self, player_area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let player_block = Block::bordered()
            .title(Line::from(" Player Area "))
            .border_set(self.get_thickness(CLMP3Widgets::Player));

        self.player_widget
            .render(player_block.inner(player_area), buf);

        player_block.render(player_area, buf);
    }

    fn get_thickness(&self, check: CLMP3Widgets) -> border::Set<'static> {
        if self.render_settings.selected_widget == check {
            border::THICK
        } else {
            border::PLAIN
        }
    }
}

impl Widget for &MainWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let player_height = ((area.height as f32 * 0.15f32) as u16).clamp(10, 50);

        let [display_area, player_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(100),
                Constraint::Length(player_height),
            ])
            .areas(area);

        let queue_width = ((display_area.width as f32 * 0.2f32) as u16).clamp(30, 100);

        let [display_area, queue_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Length(queue_width)])
            .areas(display_area);

        self.render_display_area(display_area, buf);
        self.render_queue_area(queue_area, buf);
        self.render_player_area(player_area, buf);
    }
}
