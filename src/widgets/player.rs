use ratatui::{symbols::border, text::Line, widgets::{Block, Gauge, Widget}};

pub struct PlayerWidget {}

impl PlayerWidget {
    pub fn new() -> PlayerWidget {
        PlayerWidget {}
    }
}

impl Widget for &PlayerWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let player_block = Block::bordered()
            .title(Line::from(" Player Area "))
            .border_set(border::THICK);

        player_block.render(area, buf);

        // let progress_bar = Gauge::default()
    }
}
