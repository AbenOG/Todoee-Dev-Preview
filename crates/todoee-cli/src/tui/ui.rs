use chrono::Utc;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use todoee_core::Priority;

use super::app::{App, Mode, View};
use super::widgets::{
    CategoryListWidget, FocusWidget, InsightsWidget, SettingsWidget, TodoAddWidget,
    TodoDetailWidget, TodoEditorWidget,
};

/// Main UI rendering function
pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Length(3), // Header/Input
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Status bar
            Constraint::Length(1), // Help line
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
    if app.mode == Mode::AddingFull
        && let Some(ref state) = app.add_state
    {
        let area = centered_rect(65, 60, frame.area());
        TodoAddWidget::new(state).render(frame, area);
    }
    if app.mode == Mode::Insights
        && let Some(ref data) = app.insights_data
    {
        let area = centered_rect(50, 55, frame.area());
        InsightsWidget::new(data).render(frame, area);
    }
    if app.mode == Mode::Focus
        && let Some(ref state) = app.focus_state
    {
        let area = centered_rect(50, 40, frame.area());
        FocusWidget::new(state).render(frame, area);
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
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            vec![Span::styled(format!(" {} ", label), style), Span::raw("  ")]
        })
        .collect();

    // Add filter indicators for Todos view
    if app.current_view == View::Todos {
        if let Some(priority) = app.filter.priority {
            let (text, color) = match priority {
                Priority::High => ("HIGH", Color::Red),
                Priority::Medium => ("MEDIUM", Color::Yellow),
                Priority::Low => ("LOW", Color::Green),
            };
            spans.push(Span::styled(
                format!(" [{}] ", text),
                Style::default().fg(color),
            ));
        }

        if app.filter.overdue_only {
            spans.push(Span::styled(
                " [OVERDUE] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }
    }

    let tabs_line = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(input, area);
    } else {
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " Categories ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  a", Style::default().fg(Color::Yellow)),
            Span::raw(":add  "),
            Span::styled("x", Style::default().fg(Color::Yellow)),
            Span::raw(":delete"),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(header, area);
    }
}

fn render_categories(app: &App, frame: &mut Frame, area: Rect) {
    CategoryListWidget::new(&app.categories, app.category_selected).render(frame, area);
}

fn render_settings_header(_app: &App, frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(" Settings ")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
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

    let mut spans = vec![Span::styled(prompt, style), Span::raw(input_text)];

    if matches!(app.mode, Mode::Adding | Mode::Searching | Mode::Editing) {
        spans.push(Span::styled(
            "│",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::SLOW_BLINK),
        ));
    }

    spans.push(priority_indicator);

    let input = Paragraph::new(Line::from(spans)).block(
        Block::default().borders(Borders::ALL).border_style(
            if matches!(app.mode, Mode::Adding | Mode::Searching | Mode::Editing) {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
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
                        Style::default().fg(Color::Red).bold(),
                    ),
                    0 => Span::styled(" [TODAY]", Style::default().fg(Color::Yellow).bold()),
                    1 => Span::styled(" [Tomorrow]", Style::default().fg(Color::Cyan)),
                    d if d <= 7 => {
                        Span::styled(format!(" [{}d]", d), Style::default().fg(Color::Blue))
                    }
                    _ => Span::styled(
                        format!(" [{}]", due.format("%m/%d")),
                        Style::default().fg(Color::DarkGray),
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

            // Animated cursor: alternates between filled and outline arrow
            let selector = if is_selected {
                let cursors = ['▸', '▹', '▸', '▹'];
                format!("{} ", cursors[app.animation_frame % cursors.len()])
            } else {
                "  ".to_string()
            };

            let content = Line::from(vec![
                Span::styled(
                    selector,
                    if is_selected {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    status,
                    if todo.is_completed {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(" "),
                priority,
                Span::raw(" "),
                Span::styled(
                    &todo.title,
                    if todo.is_completed {
                        Style::default().add_modifier(Modifier::CROSSED_OUT)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    format!("  {}", short_id),
                    Style::default().fg(Color::DarkGray),
                ),
                due_str,
            ]);

            ListItem::new(content).style(line_style)
        })
        .collect();

    let tasks = List::new(items).block(
        Block::default()
            .title(format!(" Tasks ({}) ", app.todos.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(tasks, area);
}

fn render_status(app: &App, frame: &mut Frame, area: Rect) {
    let status_text = app.status_message.as_deref().unwrap_or("");

    // Calculate age of status message in frames
    let age = app
        .status_set_frame
        .map(|set_frame| app.animation_frame.wrapping_sub(set_frame))
        .unwrap_or(0);

    // Icon animation: pulse for first few frames
    let icon = if status_text.starts_with('✓') {
        if age < 4 {
            ['✓', '✔', '✓', '✔'][age % 4]
        } else {
            '✓'
        }
    } else if status_text.starts_with('✗') {
        if age < 4 {
            ['✗', '✘', '✗', '✘'][age % 4]
        } else {
            '✗'
        }
    } else {
        ' '
    };

    // Replace first char with animated icon if it's a status icon
    let display_text = if !status_text.is_empty()
        && (status_text.starts_with('✓') || status_text.starts_with('✗'))
    {
        format!(
            "{}{}",
            icon,
            &status_text[status_text.chars().next().unwrap().len_utf8()..]
        )
    } else {
        status_text.to_string()
    };

    let status_style = if status_text.starts_with('✓') {
        Style::default().fg(Color::Green)
    } else if status_text.starts_with('✗') {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let status = Paragraph::new(Span::styled(&display_text, status_style)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(status, area);
}

fn render_help(app: &App, frame: &mut Frame, area: Rect) {
    let help_text = match app.mode {
        Mode::Adding => "Enter:submit  Shift+Enter:with-AI  Tab:priority  Esc:cancel",
        Mode::Editing => "Enter:submit  Esc:cancel",
        Mode::EditingFull => "Tab:next  Shift+Tab:prev  Enter:save  Esc:cancel",
        Mode::AddingFull => "Tab:next  Shift+Tab:prev  Enter:save  Esc:cancel",
        Mode::Searching => "Enter:apply  Esc:cancel  Ctrl+U:clear",
        Mode::Help => "Press any key to close",
        Mode::ViewingDetail => "Esc/q/v/Enter: close detail view",
        Mode::AddingCategory => "Enter:create  Esc:cancel",
        Mode::Insights => "Press any key to close",
        Mode::Focus => "Space:pause  q/Esc:cancel  Enter:complete early",
        Mode::Normal => match app.current_view {
            View::Todos => {
                "j/k:nav a:add d:done x:del u:undo z:stash o:overdue i:insights f:focus n:now ?:help q:quit"
            }
            View::Categories => "j/k:nav  a:add  x:delete  1/2/3:tabs  q:quit",
            View::Settings => "j/k:nav sections  r:reload config  1/2/3:tabs  q:quit",
        },
    };

    let help = Paragraph::new(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(help, area);
}

fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(75, 90, frame.area());

    let help_text = vec![
        Line::from(Span::styled(
            "═══ TODOEE KEYBOARD SHORTCUTS ═══",
            Style::default().bold().fg(Color::Cyan),
        )),
        Line::from(""),
        // ─────────────────────────────────────────────────────────────────
        Line::from(Span::styled(
            "─── NAVIGATION ───",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / ↓       Move down            g           Jump to top"),
        Line::from("  k / ↑       Move up              G           Jump to bottom"),
        Line::from("  1 / 2 / 3   Switch views (Todos/Categories/Settings)"),
        Line::from(""),
        // ─────────────────────────────────────────────────────────────────
        Line::from(Span::styled(
            "─── CORE ACTIONS ───",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from("  a           Add task (full editor with all fields)"),
        Line::from("  A           Quick add (offline, Shift+Enter for AI)"),
        Line::from("  e           Edit selected task"),
        Line::from("  d / Enter   Mark as done"),
        Line::from("  x           Delete task"),
        Line::from("  v / Space   View task details"),
        Line::from(""),
        // ─────────────────────────────────────────────────────────────────
        Line::from(Span::styled(
            "─── GIT-LIKE COMMANDS ───",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from("  u           Undo last action"),
        Line::from("  Ctrl+r      Redo last undone action"),
        Line::from("  z           Stash selected task (hide temporarily)"),
        Line::from("  Z           Pop from stash (restore last stashed)"),
        Line::from(""),
        // ─────────────────────────────────────────────────────────────────
        Line::from(Span::styled(
            "─── FILTERS & SORTING ───",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from("  /           Search (fuzzy matching)"),
        Line::from("  t           Toggle today filter"),
        Line::from("  o           Toggle overdue filter"),
        Line::from("  p           Cycle priority filter (All→High→Med→Low)"),
        Line::from("  c           Cycle category filter"),
        Line::from("  s           Cycle sort (Created→Due→Priority→Title)"),
        Line::from("  S           Toggle sort order (Asc/Desc)"),
        Line::from("  Tab         Toggle show/hide completed"),
        Line::from(""),
        // ─────────────────────────────────────────────────────────────────
        Line::from(Span::styled(
            "─── PRODUCTIVITY ───",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from("  n           Jump to recommended task (smart pick)"),
        Line::from("  f           Start focus session (25 min pomodoro)"),
        Line::from("  F           Quick focus (5 min)"),
        Line::from("  i           View productivity insights"),
        Line::from(""),
        // ─────────────────────────────────────────────────────────────────
        Line::from(Span::styled(
            "─── FOCUS MODE ───",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
        Line::from("  Space       Pause / Resume timer"),
        Line::from("  Enter       Complete early"),
        Line::from("  q / Esc     Cancel focus session"),
        Line::from(""),
        // ─────────────────────────────────────────────────────────────────
        Line::from(Span::styled(
            "─── EDITOR MODE ───",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
        Line::from("  Tab         Next field           Shift+Tab   Previous field"),
        Line::from("  1 / 2 / 3   Set priority (on priority field)"),
        Line::from("  Enter       Save changes         Esc         Cancel"),
        Line::from(""),
        // ─────────────────────────────────────────────────────────────────
        Line::from(Span::styled(
            "─── QUICK ADD (A) ───",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
        Line::from("  Enter       Submit (offline)"),
        Line::from("  Shift+Enter Submit with AI parsing"),
        Line::from("  Tab         Cycle priority       Ctrl+1/2/3  Set priority"),
        Line::from(""),
        // ─────────────────────────────────────────────────────────────────
        Line::from(Span::styled(
            "─── GENERAL ───",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?           Toggle this help     q           Quit"),
        Line::from("  Esc         Close modal / Cancel"),
        Line::from(""),
        Line::from(Span::styled(
            "Tip: Use CLI for batch ops: todoee batch done id1 id2 id3",
            Style::default().fg(Color::DarkGray).italic(),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help (press any key to close) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}

fn render_loading_overlay(app: &App, frame: &mut Frame) {
    use super::spinner::bracketed_progress;

    let area = centered_rect(50, 25, frame.area());

    let spinner_char = app.spinner_style.frame(app.animation_frame);
    let message = app.loading_message.as_deref().unwrap_or("Loading...");

    // Animated dots
    let dots_count = app.animation_frame % 4;
    let dots = ".".repeat(dots_count);
    let dots_padding = " ".repeat(3 - dots_count);

    let mut content = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}  {}{}{}", spinner_char, message, dots, dots_padding),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Add progress bar if available
    if let Some(ref progress) = app.loading_progress {
        let bar = bracketed_progress(progress.percentage(), 30);
        let percentage = (progress.percentage() * 100.0) as u8;

        content.push(Line::from(Span::styled(
            format!("  {} {}%", bar, percentage),
            Style::default().fg(Color::Green),
        )));

        // Show step name if available
        if let Some(ref step) = progress.step_name {
            content.push(Line::from(""));
            content.push(Line::from(Span::styled(
                format!("  {}", step),
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Show progress count
        content.push(Line::from(Span::styled(
            format!("  ({}/{})", progress.current, progress.total),
            Style::default().fg(Color::DarkGray),
        )));
    }

    content.push(Line::from(""));

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
