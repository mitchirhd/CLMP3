use ratatui::{
    layout::Constraint,
    style::Style,
    symbols::border,
    text::Line,
    widgets::{Paragraph, Widget, Wrap},
};

use crate::{
    app_state::AppState,
    enums::{Clmp3Widget, WidgetArea},
};

pub fn thickness(check: WidgetArea, state: &AppState) -> (border::Set<'static>, Style) {
    if state.selected_widget().area() == check {
        (border::THICK, Style::default().light_blue())
    } else {
        (border::PLAIN, Style::default().white())
    }
}

pub fn border_title_first_style(check: Clmp3Widget, state: &AppState) -> Style {
    if state.selected_widget() == check {
        Style::default().blue().bold()
    } else {
        Style::default().blue()
    }
}

pub fn border_title_style(check: Clmp3Widget, state: &AppState) -> Style {
    if state.selected_widget() == check {
        Style::default().light_blue().bold()
    } else {
        Style::default().white()
    }
}

pub fn border_button_style(check: Clmp3Widget, state: &AppState) -> Style {
    if state.selected_widget() == check {
        Style::default().light_blue()
    } else {
        Style::default().dark_gray()
    }
}

pub fn border_text_style(check: Clmp3Widget, state: &AppState) -> Style {
    if state.selected_widget() == check {
        Style::default().white()
    } else {
        Style::default().dark_gray()
    }
}

pub struct MessageWidget {
    lines: Vec<String>,
}

impl Widget for &MessageWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let lines: Vec<Line> = self
            .lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect();
        let area = area.centered_vertically(Constraint::Length(lines.len() as u16));
        Paragraph::new(lines)
            .centered()
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}

// pub fn message_widget(msg: String) -> MessageWidget {
//     MessageWidget {
//         lines: Vec::from([msg]),
//     }
// }

pub fn messages_widget(messages: &[String]) -> MessageWidget {
    MessageWidget {
        lines: Vec::from(messages),
    }
}
