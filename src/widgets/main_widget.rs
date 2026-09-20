use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    io,
    rc::Rc,
    sync::Arc,
};

use crate::{
    app_state::AppState,
    audio::{AudioControls, SourceQueue},
    enums::{CLMP3Widget, WidgetArea},
    widgets::{
        display::DisplayWidget, load::LoadWidget, player::PlayerWidget, queue::QueueWidget,
        traits::InputWidget,
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
    selected_widget: Cell<CLMP3Widget>,
    last_display_widget: Cell<CLMP3Widget>,
}

impl RenderSettings {
    pub fn get_selected_widget(&self) -> CLMP3Widget {
        self.selected_widget.get()
    }

    pub fn set_selected_widget(&self, val: CLMP3Widget) {
        self.selected_widget.set(val);
        if val.get_area() == WidgetArea::Display {
            self.set_last_display_widget(val);
        }
    }

    pub fn get_last_display_widget(&self) -> CLMP3Widget {
        self.last_display_widget.get()
    }

    fn set_last_display_widget(&self, val: CLMP3Widget) {
        self.last_display_widget.set(val);
    }
}

pub struct MainWidget {
    render_settings: Rc<RenderSettings>,
    input_widgets: HashMap<CLMP3Widget, Rc<RefCell<dyn InputWidget>>>,
    player_widget: Rc<RefCell<PlayerWidget>>,
    display_widget: Rc<RefCell<DisplayWidget>>,
    queue_widget: Rc<RefCell<QueueWidget>>,
    load_widget: Rc<RefCell<LoadWidget>>,
}

impl MainWidget {
    pub fn new(
        controls: Arc<AudioControls>,
        queue: Arc<SourceQueue>,
    ) -> (MainWidget, Rc<RenderSettings>) {
        let render_settings = Rc::new(RenderSettings {
            selected_widget: Cell::new(CLMP3Widget::Player),
            last_display_widget: Cell::new(CLMP3Widget::Display),
        });

        let player_widget = Rc::new(RefCell::new(PlayerWidget::new(controls)));
        let queue_widget = Rc::new(RefCell::new(QueueWidget::new(queue.clone())));
        let load_widget = Rc::new(RefCell::new(LoadWidget::new(queue)));

        let mut input_widgets: HashMap<CLMP3Widget, Rc<RefCell<dyn InputWidget>>> = HashMap::new();
        input_widgets.insert(CLMP3Widget::Player, player_widget.clone());
        input_widgets.insert(CLMP3Widget::Queue, queue_widget.clone());
        input_widgets.insert(CLMP3Widget::Load, load_widget.clone());

        (
            MainWidget {
                render_settings: render_settings.clone(),
                input_widgets: input_widgets,
                player_widget: player_widget,
                display_widget: Rc::new(RefCell::new(DisplayWidget::new())),
                queue_widget: queue_widget,
                load_widget: load_widget,
            },
            render_settings,
        )
    }

    pub fn on_navigation_key(&mut self, key: KeyCode, state: &mut AppState) {
        match self
            .input_widgets
            .get(&self.render_settings.get_selected_widget())
        {
            Some(widget) => widget.borrow_mut().on_input(key, state),
            None => {},
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

        let inner = display_block.inner(display_area);

        match self.render_settings.get_last_display_widget() {
            CLMP3Widget::Display => self.display_widget.borrow().render(inner, buf),
            CLMP3Widget::Load => self.load_widget.borrow().render(inner, buf),
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
            .borrow_mut()
            .render(queue_block.inner(queue_area), buf, state);

        queue_block.render(queue_area, buf);
    }

    fn render_player_area(&self, player_area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let player_block = Block::bordered()
            .title(Line::from(" Player Area "))
            .border_set(self.get_thickness(WidgetArea::Player));

        self.player_widget
            .borrow_mut()
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
