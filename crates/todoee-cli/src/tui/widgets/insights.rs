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
    animation_frame: usize,
    opened_frame: usize,
}

impl<'a> InsightsWidget<'a> {
    pub fn new(data: &'a InsightsData, animation_frame: usize, opened_frame: usize) -> Self {
        Self {
            data,
            animation_frame,
            opened_frame,
        }
    }

    /// Animate a number from 0 to target over animation_duration frames
    fn animated_value(&self, target: usize, animation_duration: usize) -> usize {
        let elapsed = self.animation_frame.wrapping_sub(self.opened_frame);
        if elapsed >= animation_duration {
            target
        } else {
            let progress = elapsed as f64 / animation_duration as f64;
            (target as f64 * progress).round() as usize
        }
    }

    /// Animate a float value
    fn animated_float(&self, target: f64, animation_duration: usize) -> f64 {
        let elapsed = self.animation_frame.wrapping_sub(self.opened_frame);
        if elapsed >= animation_duration {
            target
        } else {
            let progress = elapsed as f64 / animation_duration as f64;
            target * progress
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let anim_duration = 8; // Animate over 8 frames (2 seconds at 250ms tick)

        // Animated values
        let completed = self.animated_value(self.data.total_completed_7d, anim_duration);
        let created = self.animated_value(self.data.total_created_7d, anim_duration);
        let rate = self.animated_float(self.data.completion_rate, anim_duration);
        let overdue = self.animated_value(self.data.overdue_count, anim_duration);
        let high = self.animated_value(self.data.high_priority_pending, anim_duration);
        let medium = self.animated_value(self.data.medium_priority_pending, anim_duration);
        let low = self.animated_value(self.data.low_priority_pending, anim_duration);

        let rate_color = if rate >= 70.0 {
            Color::Green
        } else if rate >= 40.0 {
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
                    completed.to_string(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Created:   "),
                Span::styled(created.to_string(), Style::default().fg(Color::Blue)),
            ]),
            Line::from(vec![
                Span::raw("  Rate:      "),
                Span::styled(format!("{:.1}%", rate), Style::default().fg(rate_color)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Overdue:   "),
                Span::styled(
                    overdue.to_string(),
                    if overdue > 0 {
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
                Span::raw(format!(" High:   {}", high)),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled("!! ", Style::default().fg(Color::Yellow)),
                Span::raw(format!(" Medium: {}", medium)),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled("!  ", Style::default().fg(Color::Green)),
                Span::raw(format!(" Low:    {}", low)),
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
