use ratatui::widgets::Widget;

pub struct DisplayWidget {}

impl DisplayWidget {
    pub fn new() -> DisplayWidget {
        DisplayWidget {}
    }
}

impl Widget for &DisplayWidget {
    fn render(self, _area: ratatui::prelude::Rect, _buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // let display_block = Block::bordered()
        //     .title(Line::from(" Display Area "))
        //     .border_set(border::THICK);

        // display_block.render(area, buf);
    }
}
