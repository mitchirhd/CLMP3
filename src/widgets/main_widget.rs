use crate::widgets::{display::DisplayWidget, player::PlayerWidget, queue::QueueWidget};

use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};

pub struct MainWidget {
    player_widget: PlayerWidget,
    display_widget: DisplayWidget,
    queue_widget: QueueWidget,
}

impl MainWidget {
    pub fn new() -> MainWidget {
        MainWidget {
            player_widget: PlayerWidget::new(),
            display_widget: DisplayWidget::new(),
            queue_widget: QueueWidget::new(),
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

        self.display_widget.render(display_area, buf);
        self.queue_widget.render(queue_area, buf);
        self.player_widget.render(player_area, buf);
    }
}
