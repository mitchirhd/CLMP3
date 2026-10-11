use std::sync::Arc;

use ratatui::{
    layout::Constraint,
    widgets::{StatefulWidget, Widget},
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};

use crate::metadata::Metadata;

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

    pub fn update_state(&mut self, metadata: &Option<Arc<Metadata>>) {
        self.image_state = metadata
            .as_ref()
            .and_then(|md| md.pic.as_ref())
            .map(|img| self.picker.new_resize_protocol(img.clone()));
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
