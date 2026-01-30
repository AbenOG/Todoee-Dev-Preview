use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use chrono::Utc;
use todoee_core::Priority;

use super::app::{App, Mode, View};
use super::widgets::{CategoryListWidget, SettingsWidget, TodoDetailWidget, TodoEditorWidget};

/// Main UI rendering function
pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tab bar
            Constraint::Length(3),  // Header/Input
            Constraint::Min(10),    // Content
            Constraint::Length(3),  // Status bar
            Constraint::Length(1),  // Help line
        ])
        .split(frame.area());

    render_tabs(app, frame, chunks[0]);

    match app.current_view {
        View::Todos => {
            render_input(app, frame, chunks[1]);
            render_tasks(app, frame, chunks[2]);
        }
        View::Categories => {
            render_category_header(app, frame, chunks[1]);
            render_categories(app, frame, chunks[2]);
        }
        View::Settings => {
            render_settings_header(app, frame, chunks[1]);
            render_settings_content(app, frame, chunks[2]);
        }
    }

    render_status(app, frame, chunks[3]);
    render_help(app, frame, chunks[4]);

    // Modals
    if app.mode == Mode::Help {
        render_help_modal(frame);
    }
    if app.mode == Mode::ViewingDetail
        && let Some(todo) = app.selected_todo()
    {
        let area = centered_rect(70, 80, frame.area());
        TodoDetailWidget::new(todo).render(frame, area);
    }
    if app.mode == Mode::EditingFull
        && let Some(ref state) = app.edit_state
    {
        let area = centered_rect(60, 50, frame.area());
        TodoEditorWidget::new(state).render(frame, area);
    }

    // Loading overlay (always on top)
    if app.is_loading {
        render_loading_overlay(app, frame);
    }
}

fn render_tabs(app: &App, frame: &mut Frame, area: Rect) {
    let tabs = [
        ("1: Todos", View::Todos),
        ("2: Categories", View::Categories),
        ("3: Settings", View::Settings),
    ];

    let mut spans: Vec<Span> = tabs
        .iter()
        .flat_map(|(label, view)| {
            let style = if app.current_view == *view {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            vec![Span::styled(format!(" {} ", label), style), Span::raw("  ")]
        })
        .collect();

    // Add filter indicators for Todos view
    if app.current_view == View::Todos
        && let Some(priority) = app.filter.priority
    {
        let (text, color) = match priority {
            Priority::High => ("HIGH", Color::Red),
            Priority::Medium => ("MEDIUM", Color::Yellow),
            Priority::Low => ("LOW", Color::Green),
        };
        spans.push(Span::styled(format!(" [{}] ", text), Style::default().fg(color)));
    }

    let tabs_line = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray))
        );

    frame.render_widget(tabs_line, area);
}

fn render_category_header(app: &App, frame: &mut Frame, area: Rect) {
    if app.mode == Mode::AddingCategory {
        let input = Paragraph::new(Line::from(vec![
            Span::styled("> New category: ", Style::default().fg(Color::Green)),
            Span::raw(app.input.value()),
            Span::styled("|", Style::default().fg(Color::White)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        frame.render_widget(input, area);
    } else {
        let header = Paragraph::new(Line::from(vec![
            Span::styled(" Categories ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("  a", Style::default().fg(Color::Yellow)),
            Span::raw(":add  "),
            Span::styled("x", Style::default().fg(Color::Yellow)),
            Span::raw(":delete"),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(header, area);
    }
}

fn render_categories(app: &App, frame: &mut Frame, area: Rect) {
    CategoryListWidget::new(&app.categories, app.category_selected).render(frame, area);
}

fn render_settings_header(_app: &App, frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(" Settings ")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(header, area);
}

fn render_settings_content(app: &App, frame: &mut Frame, area: Rect) {
    SettingsWidget::new(&app.config, app.settings_section).render(frame, area);
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

    // Priority indicator for Adding mode
    let priority_indicator = if app.mode == Mode::Adding {
        let (text, color) = match app.pending_priority {
            Some(Priority::High) => (" [!!!]", Color::Red),
            Some(Priority::Medium) => (" [!!]", Color::Yellow),
            Some(Priority::Low) => (" [!]", Color::Green),
            None => (" [--]", Color::DarkGray),
        };
        Span::styled(text, Style::default().fg(color))
    } else {
        Span::raw("")
    };

    let mut spans = vec![
        Span::styled(prompt, style),
        Span::raw(input_text),
    ];

    if matches!(app.mode, Mode::Adding | Mode::Searching | Mode::Editing) {
        spans.push(Span::styled("│", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK)));
    }

    spans.push(priority_indicator);

    let input = Paragraph::new(Line::from(spans))
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
        Mode::Adding => "Enter:submit  Shift+Enter:no-AI  Tab:priority  Esc:cancel",
        Mode::Editing => "Enter:submit  Esc:cancel",
        Mode::EditingFull => "Tab:next  Shift+Tab:prev  Enter:save  Esc:cancel",
        Mode::Searching => "Enter:apply  Esc:cancel  Ctrl+U:clear",
        Mode::Help => "Press any key to close",
        Mode::ViewingDetail => "Esc/q/v/Enter: close detail view",
        Mode::AddingCategory => "Enter:create  Esc:cancel",
        Mode::Normal => match app.current_view {
            View::Todos => "j/k:nav  a:add  d:done  x:del  e:edit  v:view  p:priority  /:search  1/2/3:tabs  q:quit",
            View::Categories => "j/k:nav  a:add  x:delete  1/2/3:tabs  q:quit",
            View::Settings => "j/k:nav sections  r:reload config  1/2/3:tabs  q:quit",
        },
    };

    let help = Paragraph::new(Span::styled(help_text, Style::default().fg(Color::DarkGray)));
    frame.render_widget(help, area);
}

fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(70, 85, frame.area());

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().bold().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(Span::styled("Views", Style::default().fg(Color::Yellow))),
        Line::from("  1           Todos view"),
        Line::from("  2           Categories view"),
        Line::from("  3           Settings view"),
        Line::from("  ?           Toggle help"),
        Line::from(""),
        Line::from(Span::styled(
            "Todos View",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  j / ↓       Move down"),
        Line::from("  k / ↑       Move up"),
        Line::from("  g           Jump to top"),
        Line::from("  G           Jump to bottom"),
        Line::from("  a           Add todo"),
        Line::from("  e           Edit todo (full editor)"),
        Line::from("  v / Space   View details"),
        Line::from("  Enter       Toggle complete"),
        Line::from("  d           Mark as done"),
        Line::from("  x           Delete todo"),
        Line::from("  /           Search"),
        Line::from("  p           Cycle priority filter"),
        Line::from("  s           Cycle sort field"),
        Line::from("  S           Toggle sort order"),
        Line::from("  t           Toggle today filter"),
        Line::from("  Tab         Toggle show completed"),
        Line::from("  c           Cycle category filter"),
        Line::from("  Esc         Close modal/cancel"),
        Line::from("  q           Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Categories View",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  j / ↓       Move down"),
        Line::from("  k / ↑       Move up"),
        Line::from("  a           Add category"),
        Line::from("  x           Delete category"),
        Line::from("  Esc         Cancel"),
        Line::from("  q           Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Settings View",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  j / ↓       Next section"),
        Line::from("  k / ↑       Previous section"),
        Line::from("  r           Reload config"),
        Line::from("  q           Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Editor (when editing todo)",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  Tab         Next field"),
        Line::from("  Shift+Tab   Previous field"),
        Line::from("  Enter       Save"),
        Line::from("  Esc         Cancel"),
        Line::from(""),
        Line::from(Span::styled(
            "Adding Task",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  Enter       Submit (with AI if enabled)"),
        Line::from("  Shift+Enter Submit without AI"),
        Line::from("  Tab         Cycle priority (None→Low→Med→High)"),
        Line::from("  Ctrl+1/2/3  Set priority (Low/Med/High)"),
        Line::from("  Esc         Cancel"),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}

fn render_loading_overlay(app: &App, frame: &mut Frame) {
    let area = centered_rect(40, 15, frame.area());

    // Animated spinner characters
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        / 100) as usize
        % spinner_chars.len();
    let spinner = spinner_chars[idx];

    let message = app.loading_message.as_deref().unwrap_or("Loading...");

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}  {}", spinner, message),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let loading = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Processing "),
        )
        .alignment(Alignment::Center);

    frame.render_widget(Clear, area);
    frame.render_widget(loading, area);
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
