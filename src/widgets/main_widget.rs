use std::{cell::RefCell, collections::HashMap, io, rc::Rc};

use crate::{
    app_state::AppState,
    enums::{CLMP3Widget, WidgetArea},
    widgets::{
        display::DisplayWidget, load::LoadWidget, player::PlayerWidget, queue::QueueWidget,
        traits::InputWidget,
    },
};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    symbols::border,
    text::Line,
    widgets::{Block, StatefulWidget, Widget},
};
use tui_slider::{Slider, SliderOrientation};

pub struct MainWidget {
    input_widgets: HashMap<CLMP3Widget, Rc<RefCell<dyn InputWidget>>>,
    player_widget: Rc<RefCell<PlayerWidget>>,
    display_widget: Rc<RefCell<DisplayWidget>>,
    queue_widget: Rc<RefCell<QueueWidget>>,
    load_widget: Rc<RefCell<LoadWidget>>,
}

impl MainWidget {
    pub fn new() -> MainWidget {
        let player_widget = Rc::new(RefCell::new(PlayerWidget::new()));
        let queue_widget = Rc::new(RefCell::new(QueueWidget::new()));
        let load_widget = Rc::new(RefCell::new(LoadWidget::new()));

        let mut input_widgets: HashMap<CLMP3Widget, Rc<RefCell<dyn InputWidget>>> = HashMap::new();
        input_widgets.insert(CLMP3Widget::Player, player_widget.clone());
        input_widgets.insert(CLMP3Widget::Queue, queue_widget.clone());
        input_widgets.insert(CLMP3Widget::Load, load_widget.clone());

        MainWidget {
            input_widgets: input_widgets,
            player_widget: player_widget,
            display_widget: Rc::new(RefCell::new(DisplayWidget::new())),
            queue_widget: queue_widget,
            load_widget: load_widget,
        }
    }

    pub fn on_navigation_key(&mut self, modifier: KeyModifiers, key: KeyCode, state: &mut AppState) {
        match self.input_widgets.get(&state.get_selected_widget()) {
            Some(widget) => widget.borrow_mut().on_input(modifier, key, state),
            None => {}
        }
    }

    pub fn draw(&mut self, terminal: &mut DefaultTerminal, state: &mut AppState) -> io::Result<()> {
        terminal.draw(|frame| frame.render_stateful_widget(&*self, frame.area(), state))?;
        Ok(())
    }

    fn render_display_area(
        &self,
        display_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut AppState,
    ) {
        let display_block = Block::bordered()
            .title(Line::from(" Display Area "))
            .border_set(self.get_thickness(WidgetArea::Display, state));

        let inner = display_block.inner(display_area);

        match state.get_last_display_widget() {
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
        state: &mut AppState,
    ) {
        let queue_block = Block::bordered()
            .title(Line::from(" Queue Area "))
            .border_set(self.get_thickness(WidgetArea::Queue, state));

        if state.get_selected_widget() != CLMP3Widget::Queue {}

        self.queue_widget
            .borrow_mut()
            .render(queue_block.inner(queue_area), buf, state);

        queue_block.render(queue_area, buf);
    }

    fn render_player_area(
        &self,
        player_area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut AppState,
    ) {
        let player_block = Block::bordered()
            .title(Line::from(" Player Area "))
            .border_set(self.get_thickness(WidgetArea::Player, state));

        self.player_widget
            .borrow_mut()
            .render(player_block.inner(player_area), buf, state);

        player_block.render(player_area, buf);
    }

    fn render_volume_slider(&self, area: Rect, buf: &mut Buffer, state: &mut AppState) {
        if state.draw_volume_slider() {
            let vol_block = Block::bordered()
                .title(Line::from("Vol"))
                .border_set(border::PLAIN);

            let vol_slider = Slider::from_state(&state.volume_state)
                .orientation(SliderOrientation::Vertical)
                .filled_symbol("█")
                .empty_symbol("░")
                .handle_symbol("█")
                .block(vol_block);

            let [_, vol_area, _] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Fill(1), Constraint::Max(5), Constraint::Fill(1)])
                .areas(area);

            vol_slider.render(
                vol_area.inner(Margin {
                    horizontal: 0,
                    vertical: 3,
                }),
                buf,
            );
        }
    }

    fn get_thickness(&self, check: WidgetArea, state: &AppState) -> border::Set<'static> {
        if state.get_selected_widget().get_area() == check {
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
        // let player_height = ((area.height as f32 * 0.15f32) as u16).clamp(10, 50);

        let [display_area, player_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Max(8)])
            .areas(area);

        let queue_width = ((display_area.width as f32 * 0.2f32) as u16).clamp(30, 100);

        let [display_area, queue_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Length(queue_width)])
            .areas(display_area);

        self.render_display_area(display_area, buf, state);
        self.render_queue_area(queue_area, buf, state);
        self.render_player_area(player_area, buf, state);
        self.render_volume_slider(area, buf, state);
    }
}
