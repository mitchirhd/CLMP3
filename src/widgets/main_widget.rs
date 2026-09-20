use std::{
    io,
    sync::{Arc, Mutex},
};

use crate::{
    app_state::AppState,
    audio::{AudioControls, SourceQueue},
    enums::{CLMP3Widget, WidgetArea},
    widgets::{
        display::DisplayWidget,
        load::LoadWidget,
        player::PlayerWidget,
        queue::QueueWidget,
        traits::{InputWidget, StatefulInputWidget},
    },
};

use crossterm::event::KeyCode;
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout, Rect},
    symbols::border,
    text::Line,
    widgets::{Block, ListState, StatefulWidget, Widget},
};

pub struct RenderSettings {
    selected_widget: Mutex<CLMP3Widget>,
    last_display_widget: Mutex<CLMP3Widget>,
}

impl RenderSettings {
    pub fn get_selected_widget(&self) -> CLMP3Widget {
        *self.selected_widget.lock().expect("Mutex acquired")
    }

    pub fn set_selected_widget(&self, val: CLMP3Widget) {
        *self.selected_widget.lock().expect("Mutex acquired") = val;
        if val.get_area() == WidgetArea::Display {
            self.set_last_display_widget(val);
        }
    }

    fn set_last_display_widget(&self, val: CLMP3Widget) {
        *self.last_display_widget.lock().expect("Mutex acquired") = val
    }

    pub fn get_last_display_widget(&self) -> CLMP3Widget {
        *self.last_display_widget.lock().expect("Mutex acquired")
    }
}

pub struct MainWidget {
    render_settings: Arc<RenderSettings>,
    player_widget: PlayerWidget,
    display_widget: DisplayWidget,
    queue_widget: QueueWidget,
    load_widget: LoadWidget,
}

impl MainWidget {
    pub fn new(
        controls: Arc<AudioControls>,
        queue: Arc<SourceQueue>,
    ) -> (MainWidget, Arc<RenderSettings>) {
        let render_settings = Arc::new(RenderSettings {
            selected_widget: Mutex::new(CLMP3Widget::Player),
            last_display_widget: Mutex::new(CLMP3Widget::Display),
        });
        (
            MainWidget {
                render_settings: render_settings.clone(),
                player_widget: PlayerWidget::new(controls),
                display_widget: DisplayWidget::new(),
                queue_widget: QueueWidget::new(queue.clone()),
                load_widget: LoadWidget::new(queue),
            },
            render_settings,
        )
    }

    pub fn on_navigation_key(&mut self, key: KeyCode, state: &mut AppState) {
        enum InputWidgetKind<'a> {
            Regular(&'a mut dyn InputWidget),
            Stateful(&'a mut dyn StatefulInputWidget),
        }

        // let widget_kind: Option<&mut dyn InputWidget> = match self.render_settings.get_selected_widget()
        let widget_kind: InputWidgetKind = match self.render_settings.get_selected_widget() {
            CLMP3Widget::Player => return,
            CLMP3Widget::Display => return,
            CLMP3Widget::Load => InputWidgetKind::Regular(&mut self.load_widget),
            CLMP3Widget::Queue => InputWidgetKind::Stateful(&mut self.queue_widget),
        };

        match widget_kind {
            InputWidgetKind::Regular(input_widget) => input_widget.on_input(key),
            InputWidgetKind::Stateful(stateful_input_widget) => {
                stateful_input_widget.on_input(key, state)
            }
        }
    }

    pub fn draw(&mut self, terminal: &mut DefaultTerminal, state: &mut AppState) -> io::Result<()> {
        terminal.draw(|frame| frame.render_stateful_widget(&*self, frame.area(), state))?;
        Ok(())
    }

    fn render_display_area(&self, display_area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let display_block = Block::bordered()
            .title(Line::from(" Display Area "))
            .border_set(self.get_thickness(WidgetArea::Display));

        match self.render_settings.get_last_display_widget() {
            CLMP3Widget::Display => self
                .display_widget
                .render(display_block.inner(display_area), buf),
            CLMP3Widget::Load => self
                .load_widget
                .render(display_block.inner(display_area), buf),
            _ => {}
        }

        display_block.render(display_area, buf);
    }

    fn render_queue_area(
        &self,
        queue_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut ListState,
    ) {
        let queue_block = Block::bordered()
            .title(Line::from(" Queue Area "))
            .border_set(self.get_thickness(WidgetArea::Queue));

        if self.render_settings.get_selected_widget() != CLMP3Widget::Queue {}

        self.queue_widget
            .render(queue_block.inner(queue_area), buf, state);

        queue_block.render(queue_area, buf);
    }

    fn render_player_area(&self, player_area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let player_block = Block::bordered()
            .title(Line::from(" Player Area "))
            .border_set(self.get_thickness(WidgetArea::Player));

        self.player_widget
            .render(player_block.inner(player_area), buf);

        player_block.render(player_area, buf);
    }

    fn get_thickness(&self, check: WidgetArea) -> border::Set<'static> {
        if self.render_settings.get_selected_widget().get_area() == check {
            border::THICK
        } else {
            border::PLAIN
        }
    }
}

impl StatefulWidget for &MainWidget {
    type State = AppState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) where
        Self: Sized,
    {
        let player_height = ((area.height as f32 * 0.15f32) as u16).clamp(10, 50);

        let [display_area, player_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(100),
                Constraint::Length(player_height),
            ])
            .areas(area);

        let queue_width = ((display_area.width as f32 * 0.2f32) as u16).clamp(30, 100);

        let [display_area, queue_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Length(queue_width)])
            .areas(display_area);

        self.render_display_area(display_area, buf);
        self.render_queue_area(queue_area, buf, &mut state.queue_state);
        self.render_player_area(player_area, buf);
    }
}
