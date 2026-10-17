use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Widget, WidgetRef},
};
use ratatui_explorer::{FileExplorer, FileExplorerBuilder, Input, Theme};

use crate::{
    app_state::AppState, audio::AudioPlayer, local_file_helper::LocalFileHelper,
    widgets::traits::InputWidget,
};

pub struct LoadWidget {
    file_explorer: FileExplorer,
}

impl LoadWidget {
    pub fn new() -> LoadWidget {
        let mut file_explorer = FileExplorerBuilder::build_with_theme(
            Theme::default()
                .with_dir_style(Style::default().fg(Color::LightBlue))
                .with_highlight_dir_style(Style::default().fg(Color::White).bg(Color::DarkGray))
                .with_block(Block::default().borders(Borders::NONE)),
        )
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

    fn open_current(&mut self, modifier: KeyModifiers, state: &mut AppState) -> Input {
        let current = self.file_explorer.current();
        if current.is_dir {
            if modifier == KeyModifiers::SHIFT {
                LocalFileHelper::load_dir(
                    state.player.queue.clone(),
                    state.metadata_loader.clone(),
                    &current.path.clone(),
                );
                Input::None
            } else {
                Input::Right
            }
        } else {
            LocalFileHelper::load_file(
                state.player.queue.clone(),
                state.metadata_loader.clone(),
                &current.path.clone(),
            );
            Input::None
        }
    }
}

impl Widget for &LoadWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let [cwd_area, explorer_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .areas(area);
        Line::from(self.file_explorer.cwd().to_string_lossy().into_owned()).render(cwd_area, buf);
        self.file_explorer.widget().render_ref(explorer_area, buf);
    }
}

impl InputWidget for LoadWidget {
    fn on_input(
        &mut self,
        modifier: KeyModifiers,
        key: KeyCode,
        state: &mut AppState,
    ) -> anyhow::Result<()> {
        let input = match key {
            KeyCode::Left | KeyCode::Backspace => Input::Left,
            KeyCode::Right | KeyCode::Enter => self.open_current(modifier, state),
            KeyCode::Up => Input::Up,
            KeyCode::Down => Input::Down,
            KeyCode::Home => Input::Home,
            KeyCode::End => Input::End,
            KeyCode::PageUp => Input::PageUp,
            KeyCode::PageDown => Input::PageDown,
            _ => Input::None,
        };

        self.file_explorer.handle(input)?;
        Ok(())
    }
}
