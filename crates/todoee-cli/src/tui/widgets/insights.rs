use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::tui::app::InsightsData;

pub struct InsightsWidget<'a> {
    data: &'a InsightsData,
}

impl<'a> InsightsWidget<'a> {
    pub fn new(data: &'a InsightsData) -> Self {
        Self { data }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let rate_color = if self.data.completion_rate >= 70.0 {
            Color::Green
        } else if self.data.completion_rate >= 40.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        let lines = vec![
            Line::from(Span::styled(
                " Productivity Insights (7 days)",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Completed: "),
                Span::styled(
                    self.data.total_completed_7d.to_string(),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Created:   "),
                Span::styled(
                    self.data.total_created_7d.to_string(),
                    Style::default().fg(Color::Blue),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Rate:      "),
                Span::styled(
                    format!("{:.1}%", self.data.completion_rate),
                    Style::default().fg(rate_color),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Overdue:   "),
                Span::styled(
                    self.data.overdue_count.to_string(),
                    if self.data.overdue_count > 0 {
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Pending by Priority:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(vec![
                Span::raw("    "),
                Span::styled("!!!", Style::default().fg(Color::Red)),
                Span::raw(format!(" High:   {}", self.data.high_priority_pending)),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled("!! ", Style::default().fg(Color::Yellow)),
                Span::raw(format!(" Medium: {}", self.data.medium_priority_pending)),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled("!  ", Style::default().fg(Color::Green)),
                Span::raw(format!(" Low:    {}", self.data.low_priority_pending)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Press any key to close",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Insights ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }
}
