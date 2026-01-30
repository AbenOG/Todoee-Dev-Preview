use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::app::FocusState;

pub struct FocusWidget<'a> {
    state: &'a FocusState,
}

impl<'a> FocusWidget<'a> {
    pub fn new(state: &'a FocusState) -> Self {
        Self { state }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let remaining = self.state.remaining_secs();
        let mins = remaining / 60;
        let secs = remaining % 60;

        let progress = if self.state.duration_secs > 0 {
            1.0 - (remaining as f64 / self.state.duration_secs as f64)
        } else {
            1.0
        };

        let bar_width = 30;
        let filled = (progress * bar_width as f64) as usize;
        let empty = bar_width - filled;
        let bar = format!("[{}{}]", "#".repeat(filled), "-".repeat(empty));

        let time_color = if remaining <= 60 {
            Color::Red
        } else if remaining <= 300 {
            Color::Yellow
        } else {
            Color::Green
        };

        let status = if self.state.paused { " PAUSED" } else { "" };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "FOCUS MODE",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                &self.state.todo_title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{:02}:{:02}{}", mins, secs, status),
                Style::default()
                    .fg(time_color)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(bar, Style::default().fg(time_color))),
            Line::from(""),
            Line::from(Span::styled(
                "Space: pause  q/Esc: cancel  Enter: complete",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Focus ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            )
            .alignment(Alignment::Center);

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }
}
