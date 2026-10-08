use std::{fs, path::Path, sync::Arc, thread};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Widget, WidgetRef},
};
use ratatui_explorer::{FileExplorer, FileExplorerBuilder, Input, Theme};

use crate::{
    app_state::AppState,
    audio::AudioPlayer,
    metadata::MetadataLoader,
    source_builder::{SourceId, SourceType},
    source_queue::SharedQueue,
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
                Self::load_dir(
                    state.player.queue.clone(),
                    state.metadata_loader.clone(),
                    &current.path.clone(),
                );
                Input::None
            } else {
                Input::Right
            }
        } else {
            Self::load_file(
                state.player.queue.clone(),
                state.metadata_loader.clone(),
                &current.path.clone(),
            );
            Input::None
        }
    }

    fn load_file(queue: SharedQueue, loader: Arc<MetadataLoader>, path: &Path) {
        let path = path.to_path_buf();
        thread::spawn(move || {
            Self::load_file_internal(&queue, &loader, &path);
        });
    }

    fn load_file_internal(queue: &SharedQueue, loader: &Arc<MetadataLoader>, path: &Path) {
        let file = fs::metadata(path).unwrap();
        if !file.is_file() || !AudioPlayer::can_play(path) {
            return;
        }

        queue.enqueue(SourceType::File(path.to_path_buf()));
        let id = SourceId::File(path.to_path_buf());
        loader.load(id, path.to_path_buf());
    }

    fn load_dir(queue: SharedQueue, loader: Arc<MetadataLoader>, start_path: &Path) {
        let start_path = start_path.to_path_buf();
        thread::spawn(move || {
            Self::load_dir_internal(&queue, &loader, &start_path);
        });
    }

    fn load_dir_internal(queue: &SharedQueue, loader: &Arc<MetadataLoader>, start_path: &Path) {
        let Ok(dir) = fs::read_dir(start_path.to_string_lossy().into_owned()) else {
            return;
        };
        for dir_entry in dir {
            let Ok(dir_entry) = dir_entry else {
                continue;
            };
            if dir_entry.file_type().unwrap().is_dir() {
                Self::load_dir_internal(queue, loader, &dir_entry.path());
            } else {
                Self::load_file_internal(queue, loader, &dir_entry.path());
            }
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
    fn on_input(&mut self, modifier: KeyModifiers, key: KeyCode, state: &mut AppState) {
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

        if let Err(_err) = self.file_explorer.handle(input) {
            // state.set_error(format!("Directory cannot be listed: {err}")); TODO
        }
    }
}
