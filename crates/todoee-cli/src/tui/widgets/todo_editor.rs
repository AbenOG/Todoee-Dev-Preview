use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use todoee_core::Priority;

use crate::tui::app::{EditField, EditState};

pub struct TodoEditorWidget<'a> {
    state: &'a EditState,
}

impl<'a> TodoEditorWidget<'a> {
    pub fn new(state: &'a EditState) -> Self {
        Self { state }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Edit Todo (Tab: next, Shift+Tab: prev, Enter: save, Esc: cancel) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

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
                Constraint::Length(3), // Category
            ])
            .split(inner);

        // Title field
        self.render_field(
            frame,
            chunks[0],
            "Title",
            &self.state.title,
            self.state.active_field == EditField::Title,
        );

        // Description field
        self.render_field(
            frame,
            chunks[1],
            "Description",
            &self.state.description,
            self.state.active_field == EditField::Description,
        );

        // Priority field
        let priority_text = match self.state.priority {
            Priority::High => "High (press 1/2/3 to change)",
            Priority::Medium => "Medium (press 1/2/3 to change)",
            Priority::Low => "Low (press 1/2/3 to change)",
        };
        self.render_field(
            frame,
            chunks[2],
            "Priority",
            priority_text,
            self.state.active_field == EditField::Priority,
        );

        // Due date field
        let due_text = self
            .state
            .due_date
            .as_deref()
            .unwrap_or("(none - type YYYY-MM-DD)");
        self.render_field(
            frame,
            chunks[3],
            "Due Date",
            due_text,
            self.state.active_field == EditField::DueDate,
        );

        // Category field
        let cat_text = self
            .state
            .category_name
            .as_deref()
            .unwrap_or("(none - press any key to cycle)");
        self.render_field(
            frame,
            chunks[4],
            "Category",
            cat_text,
            self.state.active_field == EditField::Category,
        );
    }

    fn render_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        active: bool,
    ) {
        let border_style = if active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
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
                    .title(format!(" {} ", label))
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .style(text_style);

        frame.render_widget(content, area);
    }
}
