use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::{Widget, WidgetRef};
use ratatui_explorer::{FileExplorer, FileExplorerBuilder, Input, Theme};

use crate::{
    app_state::AppState, audio::AudioPlayer, source_builder::SourceType,
    widgets::traits::InputWidget,
};

pub struct LoadWidget {
    file_explorer: FileExplorer,
}

impl LoadWidget {
    pub fn new() -> LoadWidget {
        let mut file_explorer =
            FileExplorerBuilder::build_with_theme(Theme::default().add_default_title())
                .expect("Could not create file explorer");

        file_explorer
            .set_filter_map(|file| {
                if AudioPlayer::can_play(&file.path) || file.is_dir {
                    Some(file)
                } else {
                    None
                }
            })
            .unwrap();

        LoadWidget { file_explorer }
    }

    fn on_enter(&mut self, state: &mut AppState) -> Input {
        if self.file_explorer.current().is_dir {
            Input::Right
        } else {
            self.load_current(state);
            Input::None
        }
    }

    fn load_current(&mut self, state: &mut AppState) {
        let current = self.file_explorer.current();
        if !current.is_file() {
            return;
        }

        if AudioPlayer::can_play(&current.path) {
            state.queue.enqueue(SourceType::File(current.clone()));
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

impl InputWidget for LoadWidget {
    fn on_input(&mut self, _modifier: KeyModifiers, key: KeyCode, state: &mut AppState) {
        let input = match key {
            KeyCode::Backspace => Input::Left,
            KeyCode::Left => Input::Left,
            KeyCode::Right => self.on_enter(state),
            KeyCode::Up => Input::Up,
            KeyCode::Down => Input::Down,
            KeyCode::Home => Input::Home,
            KeyCode::End => Input::End,
            KeyCode::PageUp => Input::PageUp,
            KeyCode::PageDown => Input::PageDown,
            KeyCode::Tab => Input::None,
            KeyCode::Enter => self.on_enter(state),
            _ => Input::None,
        };

        self.file_explorer
            .handle(input)
            .expect("Directory cannot be listed")
    }
}
