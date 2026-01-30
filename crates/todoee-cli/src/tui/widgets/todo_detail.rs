use chrono::Utc;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use todoee_core::{Priority, Todo};

pub struct TodoDetailWidget<'a> {
    todo: &'a Todo,
}

impl<'a> TodoDetailWidget<'a> {
    pub fn new(todo: &'a Todo) -> Self {
        Self { todo }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Clear background
        frame.render_widget(Clear, area);

        let priority_color = match self.todo.priority {
            Priority::High => Color::Red,
            Priority::Medium => Color::Yellow,
            Priority::Low => Color::Green,
        };

        let priority_text = match self.todo.priority {
            Priority::High => "High",
            Priority::Medium => "Medium",
            Priority::Low => "Low",
        };

        let status = if self.todo.is_completed {
            Span::styled("Completed", Style::default().fg(Color::Green))
        } else {
            Span::styled("Pending", Style::default().fg(Color::Yellow))
        };

        let now = Utc::now();
        let due_text = if let Some(due) = self.todo.due_date {
            let days = (due.date_naive() - now.date_naive()).num_days();
            match days {
                d if d < 0 => format!("OVERDUE by {} days", -d),
                0 => "Due TODAY".to_string(),
                1 => "Due tomorrow".to_string(),
                d => format!("Due in {} days ({})", d, due.format("%Y-%m-%d")),
            }
        } else {
            "No due date".to_string()
        };

        let reminder_text = self
            .todo
            .reminder_at
            .map(|r| format!("Reminder: {}", r.format("%Y-%m-%d %H:%M")))
            .unwrap_or_else(|| "No reminder set".to_string());

        let category_text = self
            .todo
            .category_id
            .map(|id| format!("Category ID: {}", &id.to_string()[..8]))
            .unwrap_or_else(|| "No category".to_string());

        let created = format!("Created: {}", self.todo.created_at.format("%Y-%m-%d %H:%M"));
        let updated = format!("Updated: {}", self.todo.updated_at.format("%Y-%m-%d %H:%M"));

        let content = vec![
            Line::from(vec![
                Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&self.todo.title),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                status,
            ]),
            Line::from(vec![
                Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(priority_text, Style::default().fg(priority_color)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Description: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(self.todo.description.as_deref().unwrap_or("(none)")),
            Line::from(""),
            Line::from(vec![Span::styled(
                &due_text,
                Style::default().fg(if self.todo.due_date.is_some() {
                    Color::Cyan
                } else {
                    Color::DarkGray
                }),
            )]),
            Line::from(vec![Span::styled(
                &reminder_text,
                Style::default().fg(Color::DarkGray),
            )]),
            Line::from(vec![Span::styled(
                &category_text,
                Style::default().fg(Color::DarkGray),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                &created,
                Style::default().fg(Color::DarkGray),
            )]),
            Line::from(vec![Span::styled(
                &updated,
                Style::default().fg(Color::DarkGray),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.todo.id.to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" Todo Details ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}
