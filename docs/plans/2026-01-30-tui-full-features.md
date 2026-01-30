# TUI Full Features Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform the basic TUI into a fully-featured application with todo details, category management, settings panel, advanced filtering, and all CLI capabilities exposed through the interface.

**Architecture:** Multi-panel TUI with modal overlays for detailed views. Tab-based navigation between main views (Todos, Categories, Settings). Each panel maintains its own state while sharing the core App state. Uses existing todoee-core infrastructure with enhanced UI interactions.

**Tech Stack:** ratatui 0.29, crossterm 0.28, tui-input 0.11, existing todoee-core library, tokio async runtime

---

## Phase 1: Enhanced Todo Display & Editing

### Task 1: Add Todo Detail Panel

**Files:**
- Create: `crates/todoee-cli/src/tui/widgets/mod.rs`
- Create: `crates/todoee-cli/src/tui/widgets/todo_detail.rs`
- Modify: `crates/todoee-cli/src/tui/mod.rs`
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Create widgets module structure**

Create `crates/todoee-cli/src/tui/widgets/mod.rs`:
```rust
pub mod todo_detail;

pub use todo_detail::TodoDetailWidget;
```

**Step 2: Create todo detail widget**

Create `crates/todoee-cli/src/tui/widgets/todo_detail.rs`:
```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use todoee_core::{Priority, Todo};
use chrono::Utc;

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
            Span::styled("✓ Completed", Style::default().fg(Color::Green))
        } else {
            Span::styled("○ Pending", Style::default().fg(Color::Yellow))
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

        let reminder_text = self.todo.reminder_at
            .map(|r| format!("Reminder: {}", r.format("%Y-%m-%d %H:%M")))
            .unwrap_or_else(|| "No reminder set".to_string());

        let category_text = self.todo.category_id
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
            Line::from(vec![
                Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(self.todo.description.as_deref().unwrap_or("(none)")),
            Line::from(""),
            Line::from(vec![
                Span::styled(&due_text, Style::default().fg(if self.todo.due_date.is_some() { Color::Cyan } else { Color::DarkGray })),
            ]),
            Line::from(vec![
                Span::styled(&reminder_text, Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled(&category_text, Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(&created, Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled(&updated, Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::DarkGray)),
                Span::styled(self.todo.id.to_string(), Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" Todo Details ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}
```

**Step 3: Update mod.rs to include widgets**

Add to `crates/todoee-cli/src/tui/mod.rs`:
```rust
pub mod widgets;
```

**Step 4: Add ViewingDetail mode to app.rs**

In `crates/todoee-cli/src/tui/app.rs`, update the Mode enum:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Adding,
    Editing,
    Searching,
    Help,
    ViewingDetail,  // NEW
}
```

**Step 5: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 6: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add todo detail widget and ViewingDetail mode"
```

---

### Task 2: Integrate Detail View with Handler and UI

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Add detail view handler**

In `crates/todoee-cli/src/tui/handler.rs`, add to handle_normal_mode:
```rust
// In handle_normal_mode, add this case:
KeyCode::Char('v') | KeyCode::Char(' ') => {
    if app.selected_todo().is_some() {
        app.mode = Mode::ViewingDetail;
    }
}
```

Add new handler function:
```rust
fn handle_detail_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('v') | KeyCode::Char(' ') => {
            app.mode = Mode::Normal;
        }
        // Allow d to mark done from detail view
        KeyCode::Char('d') | KeyCode::Enter => {
            app.mode = Mode::Normal;
            // Will be handled by normal mode on next tick
        }
        _ => {}
    }
}
```

Update handle_key_event to include:
```rust
Mode::ViewingDetail => handle_detail_mode(app, key),
```

**Step 2: Update UI to render detail overlay**

In `crates/todoee-cli/src/tui/ui.rs`, add import:
```rust
use super::widgets::TodoDetailWidget;
```

Update render function to show detail modal:
```rust
// After help modal check, add:
if app.mode == Mode::ViewingDetail {
    if let Some(todo) = app.selected_todo() {
        let area = centered_rect(70, 80, frame.area());
        TodoDetailWidget::new(todo).render(frame, area);
    }
}
```

Update render_help to show 'v' key:
```rust
Mode::Normal => "j/k:nav  a:add  d:done  x:del  e:edit  v:view  /:search  t:today  ?:help  q:quit",
```

**Step 3: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): integrate todo detail view with v/space keys"
```

---

### Task 3: Add Full Todo Edit Modal

**Files:**
- Create: `crates/todoee-cli/src/tui/widgets/todo_editor.rs`
- Modify: `crates/todoee-cli/src/tui/widgets/mod.rs`
- Modify: `crates/todoee-cli/src/tui/app.rs`
- Modify: `crates/todoee-cli/src/tui/handler.rs`

**Step 1: Create edit state struct in app.rs**

Add to `crates/todoee-cli/src/tui/app.rs`:
```rust
/// State for editing a todo with multiple fields
#[derive(Debug, Clone)]
pub struct EditState {
    pub todo_id: uuid::Uuid,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub due_date: Option<String>,  // Store as string for editing
    pub active_field: EditField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    Title,
    Description,
    Priority,
    DueDate,
}

impl EditState {
    pub fn from_todo(todo: &Todo) -> Self {
        Self {
            todo_id: todo.id,
            title: todo.title.clone(),
            description: todo.description.clone().unwrap_or_default(),
            priority: todo.priority,
            due_date: todo.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
            active_field: EditField::Title,
        }
    }
}
```

Add to App struct:
```rust
pub edit_state: Option<EditState>,
```

Initialize in App::new():
```rust
edit_state: None,
```

Update Mode enum:
```rust
pub enum Mode {
    Normal,
    Adding,
    Editing,      // Title-only quick edit
    EditingFull,  // Full multi-field edit  // NEW
    Searching,
    Help,
    ViewingDetail,
}
```

**Step 2: Create todo editor widget**

Create `crates/todoee-cli/src/tui/widgets/todo_editor.rs`:
```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use todoee_core::Priority;
use super::super::app::{EditState, EditField};

pub struct TodoEditorWidget<'a> {
    state: &'a EditState,
}

impl<'a> TodoEditorWidget<'a> {
    pub fn new(state: &'a EditState) -> Self {
        Self { state }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(5),  // Description
                Constraint::Length(3),  // Priority
                Constraint::Length(3),  // Due date
                Constraint::Length(2),  // Help
            ])
            .split(area);

        let block = Block::default()
            .title(" Edit Todo (Tab: next field, Enter: save, Esc: cancel) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        frame.render_widget(block, area);

        // Title field
        self.render_field(frame, chunks[0], "Title", &self.state.title,
            self.state.active_field == EditField::Title);

        // Description field
        self.render_field(frame, chunks[1], "Description", &self.state.description,
            self.state.active_field == EditField::Description);

        // Priority field
        let priority_text = match self.state.priority {
            Priority::High => "High (3)",
            Priority::Medium => "Medium (2)",
            Priority::Low => "Low (1)",
        };
        self.render_field(frame, chunks[2], "Priority (1/2/3)", priority_text,
            self.state.active_field == EditField::Priority);

        // Due date field
        let due_text = self.state.due_date.as_deref().unwrap_or("(none - type YYYY-MM-DD)");
        self.render_field(frame, chunks[3], "Due Date", due_text,
            self.state.active_field == EditField::DueDate);

        // Help text
        let help = Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::raw(": next  "),
            Span::styled("Shift+Tab", Style::default().fg(Color::Cyan)),
            Span::raw(": prev  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(": save  "),
            Span::styled("Esc", Style::default().fg(Color::Red)),
            Span::raw(": cancel"),
        ]));
        frame.render_widget(help, chunks[4]);
    }

    fn render_field(&self, frame: &mut Frame, area: Rect, label: &str, value: &str, active: bool) {
        let style = if active {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let border_style = if active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let cursor = if active { "│" } else { "" };

        let content = Paragraph::new(format!("{}{}", value, cursor))
            .block(
                Block::default()
                    .title(format!(" {} ", label))
                    .borders(Borders::ALL)
                    .border_style(border_style)
            )
            .style(style);

        frame.render_widget(content, area);
    }
}
```

**Step 3: Update widgets/mod.rs**

```rust
pub mod todo_detail;
pub mod todo_editor;

pub use todo_detail::TodoDetailWidget;
pub use todo_editor::TodoEditorWidget;
```

**Step 4: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add full todo editor widget with multi-field support"
```

---

### Task 4: Implement Full Edit Handler

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Add E key for full edit in handler.rs**

In handle_normal_mode, update 'e' handling:
```rust
KeyCode::Char('e') => {
    if let Some(todo) = app.selected_todo() {
        app.edit_state = Some(EditState::from_todo(todo));
        app.mode = Mode::EditingFull;
    }
}
```

Remove or keep the old Editing mode for quick title-only edit if desired.

**Step 2: Add full edit mode handler**

Add to handler.rs:
```rust
async fn handle_editing_full_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    let Some(ref mut state) = app.edit_state else {
        app.mode = Mode::Normal;
        return Ok(());
    };

    match key.code {
        KeyCode::Esc => {
            app.edit_state = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            state.active_field = match state.active_field {
                EditField::Title => EditField::Description,
                EditField::Description => EditField::Priority,
                EditField::Priority => EditField::DueDate,
                EditField::DueDate => EditField::Title,
            };
        }
        KeyCode::BackTab => {
            state.active_field = match state.active_field {
                EditField::Title => EditField::DueDate,
                EditField::Description => EditField::Title,
                EditField::Priority => EditField::Description,
                EditField::DueDate => EditField::Priority,
            };
        }
        KeyCode::Enter => {
            // Save changes
            let todo_id = state.todo_id;
            if let Some(todo) = app.todos.iter_mut().find(|t| t.id == todo_id) {
                todo.title = state.title.clone();
                todo.description = if state.description.is_empty() { None } else { Some(state.description.clone()) };
                todo.priority = state.priority;
                todo.due_date = state.due_date.as_ref().and_then(|s| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .map(|d| d.and_hms_opt(12, 0, 0).unwrap())
                        .map(|dt| chrono::Utc.from_utc_datetime(&dt))
                });
                todo.updated_at = chrono::Utc::now();
                todo.sync_status = todoee_core::SyncStatus::Pending;
                app.db.update_todo(todo).await?;
                app.status_message = Some(format!("✓ Updated: {}", todo.title));
            }
            app.edit_state = None;
            app.mode = Mode::Normal;
            app.refresh_todos().await?;
        }
        KeyCode::Char(c) => {
            match state.active_field {
                EditField::Title => state.title.push(c),
                EditField::Description => state.description.push(c),
                EditField::Priority => {
                    state.priority = match c {
                        '1' => Priority::Low,
                        '2' => Priority::Medium,
                        '3' => Priority::High,
                        _ => state.priority,
                    };
                }
                EditField::DueDate => {
                    let due = state.due_date.get_or_insert_with(String::new);
                    if c.is_ascii_digit() || c == '-' {
                        due.push(c);
                    }
                }
            }
        }
        KeyCode::Backspace => {
            match state.active_field {
                EditField::Title => { state.title.pop(); }
                EditField::Description => { state.description.pop(); }
                EditField::Priority => {} // Can't backspace priority
                EditField::DueDate => {
                    if let Some(ref mut due) = state.due_date {
                        due.pop();
                        if due.is_empty() {
                            state.due_date = None;
                        }
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}
```

Update handle_key_event:
```rust
Mode::EditingFull => handle_editing_full_mode(app, key).await?,
```

Add necessary imports:
```rust
use super::app::{EditState, EditField};
use todoee_core::Priority;
use chrono::TimeZone;
```

**Step 3: Update UI to render editor**

In ui.rs, add import:
```rust
use super::widgets::TodoEditorWidget;
```

Add to render function:
```rust
if app.mode == Mode::EditingFull {
    if let Some(ref state) = app.edit_state {
        let area = centered_rect(60, 50, frame.area());
        TodoEditorWidget::new(state).render(frame, area);
    }
}
```

**Step 4: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): implement full todo editor with all fields"
```

---

## Phase 2: Category Management

### Task 5: Create Category List View

**Files:**
- Create: `crates/todoee-cli/src/tui/widgets/category_list.rs`
- Modify: `crates/todoee-cli/src/tui/widgets/mod.rs`
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Add View enum to app.rs**

```rust
/// Main view/tab of the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Todos,
    Categories,
    Settings,
}
```

Add to App struct:
```rust
pub current_view: View,
pub category_selected: usize,
```

Initialize in App::new():
```rust
current_view: View::default(),
category_selected: 0,
```

**Step 2: Create category list widget**

Create `crates/todoee-cli/src/tui/widgets/category_list.rs`:
```rust
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use todoee_core::Category;

pub struct CategoryListWidget<'a> {
    categories: &'a [Category],
    selected: usize,
}

impl<'a> CategoryListWidget<'a> {
    pub fn new(categories: &'a [Category], selected: usize) -> Self {
        Self { categories, selected }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected;
                let selector = if is_selected { "▸ " } else { "  " };

                let color = cat.color.as_ref()
                    .and_then(|c| parse_hex_color(c))
                    .unwrap_or(Color::White);

                let ai_badge = if cat.is_ai_generated {
                    Span::styled(" [AI]", Style::default().fg(Color::Magenta))
                } else {
                    Span::raw("")
                };

                let content = Line::from(vec![
                    Span::styled(selector, Style::default().fg(Color::Cyan)),
                    Span::styled("● ", Style::default().fg(color)),
                    Span::raw(&cat.name),
                    ai_badge,
                ]);

                let style = if is_selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" Categories ({}) ", self.categories.len()))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
            );

        frame.render_widget(list, area);
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
```

**Step 3: Update widgets/mod.rs**

```rust
pub mod todo_detail;
pub mod todo_editor;
pub mod category_list;

pub use todo_detail::TodoDetailWidget;
pub use todo_editor::TodoEditorWidget;
pub use category_list::CategoryListWidget;
```

**Step 4: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add category list widget with color support"
```

---

### Task 6: Add Tab Navigation Between Views

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Add number key navigation in handler.rs**

In handle_normal_mode, add:
```rust
// View switching with number keys
KeyCode::Char('1') => {
    app.current_view = View::Todos;
}
KeyCode::Char('2') => {
    app.current_view = View::Categories;
}
KeyCode::Char('3') => {
    app.current_view = View::Settings;
}
```

Add import:
```rust
use super::app::View;
```

**Step 2: Update navigation based on view**

Modify handle_normal_mode to be view-aware:
```rust
async fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    // View switching always available
    match key.code {
        KeyCode::Char('1') => { app.current_view = View::Todos; return Ok(()); }
        KeyCode::Char('2') => { app.current_view = View::Categories; return Ok(()); }
        KeyCode::Char('3') => { app.current_view = View::Settings; return Ok(()); }
        KeyCode::Char('q') | KeyCode::Esc => { app.quit(); return Ok(()); }
        KeyCode::Char('?') => { app.mode = Mode::Help; return Ok(()); }
        _ => {}
    }

    // View-specific handling
    match app.current_view {
        View::Todos => handle_todos_view(app, key).await?,
        View::Categories => handle_categories_view(app, key).await?,
        View::Settings => handle_settings_view(app, key)?,
    }

    Ok(())
}

async fn handle_todos_view(app: &mut App, key: KeyEvent) -> Result<()> {
    // ... existing todo navigation and actions ...
}

async fn handle_categories_view(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.category_selected < app.categories.len().saturating_sub(1) {
                app.category_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.category_selected > 0 {
                app.category_selected -= 1;
            }
        }
        KeyCode::Char('a') => {
            app.mode = Mode::AddingCategory;
            app.input.reset();
        }
        // More category actions...
        _ => {}
    }
    Ok(())
}

fn handle_settings_view(app: &mut App, key: KeyEvent) -> Result<()> {
    // Settings navigation - to be implemented
    Ok(())
}
```

**Step 3: Update UI to show tabs and current view**

In ui.rs, update render function:
```rust
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
            render_settings(app, frame, chunks[2]);
        }
    }

    render_status(app, frame, chunks[3]);
    render_help(app, frame, chunks[4]);

    // Modals
    if app.mode == Mode::Help {
        render_help_modal(frame);
    }
    // ... other modals
}

fn render_tabs(app: &App, frame: &mut Frame, area: Rect) {
    let tabs = vec![
        ("1: Todos", View::Todos),
        ("2: Categories", View::Categories),
        ("3: Settings", View::Settings),
    ];

    let spans: Vec<Span> = tabs
        .iter()
        .map(|(label, view)| {
            if app.current_view == *view {
                Span::styled(
                    format!(" {} ", label),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                )
            } else {
                Span::styled(
                    format!(" {} ", label),
                    Style::default().fg(Color::DarkGray)
                )
            }
        })
        .collect();

    let tabs_line = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray))
        );

    frame.render_widget(tabs_line, area);
}
```

**Step 4: Add helper render functions**

```rust
fn render_category_header(app: &App, frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" Categories ", Style::default().fg(Color::Cyan).bold()),
        Span::raw("- Press 'a' to add, 'x' to delete"),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(header, area);
}

fn render_categories(app: &App, frame: &mut Frame, area: Rect) {
    use super::widgets::CategoryListWidget;
    CategoryListWidget::new(&app.categories, app.category_selected).render(frame, area);
}

fn render_settings_header(_app: &App, frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(" Settings ")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(header, area);
}

fn render_settings(_app: &App, frame: &mut Frame, area: Rect) {
    let placeholder = Paragraph::new("Settings panel coming soon...")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(placeholder, area);
}
```

**Step 5: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 6: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add tab navigation between Todos, Categories, Settings"
```

---

### Task 7: Implement Category CRUD Operations

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-core/src/db/local.rs`

**Step 1: Add AddingCategory mode**

In app.rs, update Mode enum:
```rust
pub enum Mode {
    Normal,
    Adding,
    Editing,
    EditingFull,
    Searching,
    Help,
    ViewingDetail,
    AddingCategory,  // NEW
}
```

Add category methods to App:
```rust
/// Add a new category
pub async fn add_category(&mut self, name: String, color: Option<String>) -> Result<()> {
    if name.is_empty() {
        self.status_message = Some("Category name cannot be empty".to_string());
        return Ok(());
    }

    // Check if category exists
    if self.db.get_category_by_name(&name).await?.is_some() {
        self.status_message = Some(format!("Category '{}' already exists", name));
        return Ok(());
    }

    let category = Category::new(name.clone(), color, false);
    self.db.create_category(&category).await?;
    self.status_message = Some(format!("✓ Created category: {}", name));
    self.refresh_categories().await?;
    Ok(())
}

/// Delete selected category
pub async fn delete_selected_category(&mut self) -> Result<()> {
    if let Some(cat) = self.categories.get(self.category_selected) {
        let name = cat.name.clone();
        self.db.delete_category(cat.id).await?;
        self.status_message = Some(format!("✗ Deleted category: {}", name));
        self.refresh_categories().await?;
        if self.category_selected >= self.categories.len() && !self.categories.is_empty() {
            self.category_selected = self.categories.len() - 1;
        }
    }
    Ok(())
}
```

**Step 2: Add delete_category to LocalDb**

In `crates/todoee-core/src/db/local.rs`, add:
```rust
/// Delete a category by ID
pub async fn delete_category(&self, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
    Ok(())
}
```

**Step 3: Add category handler**

In handler.rs, add AddingCategory handler:
```rust
async fn handle_adding_category_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
        }
        KeyCode::Enter => {
            let name = app.input.value().trim().to_string();
            app.add_category(name, None).await?;
            app.input.reset();
            app.mode = Mode::Normal;
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }
    Ok(())
}
```

Update handle_key_event:
```rust
Mode::AddingCategory => handle_adding_category_mode(app, key).await?,
```

Update handle_categories_view:
```rust
async fn handle_categories_view(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.category_selected < app.categories.len().saturating_sub(1) {
                app.category_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.category_selected > 0 {
                app.category_selected -= 1;
            }
        }
        KeyCode::Char('a') => {
            app.mode = Mode::AddingCategory;
            app.input.reset();
        }
        KeyCode::Char('x') => {
            app.delete_selected_category().await?;
        }
        _ => {}
    }
    Ok(())
}
```

**Step 4: Update UI for category adding**

In render_category_header, make it mode-aware:
```rust
fn render_category_header(app: &App, frame: &mut Frame, area: Rect) {
    if app.mode == Mode::AddingCategory {
        let input = Paragraph::new(Line::from(vec![
            Span::styled("> New category: ", Style::default().fg(Color::Green)),
            Span::raw(app.input.value()),
            Span::styled("│", Style::default().fg(Color::White)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        frame.render_widget(input, area);
    } else {
        let header = Paragraph::new(Line::from(vec![
            Span::styled(" Categories ", Style::default().fg(Color::Cyan).bold()),
            Span::styled("  a", Style::default().fg(Color::Yellow)),
            Span::raw(":add  "),
            Span::styled("x", Style::default().fg(Color::Yellow)),
            Span::raw(":delete"),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(header, area);
    }
}
```

**Step 5: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 6: Commit**

```bash
git add crates/
git commit -m "feat(tui): implement category create and delete"
```

---

## Phase 3: Settings Panel

### Task 8: Create Settings Widget

**Files:**
- Create: `crates/todoee-cli/src/tui/widgets/settings.rs`
- Modify: `crates/todoee-cli/src/tui/widgets/mod.rs`
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Add settings state to app.rs**

```rust
/// Settings panel state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    Ai,
    Display,
    Notifications,
    Database,
}
```

Add to App struct:
```rust
pub settings_section: SettingsSection,
```

Initialize:
```rust
settings_section: SettingsSection::default(),
```

**Step 2: Create settings widget**

Create `crates/todoee-cli/src/tui/widgets/settings.rs`:
```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use todoee_core::Config;
use super::super::app::SettingsSection;

pub struct SettingsWidget<'a> {
    config: &'a Config,
    section: SettingsSection,
}

impl<'a> SettingsWidget<'a> {
    pub fn new(config: &'a Config, section: SettingsSection) -> Self {
        Self { config, section }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25),  // Sidebar
                Constraint::Min(40),     // Content
            ])
            .split(area);

        self.render_sidebar(frame, chunks[0]);
        self.render_content(frame, chunks[1]);
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        let sections = [
            ("AI Settings", SettingsSection::Ai),
            ("Display", SettingsSection::Display),
            ("Notifications", SettingsSection::Notifications),
            ("Database", SettingsSection::Database),
        ];

        let items: Vec<ListItem> = sections
            .iter()
            .map(|(label, sec)| {
                let is_selected = self.section == *sec;
                let style = if is_selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let prefix = if is_selected { "▸ " } else { "  " };
                ListItem::new(format!("{}{}", prefix, label)).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Sections ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
            );

        frame.render_widget(list, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect) {
        let content = match self.section {
            SettingsSection::Ai => self.render_ai_settings(),
            SettingsSection::Display => self.render_display_settings(),
            SettingsSection::Notifications => self.render_notification_settings(),
            SettingsSection::Database => self.render_database_settings(),
        };

        let title = match self.section {
            SettingsSection::Ai => " AI Configuration ",
            SettingsSection::Display => " Display Settings ",
            SettingsSection::Notifications => " Notification Settings ",
            SettingsSection::Database => " Database Settings ",
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
            );

        frame.render_widget(paragraph, area);
    }

    fn render_ai_settings(&self) -> Vec<Line<'static>> {
        let model_status = self.config.ai.model.as_ref()
            .map(|m| format!("✓ {}", m))
            .unwrap_or_else(|| "✗ Not configured".to_string());

        let api_key_status = std::env::var(&self.config.ai.api_key_env)
            .map(|_| "✓ Set")
            .unwrap_or("✗ Not set");

        vec![
            Line::from(vec![
                Span::styled("Provider: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&self.config.ai.provider),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Model: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(model_status, Style::default().fg(
                    if self.config.ai.model.is_some() { Color::Green } else { Color::Red }
                )),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("API Key (", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&self.config.ai.api_key_env),
                Span::styled("): ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(api_key_status, Style::default().fg(
                    if api_key_status.starts_with('✓') { Color::Green } else { Color::Red }
                )),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Edit ~/.config/todoee/config.toml to configure",
                Style::default().fg(Color::DarkGray)
            )),
        ]
    }

    fn render_display_settings(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(vec![
                Span::styled("Theme: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&self.config.display.theme),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Date Format: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&self.config.display.date_format),
            ]),
        ]
    }

    fn render_notification_settings(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(vec![
                Span::styled("Enabled: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if self.config.notifications.enabled { "Yes" } else { "No" },
                    Style::default().fg(if self.config.notifications.enabled { Color::Green } else { Color::Red })
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Sound: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if self.config.notifications.sound { "Yes" } else { "No" },
                    Style::default().fg(if self.config.notifications.sound { Color::Green } else { Color::Red })
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Advance Notice: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{} minutes", self.config.notifications.advance_minutes)),
            ]),
        ]
    }

    fn render_database_settings(&self) -> Vec<Line<'static>> {
        let neon_status = std::env::var(&self.config.database.url_env)
            .map(|_| "✓ Configured")
            .unwrap_or("✗ Not configured (local only)");

        let local_db = self.config.local_db_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "Error loading path".to_string());

        vec![
            Line::from(vec![
                Span::styled("Cloud Sync (", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&self.config.database.url_env),
                Span::styled("): ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled(neon_status, Style::default().fg(
                if neon_status.starts_with('✓') { Color::Green } else { Color::Yellow }
            ))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Local Database: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled(local_db, Style::default().fg(Color::DarkGray))),
        ]
    }
}
```

**Step 3: Update widgets/mod.rs**

```rust
pub mod todo_detail;
pub mod todo_editor;
pub mod category_list;
pub mod settings;

pub use todo_detail::TodoDetailWidget;
pub use todo_editor::TodoEditorWidget;
pub use category_list::CategoryListWidget;
pub use settings::SettingsWidget;
```

**Step 4: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add settings panel with all config sections"
```

---

### Task 9: Integrate Settings Navigation

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Add settings navigation handler**

In handler.rs, update handle_settings_view:
```rust
fn handle_settings_view(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.settings_section = match app.settings_section {
                SettingsSection::Ai => SettingsSection::Display,
                SettingsSection::Display => SettingsSection::Notifications,
                SettingsSection::Notifications => SettingsSection::Database,
                SettingsSection::Database => SettingsSection::Database,
            };
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.settings_section = match app.settings_section {
                SettingsSection::Ai => SettingsSection::Ai,
                SettingsSection::Display => SettingsSection::Ai,
                SettingsSection::Notifications => SettingsSection::Display,
                SettingsSection::Database => SettingsSection::Notifications,
            };
        }
        KeyCode::Char('r') => {
            // Reload config
            if let Ok(config) = Config::load() {
                app.config = config;
                app.status_message = Some("✓ Configuration reloaded".to_string());
            }
        }
        _ => {}
    }
    Ok(())
}
```

Add import:
```rust
use super::app::SettingsSection;
use todoee_core::Config;
```

**Step 2: Update render_settings**

```rust
fn render_settings(app: &App, frame: &mut Frame, area: Rect) {
    use super::widgets::SettingsWidget;
    SettingsWidget::new(&app.config, app.settings_section).render(frame, area);
}
```

**Step 3: Update help text for settings view**

In render_help:
```rust
fn render_help(app: &App, frame: &mut Frame, area: Rect) {
    let help_text = match (app.mode, app.current_view) {
        (Mode::Adding | Mode::Editing, _) => "Enter:submit  Esc:cancel",
        (Mode::Searching, _) => "Enter:apply  Esc:cancel  Ctrl+U:clear",
        (Mode::Help, _) => "Press any key to close",
        (Mode::AddingCategory, _) => "Enter:create  Esc:cancel",
        (_, View::Todos) => "j/k:nav  a:add  d:done  x:del  e:edit  v:view  /:search  ?:help  1/2/3:tabs  q:quit",
        (_, View::Categories) => "j/k:nav  a:add  x:del  1/2/3:tabs  q:quit",
        (_, View::Settings) => "j/k:nav  r:reload config  1/2/3:tabs  q:quit",
    };

    let help = Paragraph::new(Span::styled(help_text, Style::default().fg(Color::DarkGray)));
    frame.render_widget(help, area);
}
```

**Step 4: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): integrate settings navigation and config reload"
```

---

## Phase 4: Advanced Features

### Task 10: Add Priority Filter

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Add priority filter to Filter struct**

In app.rs:
```rust
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub today_only: bool,
    pub category: Option<String>,
    pub show_completed: bool,
    pub search_query: String,
    pub priority: Option<Priority>,  // NEW
}
```

Update refresh_todos to apply priority filter:
```rust
pub async fn refresh_todos(&mut self) -> Result<()> {
    // ... existing loading logic ...

    // Apply priority filter
    if let Some(priority) = self.filter.priority {
        self.todos.retain(|t| t.priority == priority);
    }

    // ... rest of the method ...
}
```

**Step 2: Add priority toggle handler**

In handler.rs, in handle_todos_view:
```rust
KeyCode::Char('p') => {
    // Cycle priority filter: None -> High -> Medium -> Low -> None
    app.filter.priority = match app.filter.priority {
        None => Some(Priority::High),
        Some(Priority::High) => Some(Priority::Medium),
        Some(Priority::Medium) => Some(Priority::Low),
        Some(Priority::Low) => None,
    };
    app.refresh_todos().await?;
}
```

**Step 3: Update header to show priority filter**

In ui.rs, update render_header:
```rust
let priority_info = app.filter.priority.map(|p| {
    let (text, color) = match p {
        Priority::High => ("HIGH", Color::Red),
        Priority::Medium => ("MEDIUM", Color::Yellow),
        Priority::Low => ("LOW", Color::Green),
    };
    format!(" [{}] ", text)
}).unwrap_or_default();
```

**Step 4: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add priority filter with p key"
```

---

### Task 11: Add Sort Options

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`
- Modify: `crates/todoee-cli/src/tui/handler.rs`

**Step 1: Add SortBy enum and state**

In app.rs:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    #[default]
    CreatedAt,
    DueDate,
    Priority,
    Title,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}
```

Add to Filter:
```rust
pub struct Filter {
    // ... existing fields ...
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
}
```

Update refresh_todos to sort:
```rust
// After loading and filtering, sort:
match (self.filter.sort_by, self.filter.sort_order) {
    (SortBy::CreatedAt, SortOrder::Ascending) => self.todos.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
    (SortBy::CreatedAt, SortOrder::Descending) => self.todos.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
    (SortBy::DueDate, SortOrder::Ascending) => self.todos.sort_by(|a, b| a.due_date.cmp(&b.due_date)),
    (SortBy::DueDate, SortOrder::Descending) => self.todos.sort_by(|a, b| b.due_date.cmp(&a.due_date)),
    (SortBy::Priority, SortOrder::Ascending) => self.todos.sort_by(|a, b| a.priority.cmp(&b.priority)),
    (SortBy::Priority, SortOrder::Descending) => self.todos.sort_by(|a, b| b.priority.cmp(&a.priority)),
    (SortBy::Title, SortOrder::Ascending) => self.todos.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
    (SortBy::Title, SortOrder::Descending) => self.todos.sort_by(|a, b| b.title.to_lowercase().cmp(&a.title.to_lowercase())),
}
```

**Step 2: Add sort handler**

In handler.rs:
```rust
KeyCode::Char('s') => {
    // Cycle sort: Created -> DueDate -> Priority -> Title -> Created
    app.filter.sort_by = match app.filter.sort_by {
        SortBy::CreatedAt => SortBy::DueDate,
        SortBy::DueDate => SortBy::Priority,
        SortBy::Priority => SortBy::Title,
        SortBy::Title => SortBy::CreatedAt,
    };
    app.refresh_todos().await?;
    app.status_message = Some(format!("Sorted by: {:?}", app.filter.sort_by));
}
KeyCode::Char('S') => {
    // Toggle sort order
    app.filter.sort_order = match app.filter.sort_order {
        SortOrder::Ascending => SortOrder::Descending,
        SortOrder::Descending => SortOrder::Ascending,
    };
    app.refresh_todos().await?;
}
```

**Step 3: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add sort options with s/S keys"
```

---

### Task 12: Add Category Assignment in Todo Edit

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`
- Modify: `crates/todoee-cli/src/tui/widgets/todo_editor.rs`
- Modify: `crates/todoee-cli/src/tui/handler.rs`

**Step 1: Update EditState**

In app.rs:
```rust
pub struct EditState {
    pub todo_id: uuid::Uuid,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub due_date: Option<String>,
    pub category_name: Option<String>,  // NEW
    pub active_field: EditField,
}

pub enum EditField {
    Title,
    Description,
    Priority,
    DueDate,
    Category,  // NEW
}
```

Update EditState::from_todo:
```rust
impl EditState {
    pub fn from_todo(todo: &Todo, categories: &[Category]) -> Self {
        let category_name = todo.category_id.and_then(|id| {
            categories.iter().find(|c| c.id == id).map(|c| c.name.clone())
        });
        Self {
            todo_id: todo.id,
            title: todo.title.clone(),
            description: todo.description.clone().unwrap_or_default(),
            priority: todo.priority,
            due_date: todo.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
            category_name,
            active_field: EditField::Title,
        }
    }
}
```

**Step 2: Update todo_editor.rs**

Add Category field rendering and Tab navigation to include Category.

**Step 3: Update handler**

Handle category cycling in EditingFull mode:
```rust
EditField::Category => {
    // Cycle through categories or None
    let cat_names: Vec<_> = app.categories.iter().map(|c| c.name.clone()).collect();
    if cat_names.is_empty() {
        return Ok(());
    }
    state.category_name = match &state.category_name {
        None => Some(cat_names[0].clone()),
        Some(current) => {
            if let Some(idx) = cat_names.iter().position(|n| n == current) {
                if idx + 1 < cat_names.len() {
                    Some(cat_names[idx + 1].clone())
                } else {
                    None
                }
            } else {
                Some(cat_names[0].clone())
            }
        }
    };
}
```

**Step 4: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/
git commit -m "feat(tui): add category assignment in todo editor"
```

---

## Phase 5: Final Polish

### Task 13: Update Help Modal

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Update help modal with all new features**

```rust
fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(70, 85, frame.area());

    let help_text = vec![
        Line::from(Span::styled("Keyboard Shortcuts", Style::default().bold().fg(Color::Cyan))),
        Line::from(""),
        Line::from(Span::styled("Views (available everywhere)", Style::default().fg(Color::Yellow))),
        Line::from("  1           Switch to Todos view"),
        Line::from("  2           Switch to Categories view"),
        Line::from("  3           Switch to Settings view"),
        Line::from("  ?           Show this help"),
        Line::from("  q / Esc     Quit"),
        Line::from(""),
        Line::from(Span::styled("Todos View", Style::default().fg(Color::Yellow))),
        Line::from("  j / ↓       Move down"),
        Line::from("  k / ↑       Move up"),
        Line::from("  g           Go to top"),
        Line::from("  G           Go to bottom"),
        Line::from("  a           Add new task (AI enabled)"),
        Line::from("  d / Enter   Mark as done"),
        Line::from("  x           Delete task"),
        Line::from("  e           Edit task (full editor)"),
        Line::from("  v / Space   View task details"),
        Line::from("  /           Search tasks"),
        Line::from("  t           Toggle today filter"),
        Line::from("  c           Cycle category filter"),
        Line::from("  p           Cycle priority filter"),
        Line::from("  s           Cycle sort field"),
        Line::from("  S           Toggle sort order"),
        Line::from("  Tab         Toggle show completed"),
        Line::from(""),
        Line::from(Span::styled("Categories View", Style::default().fg(Color::Yellow))),
        Line::from("  j/k         Navigate"),
        Line::from("  a           Add new category"),
        Line::from("  x           Delete category"),
        Line::from(""),
        Line::from(Span::styled("Settings View", Style::default().fg(Color::Yellow))),
        Line::from("  j/k         Navigate sections"),
        Line::from("  r           Reload configuration"),
        Line::from(""),
        Line::from(Span::styled("Editor (when editing)", Style::default().fg(Color::Yellow))),
        Line::from("  Tab         Next field"),
        Line::from("  Shift+Tab   Previous field"),
        Line::from("  Enter       Save changes"),
        Line::from("  Esc         Cancel"),
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
```

**Step 2: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Success

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "docs(tui): update help modal with all new features"
```

---

### Task 14: Final Testing and Cleanup

**Files:**
- No new files

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Build release**

Run: `cargo build --release`
Expected: Success with minimal warnings

**Step 3: Manual testing checklist**

```bash
./target/release/todoee
```

Test each feature:
- [ ] Tab between views with 1/2/3
- [ ] Add todo with 'a', verify AI parsing
- [ ] View todo details with 'v' or Space
- [ ] Edit todo with 'e', modify all fields
- [ ] Mark done with 'd', delete with 'x'
- [ ] Filter by today (t), category (c), priority (p)
- [ ] Sort with 's', reverse with 'S'
- [ ] Search with '/'
- [ ] Add/delete categories in Categories view
- [ ] View all settings sections
- [ ] Reload config with 'r' in Settings
- [ ] Help modal with '?'
- [ ] Quit with 'q'

**Step 4: Fix any warnings**

Run: `cargo clippy -p todoee-cli`
Fix any warnings

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore(tui): complete full-featured TUI implementation"
```

---

## Summary

This plan implements a fully-featured TUI with:

1. **Enhanced Todo Management**
   - Detail view modal (v/Space)
   - Full editor with all fields (e)
   - AI-powered quick add (a)

2. **Category Management**
   - Category list view (Tab 2)
   - Create/delete categories
   - Color support

3. **Settings Panel**
   - AI configuration display
   - Display settings
   - Notification settings
   - Database status

4. **Advanced Filtering & Sorting**
   - Priority filter (p)
   - Multiple sort options (s/S)
   - Category filter (c)
   - Today filter (t)
   - Text search (/)

5. **Improved UX**
   - Tab navigation (1/2/3)
   - Comprehensive help modal
   - Status messages
   - Keyboard-driven interface

**Total Tasks:** 14
**Estimated Commits:** 14
