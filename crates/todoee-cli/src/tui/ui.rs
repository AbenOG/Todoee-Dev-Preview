use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use chrono::Utc;
use todoee_core::Priority;

use super::app::{App, Mode};

/// Main UI rendering function
pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // Input
            Constraint::Min(10),    // Task list
            Constraint::Length(3),  // Status bar
            Constraint::Length(1),  // Help line
        ])
        .split(frame.area());

    render_header(app, frame, chunks[0]);
    render_input(app, frame, chunks[1]);
    render_tasks(app, frame, chunks[2]);
    render_status(app, frame, chunks[3]);
    render_help(app, frame, chunks[4]);

    // Render modal overlays
    if app.mode == Mode::Help {
        render_help_modal(frame);
    }
}

fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let title = format!(
        " todoee {} ",
        if app.filter.today_only { "[TODAY]" } else { "" }
    );

    let filter_info = if !app.filter.search_query.is_empty() {
        format!(" 🔍 \"{}\" ", app.filter.search_query)
    } else if let Some(ref cat) = app.filter.category {
        format!(" 📁 {} ", cat)
    } else {
        String::new()
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(title, Style::default().fg(Color::Cyan).bold()),
        Span::raw(filter_info),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
    );

    frame.render_widget(header, area);
}

fn render_input(app: &App, frame: &mut Frame, area: Rect) {
    let (prompt, style) = match app.mode {
        Mode::Adding => ("> Add task: ", Style::default().fg(Color::Green)),
        Mode::Searching => ("> Search: ", Style::default().fg(Color::Yellow)),
        Mode::Editing => ("> Edit: ", Style::default().fg(Color::Blue)),
        _ => ("> ", Style::default().fg(Color::DarkGray)),
    };

    let input_text = if matches!(app.mode, Mode::Adding | Mode::Searching | Mode::Editing) {
        app.input.value()
    } else {
        "Press 'a' to add task, '/' to search"
    };

    let input = Paragraph::new(Line::from(vec![
        Span::styled(prompt, style),
        Span::raw(input_text),
        if matches!(app.mode, Mode::Adding | Mode::Searching | Mode::Editing) {
            Span::styled("│", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK))
        } else {
            Span::raw("")
        },
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(if matches!(app.mode, Mode::Adding | Mode::Searching | Mode::Editing) {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            })
    );

    frame.render_widget(input, area);
}

fn render_tasks(app: &App, frame: &mut Frame, area: Rect) {
    let now = Utc::now();

    let items: Vec<ListItem> = app
        .todos
        .iter()
        .enumerate()
        .map(|(i, todo)| {
            let is_selected = i == app.selected;

            // Status indicator
            let status = if todo.is_completed { "[x]" } else { "[ ]" };

            // Priority indicator
            let priority = match todo.priority {
                Priority::High => Span::styled("!!!", Style::default().fg(Color::Red).bold()),
                Priority::Medium => Span::styled("!! ", Style::default().fg(Color::Yellow)),
                Priority::Low => Span::styled("!  ", Style::default().fg(Color::Green)),
            };

            // Short ID
            let short_id = &todo.id.to_string()[..8];

            // Due date
            let due_str = if let Some(due) = todo.due_date {
                let days_until = (due.date_naive() - now.date_naive()).num_days();
                match days_until {
                    d if d < 0 => Span::styled(
                        format!(" [OVERDUE {}d]", -d),
                        Style::default().fg(Color::Red).bold()
                    ),
                    0 => Span::styled(" [TODAY]", Style::default().fg(Color::Yellow).bold()),
                    1 => Span::styled(" [Tomorrow]", Style::default().fg(Color::Cyan)),
                    d if d <= 7 => Span::styled(
                        format!(" [{}d]", d),
                        Style::default().fg(Color::Blue)
                    ),
                    _ => Span::styled(
                        format!(" [{}]", due.format("%m/%d")),
                        Style::default().fg(Color::DarkGray)
                    ),
                }
            } else {
                Span::raw("")
            };

            // Build the line
            let line_style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else if todo.is_completed {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            let selector = if is_selected { "▸ " } else { "  " };

            let content = Line::from(vec![
                Span::styled(selector, Style::default().fg(Color::Cyan)),
                Span::styled(status, if todo.is_completed {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                }),
                Span::raw(" "),
                priority,
                Span::raw(" "),
                Span::styled(
                    &todo.title,
                    if todo.is_completed {
                        Style::default().add_modifier(Modifier::CROSSED_OUT)
                    } else {
                        Style::default()
                    }
                ),
                Span::styled(format!("  {}", short_id), Style::default().fg(Color::DarkGray)),
                due_str,
            ]);

            ListItem::new(content).style(line_style)
        })
        .collect();

    let tasks = List::new(items)
        .block(
            Block::default()
                .title(format!(" Tasks ({}) ", app.todos.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
        );

    frame.render_widget(tasks, area);
}

fn render_status(app: &App, frame: &mut Frame, area: Rect) {
    let status_text = app.status_message.as_deref().unwrap_or("");
    let status_style = if status_text.starts_with('✓') {
        Style::default().fg(Color::Green)
    } else if status_text.starts_with('✗') {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let status = Paragraph::new(Span::styled(status_text, status_style))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
        );

    frame.render_widget(status, area);
}

fn render_help(app: &App, frame: &mut Frame, area: Rect) {
    let help_text = match app.mode {
        Mode::Adding | Mode::Editing => "Enter:submit  Esc:cancel",
        Mode::Searching => "Enter:apply  Esc:cancel  Ctrl+U:clear",
        Mode::Help => "Press any key to close",
        Mode::Normal => "j/k:nav  a:add  d:done  x:del  e:edit  /:search  t:today  ?:help  q:quit",
    };

    let help = Paragraph::new(Span::styled(help_text, Style::default().fg(Color::DarkGray)));
    frame.render_widget(help, area);
}

fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());

    let help_text = vec![
        Line::from(Span::styled("Keyboard Shortcuts", Style::default().bold().fg(Color::Cyan))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("  j / ↓       Move down"),
        Line::from("  k / ↑       Move up"),
        Line::from("  g           Go to top"),
        Line::from("  G           Go to bottom"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Actions", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("  a           Add new task"),
        Line::from("  d / Enter   Mark as done"),
        Line::from("  x           Delete task"),
        Line::from("  e           Edit task"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Filtering", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("  /           Search tasks"),
        Line::from("  t           Toggle today filter"),
        Line::from("  c           Cycle categories"),
        Line::from("  Tab         Show all / incomplete"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Other", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("  ?           Show this help"),
        Line::from("  q / Esc     Quit"),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
