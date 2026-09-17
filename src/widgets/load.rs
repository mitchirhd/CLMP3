use ratatui::widgets::{Widget, WidgetRef};
use ratatui_explorer::FileExplorer;

pub struct LoadWidget {
    file_explorer: FileExplorer,
}

impl LoadWidget {
    pub fn new() -> LoadWidget {
        LoadWidget {
            file_explorer: FileExplorer::new().expect("Could not create file explorer"),
        }
    }
}

impl Widget for &LoadWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        self.file_explorer.widget().render_ref(area, buf);
    }
}
