use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::app::FocusState;
use crate::tui::spinner::bracketed_progress;

pub struct FocusWidget<'a> {
    state: &'a FocusState,
    animation_frame: usize,
}

impl<'a> FocusWidget<'a> {
    pub fn new(state: &'a FocusState, animation_frame: usize) -> Self {
        Self {
            state,
            animation_frame,
        }
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

        let time_color = if remaining <= 60 {
            Color::Red
        } else if remaining <= 300 {
            Color::Yellow
        } else {
            Color::Green
        };

        // Animated header when paused
        let header = if self.state.paused {
            let blink = self.animation_frame % 4 < 2;
            if blink { "⏸ PAUSED" } else { "  PAUSED" }
        } else {
            "FOCUS MODE"
        };

        // Animated timer separator (blinks when not paused)
        let separator = if self.state.paused {
            ':'
        } else {
            [' ', ':'][self.animation_frame % 2]
        };

        // Enhanced progress bar
        let bar = bracketed_progress(progress, 32);

        // Motivational messages based on progress
        let motivation = match progress {
            p if p < 0.25 => "Just getting started...",
            p if p < 0.50 => "Making progress!",
            p if p < 0.75 => "Over halfway there!",
            p if p < 0.90 => "Almost done!",
            _ => "Final stretch!",
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                header,
                Style::default()
                    .fg(if self.state.paused {
                        Color::Yellow
                    } else {
                        Color::Magenta
                    })
                    .add_modifier(Modifier::BOLD),
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
                format!("{:02}{}{:02}", mins, separator, secs),
                Style::default().fg(time_color).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(bar, Style::default().fg(time_color))),
            Line::from(""),
            Line::from(Span::styled(
                motivation,
                Style::default().fg(Color::DarkGray).italic(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Space: pause  q/Esc: cancel  Enter: complete",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let border_color = if self.state.paused {
            Color::Yellow
        } else {
            time_color
        };

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Focus ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .alignment(Alignment::Center);

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }
}
