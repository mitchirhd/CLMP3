use std::sync::Arc;

use ratatui::widgets::{List, Widget};

use crate::audio::SourceQueue;

pub struct QueueWidget {
    queue: Arc<SourceQueue>,
}

impl QueueWidget {
    pub fn new(queue: Arc<SourceQueue>) -> QueueWidget {
        QueueWidget { queue: queue }
    }
}

impl Widget for &QueueWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let items = self.queue.as_vec().into_iter().map(|s| match s {
            crate::source_builder::SourceType::SineWave(freq, _) => format!("Sine {freq}"),
            crate::source_builder::SourceType::File(file) => format!("{}", file.name),
        });

        let list = List::new(items);

        list.render(area, buf);
    }
}
