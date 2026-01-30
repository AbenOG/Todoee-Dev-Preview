use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
};
use todoee_core::Priority;

use crate::tui::app::{AddField, AddState};

pub struct TodoAddWidget<'a> {
    state: &'a AddState,
}

impl<'a> TodoAddWidget<'a> {
    pub fn new(state: &'a AddState) -> Self {
        Self { state }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Add New Task (Tab: next, Shift+Tab: prev, Enter: save, Esc: cancel) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Description
                Constraint::Length(3), // Priority
                Constraint::Length(3), // Due date
                Constraint::Length(3), // Reminder
                Constraint::Length(3), // Category
            ])
            .split(inner);

        // Title field (required)
        self.render_field(
            frame,
            chunks[0],
            "Title *",
            &self.state.title,
            self.state.active_field == AddField::Title,
            true,
        );

        // Description field
        self.render_field(
            frame,
            chunks[1],
            "Description",
            &self.state.description,
            self.state.active_field == AddField::Description,
            false,
        );

        // Priority field
        let priority_text = match self.state.priority {
            Priority::High => "!!! High (1/2/3 to change)",
            Priority::Medium => "!!  Medium (1/2/3 to change)",
            Priority::Low => "!   Low (1/2/3 to change)",
        };
        self.render_field(
            frame,
            chunks[2],
            "Priority",
            priority_text,
            self.state.active_field == AddField::Priority,
            false,
        );

        // Due date field
        let due_text = self
            .state
            .due_date
            .as_deref()
            .unwrap_or("(YYYY-MM-DD or 'today', 'tomorrow', '+3d')");
        self.render_field(
            frame,
            chunks[3],
            "Due Date",
            due_text,
            self.state.active_field == AddField::DueDate,
            false,
        );

        // Reminder field
        let reminder_text = self
            .state
            .reminder
            .as_deref()
            .unwrap_or("(YYYY-MM-DD HH:MM)");
        self.render_field(
            frame,
            chunks[4],
            "Reminder",
            reminder_text,
            self.state.active_field == AddField::Reminder,
            false,
        );

        // Category field
        let cat_text = self
            .state
            .category_name
            .as_deref()
            .unwrap_or("(press any key to cycle, backspace to clear)");
        self.render_field(
            frame,
            chunks[5],
            "Category",
            cat_text,
            self.state.active_field == AddField::Category,
            false,
        );
    }

    fn render_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        active: bool,
        _required: bool,
    ) {
        let border_color = if active { Color::Cyan } else { Color::DarkGray };

        let label_style = if active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let text_style = if active {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let cursor = if active { "|" } else { "" };

        let content = Paragraph::new(format!("{}{}", value, cursor))
            .block(
                Block::default()
                    .title(Span::styled(format!(" {} ", label), label_style))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .style(text_style);

        frame.render_widget(content, area);
    }
}
