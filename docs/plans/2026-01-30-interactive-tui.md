# Interactive TUI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform todoee into an interactive TUI application with Claude Code-inspired aesthetics, supporting both interactive mode and quick terminal shortcuts.

**Architecture:** Dual-mode CLI - runs interactive TUI when invoked without arguments, supports quick commands with flags. Uses ratatui for rendering, crossterm for terminal events. Component-based architecture with App state, event handling loop, and immediate-mode rendering. Vim-style keybindings for power users.

**Tech Stack:** ratatui 0.29, crossterm 0.28, tui-input (text input), tokio (async), existing todoee-core library

---

## Design Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│  todoee                                              v0.1.0   ?help │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  > Add task...                                    [Enter to submit] │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│  TODAY (3)                                                          │
│  ─────────                                                          │
│  [ ] !!! Review PR #123                           8a3b2c1d  [TODAY] │
│  [>] !!  Buy groceries                            f2e1d0c9  [TODAY] │
│  [ ] !   Send email to client                     a1b2c3d4  [TODAY] │
│                                                                     │
│  WORK (2)                                                           │
│  ─────────                                                          │
│  [ ] !!  Prepare presentation                     e5f6a7b8  [3 days]│
│  [ ] !   Update documentation                     c9d0e1f2  [1 week]│
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│  j/k:navigate  a:add  d:done  x:delete  e:edit  /:search  q:quit   │
└─────────────────────────────────────────────────────────────────────┘
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `a` | Quick add (focus input) |
| `d` / `Enter` | Mark selected as done |
| `x` | Delete selected |
| `e` | Edit selected |
| `/` | Search/filter |
| `t` | Toggle show today only |
| `c` | Filter by category |
| `1-3` | Set priority filter |
| `Tab` | Cycle through categories |
| `?` | Show help |
| `q` / `Esc` | Quit |

---

## Phase 1: TUI Foundation

### Task 1: Create TUI Module Structure

**Files:**
- Create: `crates/todoee-cli/src/tui/mod.rs`
- Create: `crates/todoee-cli/src/tui/app.rs`
- Create: `crates/todoee-cli/src/tui/event.rs`
- Create: `crates/todoee-cli/src/tui/ui.rs`
- Modify: `crates/todoee-cli/src/main.rs`
- Modify: `crates/todoee-cli/Cargo.toml`

**Step 1: Add tui-input dependency to Cargo.toml**

Add to `crates/todoee-cli/Cargo.toml`:
```toml
tui-input = "0.11"
```

**Step 2: Create tui/mod.rs**

Create `crates/todoee-cli/src/tui/mod.rs`:
```rust
pub mod app;
pub mod event;
pub mod ui;

pub use app::App;
pub use event::{Event, EventHandler};
```

**Step 3: Create tui/event.rs with event handling**

Create `crates/todoee-cli/src/tui/event.rs`:
```rust
use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Terminal events
#[derive(Clone, Debug)]
pub enum Event {
    /// Terminal tick (for animations/updates)
    Tick,
    /// Key press
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
}

/// Handles terminal events
pub struct EventHandler {
    /// Event sender
    sender: mpsc::Sender<Event>,
    /// Event receiver
    receiver: mpsc::Receiver<Event>,
    /// Event handler thread
    _handler: thread::JoinHandle<()>,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (sender, receiver) = mpsc::channel();
        let handler_sender = sender.clone();

        let handler = thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(tick_rate);

                if event::poll(timeout).expect("failed to poll events") {
                    match event::read().expect("failed to read event") {
                        CrosstermEvent::Key(e) => {
                            if handler_sender.send(Event::Key(e)).is_err() {
                                break;
                            }
                        }
                        CrosstermEvent::Mouse(e) => {
                            if handler_sender.send(Event::Mouse(e)).is_err() {
                                break;
                            }
                        }
                        CrosstermEvent::Resize(w, h) => {
                            if handler_sender.send(Event::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if handler_sender.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self {
            sender,
            receiver,
            _handler: handler,
        }
    }

    /// Receive the next event
    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv()?)
    }
}
```

**Step 4: Create tui/app.rs with application state**

Create `crates/todoee-cli/src/tui/app.rs`:
```rust
use anyhow::Result;
use todoee_core::{Config, LocalDb, Todo, Category, Priority};
use tui_input::Input;

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal navigation mode
    Normal,
    /// Adding a new task
    Adding,
    /// Editing a task
    Editing,
    /// Searching/filtering
    Searching,
    /// Showing help
    Help,
}

/// Filter state for the task list
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub today_only: bool,
    pub category: Option<String>,
    pub show_completed: bool,
    pub search_query: String,
}

/// Application state
pub struct App {
    /// Is the app running?
    pub running: bool,
    /// Current mode
    pub mode: Mode,
    /// List of todos
    pub todos: Vec<Todo>,
    /// List of categories
    pub categories: Vec<Category>,
    /// Currently selected index
    pub selected: usize,
    /// Input field for adding/editing/searching
    pub input: Input,
    /// Current filter
    pub filter: Filter,
    /// Status message to display
    pub status_message: Option<String>,
    /// Database connection
    db: LocalDb,
    /// Configuration
    config: Config,
}

impl App {
    /// Create a new application instance
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let db_path = config.local_db_path()?;
        let db = LocalDb::new(&db_path).await?;

        let mut app = Self {
            running: true,
            mode: Mode::Normal,
            todos: Vec::new(),
            categories: Vec::new(),
            selected: 0,
            input: Input::default(),
            filter: Filter::default(),
            status_message: None,
            db,
            config,
        };

        app.refresh_todos().await?;
        app.refresh_categories().await?;

        Ok(app)
    }

    /// Refresh the todo list from database
    pub async fn refresh_todos(&mut self) -> Result<()> {
        self.todos = if self.filter.today_only {
            self.db.list_todos_due_today().await?
        } else if let Some(ref cat_name) = self.filter.category {
            if let Some(cat) = self.db.get_category_by_name(cat_name).await? {
                self.db.list_todos_by_category(cat.id).await?
            } else {
                Vec::new()
            }
        } else {
            self.db.list_todos(!self.filter.show_completed).await?
        };

        // Apply search filter
        if !self.filter.search_query.is_empty() {
            let query = self.filter.search_query.to_lowercase();
            self.todos.retain(|t| t.title.to_lowercase().contains(&query));
        }

        // Ensure selected index is valid
        if self.selected >= self.todos.len() && !self.todos.is_empty() {
            self.selected = self.todos.len() - 1;
        }

        Ok(())
    }

    /// Refresh categories from database
    pub async fn refresh_categories(&mut self) -> Result<()> {
        self.categories = self.db.list_categories().await?;
        Ok(())
    }

    /// Get the currently selected todo
    pub fn selected_todo(&self) -> Option<&Todo> {
        self.todos.get(self.selected)
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected < self.todos.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Mark selected todo as done
    pub async fn mark_selected_done(&mut self) -> Result<()> {
        if let Some(todo) = self.todos.get_mut(self.selected) {
            if !todo.is_completed {
                todo.mark_complete();
                self.db.update_todo(todo).await?;
                self.status_message = Some(format!("✓ Completed: {}", todo.title));
                self.refresh_todos().await?;
            } else {
                self.status_message = Some("Already completed".to_string());
            }
        }
        Ok(())
    }

    /// Delete selected todo
    pub async fn delete_selected(&mut self) -> Result<()> {
        if let Some(todo) = self.todos.get(self.selected) {
            let title = todo.title.clone();
            self.db.delete_todo(todo.id).await?;
            self.status_message = Some(format!("✗ Deleted: {}", title));
            self.refresh_todos().await?;
        }
        Ok(())
    }

    /// Add a new todo from input
    pub async fn add_todo_from_input(&mut self) -> Result<()> {
        let description = self.input.value().trim().to_string();
        if description.is_empty() {
            self.status_message = Some("Cannot add empty task".to_string());
            return Ok(());
        }

        let todo = Todo::new(description.clone(), None);
        self.db.create_todo(&todo).await?;
        self.status_message = Some(format!("✓ Added: {}", description));
        self.input.reset();
        self.refresh_todos().await?;

        Ok(())
    }

    /// Toggle today filter
    pub fn toggle_today_filter(&mut self) {
        self.filter.today_only = !self.filter.today_only;
        self.filter.category = None;
    }

    /// Toggle show completed
    pub fn toggle_show_completed(&mut self) {
        self.filter.show_completed = !self.filter.show_completed;
    }

    /// Set search query from input
    pub fn apply_search(&mut self) {
        self.filter.search_query = self.input.value().to_string();
        self.input.reset();
    }

    /// Clear search
    pub fn clear_search(&mut self) {
        self.filter.search_query.clear();
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.running = false;
    }
}
```

**Step 5: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Compilation errors (ui.rs not yet created)

**Step 6: Commit**

```bash
git add crates/todoee-cli/
git commit -m "feat(tui): add event handling and app state modules"
```

---

### Task 2: Implement UI Rendering

**Files:**
- Create: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Create tui/ui.rs with rendering logic**

Create `crates/todoee-cli/src/tui/ui.rs`:
```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
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
```

**Step 2: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success (or minor fixes needed)

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "feat(tui): implement UI rendering with ratatui"
```

---

### Task 3: Implement Terminal Management and Main Loop

**Files:**
- Create: `crates/todoee-cli/src/tui/terminal.rs`
- Modify: `crates/todoee-cli/src/tui/mod.rs`

**Step 1: Create tui/terminal.rs**

Create `crates/todoee-cli/src/tui/terminal.rs`:
```rust
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

/// Terminal wrapper that handles setup and cleanup
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    /// Create and initialize a new terminal
    pub fn new() -> Result<Self> {
        let terminal = Self::setup_terminal()?;
        Ok(Self { terminal })
    }

    fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(terminal)
    }

    fn restore_terminal() -> Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    /// Get mutable reference to the terminal
    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    /// Draw a frame
    pub fn draw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        if let Err(e) = Self::restore_terminal() {
            eprintln!("Failed to restore terminal: {}", e);
        }
    }
}
```

**Step 2: Update tui/mod.rs to export terminal**

Update `crates/todoee-cli/src/tui/mod.rs`:
```rust
pub mod app;
pub mod event;
pub mod terminal;
pub mod ui;

pub use app::App;
pub use event::{Event, EventHandler};
pub use terminal::Tui;
```

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add terminal management with cleanup on drop"
```

---

### Task 4: Implement Input Handler

**Files:**
- Create: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/mod.rs`

**Step 1: Create tui/handler.rs**

Create `crates/todoee-cli/src/tui/handler.rs`:
```rust
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler as InputHandler;

use super::app::{App, Mode};

/// Handle key events and update app state
pub async fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    // Clear status message on any key press
    app.status_message = None;

    match app.mode {
        Mode::Normal => handle_normal_mode(app, key).await?,
        Mode::Adding => handle_adding_mode(app, key).await?,
        Mode::Editing => handle_editing_mode(app, key).await?,
        Mode::Searching => handle_searching_mode(app, key).await?,
        Mode::Help => handle_help_mode(app, key),
    }

    Ok(())
}

async fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => app.quit(),

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_previous(),
        KeyCode::Char('g') => app.selected = 0,
        KeyCode::Char('G') => {
            if !app.todos.is_empty() {
                app.selected = app.todos.len() - 1;
            }
        }

        // Actions
        KeyCode::Char('a') => {
            app.mode = Mode::Adding;
            app.input.reset();
        }
        KeyCode::Char('d') | KeyCode::Enter => {
            app.mark_selected_done().await?;
        }
        KeyCode::Char('x') => {
            app.delete_selected().await?;
        }
        KeyCode::Char('e') => {
            if let Some(todo) = app.selected_todo() {
                app.mode = Mode::Editing;
                app.input = tui_input::Input::new(todo.title.clone());
            }
        }

        // Filtering
        KeyCode::Char('/') => {
            app.mode = Mode::Searching;
            app.input.reset();
        }
        KeyCode::Char('t') => {
            app.toggle_today_filter();
            app.refresh_todos().await?;
        }
        KeyCode::Tab => {
            app.toggle_show_completed();
            app.refresh_todos().await?;
        }
        KeyCode::Char('c') => {
            // Cycle through categories
            if app.categories.is_empty() {
                app.filter.category = None;
            } else if let Some(ref current) = app.filter.category {
                let idx = app.categories.iter().position(|c| &c.name == current);
                app.filter.category = match idx {
                    Some(i) if i + 1 < app.categories.len() => {
                        Some(app.categories[i + 1].name.clone())
                    }
                    _ => None,
                };
            } else {
                app.filter.category = Some(app.categories[0].name.clone());
            }
            app.filter.today_only = false;
            app.refresh_todos().await?;
        }

        // Help
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }

        _ => {}
    }

    Ok(())
}

async fn handle_adding_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
        }
        KeyCode::Enter => {
            app.add_todo_from_input().await?;
            app.mode = Mode::Normal;
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }

    Ok(())
}

async fn handle_editing_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
        }
        KeyCode::Enter => {
            let new_title = app.input.value().trim().to_string();
            if !new_title.is_empty() {
                if let Some(todo) = app.todos.get_mut(app.selected) {
                    todo.title = new_title.clone();
                    todo.updated_at = chrono::Utc::now();
                    todo.sync_status = todoee_core::SyncStatus::Pending;
                    app.db.update_todo(todo).await?;
                    app.status_message = Some(format!("✓ Updated: {}", new_title));
                }
            }
            app.mode = Mode::Normal;
            app.input.reset();
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }

    Ok(())
}

async fn handle_searching_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
            app.clear_search();
            app.refresh_todos().await?;
        }
        KeyCode::Enter => {
            app.apply_search();
            app.mode = Mode::Normal;
            app.refresh_todos().await?;
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input.reset();
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }

    Ok(())
}

fn handle_help_mode(app: &mut App, _key: KeyEvent) {
    app.mode = Mode::Normal;
}
```

**Step 2: Update tui/mod.rs**

Update `crates/todoee-cli/src/tui/mod.rs`:
```rust
pub mod app;
pub mod event;
pub mod handler;
pub mod terminal;
pub mod ui;

pub use app::App;
pub use event::{Event, EventHandler};
pub use handler::handle_key_event;
pub use terminal::Tui;
```

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): implement keyboard input handler"
```

---

### Task 5: Integrate TUI with Main Entry Point

**Files:**
- Modify: `crates/todoee-cli/src/main.rs`

**Step 1: Update main.rs to support interactive mode**

Replace `crates/todoee-cli/src/main.rs`:
```rust
use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod tui;

/// todoee - AI-powered todo manager
#[derive(Parser)]
#[command(name = "todoee")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run in interactive TUI mode (default when no command given)
    #[arg(short, long, global = true)]
    interactive: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new todo item
    Add {
        /// Description of the todo (can be natural language)
        #[arg(required = true)]
        description: Vec<String>,

        /// Skip AI parsing and use description as-is
        #[arg(long)]
        no_ai: bool,

        /// Category for the todo
        #[arg(short, long)]
        category: Option<String>,

        /// Priority (1=low, 2=medium, 3=high)
        #[arg(short, long)]
        priority: Option<i32>,
    },

    /// List todos
    List {
        /// Show only today's todos
        #[arg(long)]
        today: bool,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Show all todos including completed
        #[arg(short, long)]
        all: bool,
    },

    /// Mark a todo as done
    Done {
        /// Todo ID (UUID or short ID)
        id: String,
    },

    /// Delete a todo
    Delete {
        /// Todo ID (UUID or short ID)
        id: String,
    },

    /// Edit a todo
    Edit {
        /// Todo ID (UUID or short ID)
        id: String,

        /// New title
        #[arg(short, long)]
        title: Option<String>,

        /// New category
        #[arg(short, long)]
        category: Option<String>,

        /// New priority (1=low, 2=medium, 3=high)
        #[arg(short, long)]
        priority: Option<i32>,
    },

    /// Sync todos with the server
    Sync,

    /// Configure todoee
    Config {
        /// Initialize configuration with interactive setup
        #[arg(long)]
        init: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // If no command provided or -i flag, run interactive mode
    if cli.command.is_none() || cli.interactive {
        return run_interactive().await;
    }

    // Handle subcommands
    match cli.command.unwrap() {
        Commands::Add {
            description,
            no_ai,
            category,
            priority,
        } => {
            commands::add(description, no_ai, category, priority).await?;
        }
        Commands::List {
            today,
            category,
            all,
        } => {
            commands::list(today, category, all).await?;
        }
        Commands::Done { id } => {
            commands::done(id).await?;
        }
        Commands::Delete { id } => {
            commands::delete(id).await?;
        }
        Commands::Edit {
            id,
            title,
            category,
            priority,
        } => {
            commands::edit(id, title, category, priority).await?;
        }
        Commands::Sync => {
            commands::sync().await?;
        }
        Commands::Config { init } => {
            commands::config(init).await?;
        }
    }

    Ok(())
}

/// Run the interactive TUI
async fn run_interactive() -> Result<()> {
    // Initialize application state
    let mut app = tui::App::new().await?;

    // Initialize terminal
    let mut terminal = tui::Tui::new()?;

    // Create event handler
    let events = tui::EventHandler::new(250);

    // Main loop
    while app.running {
        // Render UI
        terminal.draw(|frame| tui::ui::render(&app, frame))?;

        // Handle events
        match events.next()? {
            tui::Event::Tick => {
                // Could refresh data periodically here
            }
            tui::Event::Key(key) => {
                tui::handle_key_event(&mut app, key).await?;
            }
            tui::Event::Mouse(_) => {
                // Mouse support could be added here
            }
            tui::Event::Resize(_, _) => {
                // Terminal handles resize automatically
            }
        }
    }

    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build -p todoee-cli`
Expected: Successful compilation

**Step 3: Test interactive mode**

Run: `cargo run -p todoee-cli`
Expected: Interactive TUI opens

**Step 4: Test quick commands still work**

Run: `cargo run -p todoee-cli -- list`
Expected: Shows list in non-interactive mode

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/main.rs
git commit -m "feat(tui): integrate interactive TUI with main entry point"
```

---

## Phase 2: Enhanced Features

### Task 6: Add Visual Polish and Theming

**Files:**
- Create: `crates/todoee-cli/src/tui/theme.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`
- Modify: `crates/todoee-cli/src/tui/mod.rs`

**Step 1: Create tui/theme.rs**

Create `crates/todoee-cli/src/tui/theme.rs`:
```rust
use ratatui::style::{Color, Modifier, Style};

/// Application theme colors
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub muted: Color,
    pub border: Color,
    pub border_focused: Color,
    pub selection_bg: Color,
    pub text: Color,
    pub text_muted: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            muted: Color::DarkGray,
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            selection_bg: Color::Rgb(40, 40, 50),
            text: Color::White,
            text_muted: Color::Gray,
        }
    }
}

impl Theme {
    pub fn border_style(&self, focused: bool) -> Style {
        Style::default().fg(if focused { self.border_focused } else { self.border })
    }

    pub fn title_style(&self) -> Style {
        Style::default().fg(self.primary).add_modifier(Modifier::BOLD)
    }

    pub fn selected_style(&self) -> Style {
        Style::default().bg(self.selection_bg)
    }

    pub fn priority_high(&self) -> Style {
        Style::default().fg(self.error).add_modifier(Modifier::BOLD)
    }

    pub fn priority_medium(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn priority_low(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn completed_style(&self) -> Style {
        Style::default()
            .fg(self.muted)
            .add_modifier(Modifier::CROSSED_OUT)
    }
}
```

**Step 2: Update tui/mod.rs**

Add to `crates/todoee-cli/src/tui/mod.rs`:
```rust
pub mod app;
pub mod event;
pub mod handler;
pub mod terminal;
pub mod theme;
pub mod ui;

pub use app::App;
pub use event::{Event, EventHandler};
pub use handler::handle_key_event;
pub use terminal::Tui;
pub use theme::Theme;
```

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add theming support"
```

---

### Task 7: Add Quick Add with AI Parsing

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`
- Modify: `crates/todoee-cli/src/tui/handler.rs`

**Step 1: Update app.rs to support AI parsing**

Add to `App` struct in `crates/todoee-cli/src/tui/app.rs` (at the end of the impl block):
```rust
    /// Add a new todo with optional AI parsing
    pub async fn add_todo_with_ai(&mut self, use_ai: bool) -> Result<()> {
        let description = self.input.value().trim().to_string();
        if description.is_empty() {
            self.status_message = Some("Cannot add empty task".to_string());
            return Ok(());
        }

        let todo = if use_ai && self.config.ai.model.is_some() {
            match self.parse_with_ai(&description).await {
                Ok(t) => t,
                Err(e) => {
                    self.status_message = Some(format!("AI failed: {}, using plain text", e));
                    Todo::new(description.clone(), None)
                }
            }
        } else {
            Todo::new(description.clone(), None)
        };

        let title = todo.title.clone();
        self.db.create_todo(&todo).await?;
        self.status_message = Some(format!("✓ Added: {}", title));
        self.input.reset();
        self.refresh_todos().await?;

        Ok(())
    }

    async fn parse_with_ai(&self, description: &str) -> Result<Todo> {
        use todoee_core::{AiClient, ParsedTask};

        let client = AiClient::new(&self.config)?;
        let parsed = client.parse_task(description).await?;

        let mut todo = Todo::new(parsed.title, None);
        todo.description = parsed.description;
        todo.due_date = parsed.due_date;
        todo.reminder_at = parsed.reminder_at;

        if let Some(p) = parsed.priority {
            todo.priority = match p {
                1 => Priority::Low,
                3 => Priority::High,
                _ => Priority::Medium,
            };
        }

        todo.ai_metadata = Some(serde_json::json!({
            "original_input": description,
            "parsed_category": parsed.category,
        }));

        Ok(todo)
    }

    /// Check if AI is configured
    pub fn has_ai(&self) -> bool {
        self.config.ai.model.is_some()
    }
```

**Step 2: Update handler.rs for AI add**

Update `handle_adding_mode` in `crates/todoee-cli/src/tui/handler.rs`:
```rust
async fn handle_adding_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
        }
        KeyCode::Enter => {
            // Use AI if available and Shift not held
            let use_ai = app.has_ai() && !key.modifiers.contains(KeyModifiers::SHIFT);
            app.add_todo_with_ai(use_ai).await?;
            app.mode = Mode::Normal;
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }

    Ok(())
}
```

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add AI-powered task parsing in interactive mode"
```

---

### Task 8: Final Testing and Polish

**Files:**
- No new files

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Build release**

Run: `cargo build --release`
Expected: Success

**Step 3: Test interactive mode end-to-end**

```bash
./target/release/todoee
# Test: Press 'a', type "buy groceries tomorrow", press Enter
# Test: Press 'j'/'k' to navigate
# Test: Press 'd' to mark done
# Test: Press '/' to search
# Test: Press '?' for help
# Test: Press 'q' to quit
```

**Step 4: Test quick commands still work**

```bash
./target/release/todoee add "quick task" --no-ai
./target/release/todoee list
./target/release/todoee done <id>
```

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: interactive TUI complete with AI support"
```

---

## Summary

This plan implements a Claude Code-inspired interactive TUI for todoee with:

1. **Dual-mode operation**: Interactive TUI by default, quick commands with subcommands
2. **Vim-style navigation**: j/k, g/G for movement
3. **Quick actions**: a (add), d (done), x (delete), e (edit)
4. **Filtering**: / (search), t (today), c (categories), Tab (completed)
5. **Visual feedback**: Priority colors, due date warnings, status messages
6. **AI integration**: Smart parsing when Enter pressed in add mode
7. **Help modal**: ? shows all keybindings
8. **Clean terminal handling**: Proper alternate screen and cleanup

**Tech used:**
- ratatui for UI rendering
- crossterm for terminal events
- tui-input for text input handling
- Existing todoee-core for data layer

Sources:
- [Ratatui GitHub](https://github.com/ratatui/ratatui)
- [Ratatui Best Practices Discussion](https://github.com/ratatui/ratatui/discussions/220)
- [Ratatui Documentation](https://ratatui.rs/)
