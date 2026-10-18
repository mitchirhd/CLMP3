use std::sync::Arc;

use crossterm::event::KeyEvent;
use image::DynamicImage;

use crate::source_builder::SourceType;

pub enum Clmp3Event {
    KeyInput(KeyEvent),
    Redraw,
    NewSongStarted,
    CoverLoaded(Arc<SourceType>, Option<DynamicImage>),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Clmp3Widget {
    Player,
    Display,
    Load,
    Queue,
    Online,
}

impl Clmp3Widget {
    pub fn area(&self) -> WidgetArea {
        match self {
            Clmp3Widget::Player => WidgetArea::Player,
            Clmp3Widget::Display => WidgetArea::Display,
            Clmp3Widget::Load => WidgetArea::Display,
            Clmp3Widget::Queue => WidgetArea::Queue,
            Clmp3Widget::Online => WidgetArea::Display,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WidgetArea {
    Player,
    Display,
    Queue,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Clmp3Error {
    TerminalSize,
    AudioError,
    InputError,
}
