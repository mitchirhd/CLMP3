use ratatui::{symbols::border, text::Line, widgets::{Block, Widget}};

pub struct QueueWidget {}

impl QueueWidget {
    pub fn new() -> QueueWidget {
        QueueWidget {}
    }
}

impl Widget for &QueueWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let queue_block = Block::bordered()
            .title(Line::from(" Queue Area "))
            .border_set(border::THICK);

        queue_block.render(area, buf);
    }
}
