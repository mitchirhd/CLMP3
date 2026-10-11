use image::DynamicImage;
use ratatui::{
    layout::Constraint,
    widgets::{StatefulWidget, Widget},
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};

pub struct DisplayWidget {
    picker: Picker,
    image_state: Option<StatefulProtocol>,
}

impl DisplayWidget {
    pub fn new() -> anyhow::Result<DisplayWidget> {
        let picker = Picker::from_query_stdio()?;
        Ok(DisplayWidget {
            picker,
            image_state: None,
        })
    }

    pub fn update_state(&mut self, new_image: Option<DynamicImage>) {
        self.image_state = new_image.map(|img| self.picker.new_resize_protocol(img));
    }
}

impl Widget for &mut DisplayWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        if let Some(state) = self.image_state.as_mut() {
            let resize = Resize::Scale(None);
            let size = state.size_for(resize.clone(), area.as_size());
            let centered = area.centered(
                Constraint::Length(size.width),
                Constraint::Length(size.height),
            );

            let image = StatefulImage::default().resize(resize);
            image.render(centered, buf, state);
        }
    }
}
