# TUI Full Add Mode Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the simple single-line add mode with a full multi-field modal that gives users complete control over all task fields when creating a new task.

**Architecture:** Introduce `Mode::AddingFull` and `AddState` similar to `EditingFull`/`EditState`. Reuse the existing `TodoEditorWidget` pattern for the add modal, with fields for Title, Description, Priority, Due Date, Category, and Reminder.

**Tech Stack:** ratatui 0.29, crossterm 0.28, Rust async

---

## Analysis

Currently when pressing `a`, users can only type a title and optionally set priority with Tab. The Todo model supports many more fields:
- `title` (required)
- `description` (optional)
- `priority` (Low/Medium/High)
- `due_date` (optional DateTime)
- `reminder_at` (optional DateTime)
- `category_id` (optional, linked to Category)

We should give users full control over these fields in a modal similar to the edit modal.

---

## Phase 1: Add State Infrastructure

### Task 1: Create AddState and AddField Types

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Add AddField enum after EditField**

```rust
/// Field being edited in add mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddField {
    #[default]
    Title,
    Description,
    Priority,
    DueDate,
    Reminder,
    Category,
}
```

**Step 2: Add AddState struct after EditState**

```rust
/// State for adding a new todo with multiple fields
#[derive(Debug, Clone, Default)]
pub struct AddState {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub due_date: Option<String>,     // YYYY-MM-DD format
    pub reminder: Option<String>,      // YYYY-MM-DD HH:MM format
    pub category_name: Option<String>,
    pub active_field: AddField,
}

impl AddState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_valid(&self) -> bool {
        !self.title.trim().is_empty()
    }
}
```

**Step 3: Add AddingFull variant to Mode enum**

```rust
/// Adding a new task with full fields
AddingFull,
```

**Step 4: Add add_state field to App struct**

```rust
/// Add state for creating new todos
pub add_state: Option<AddState>,
```

**Step 5: Initialize add_state in App::new()**

```rust
add_state: None,
```

**Step 6: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 7: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "feat(tui): add AddState and AddField types for full add mode"
```

---

### Task 2: Create TodoAddWidget

**Files:**
- Create: `crates/todoee-cli/src/tui/widgets/todo_add.rs`
- Modify: `crates/todoee-cli/src/tui/widgets/mod.rs`

**Step 1: Create todo_add.rs**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
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
        required: bool,
    ) {
        let border_color = if active {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let label_style = if required && active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
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
                    .title(ratatui::text::Span::styled(format!(" {} ", label), label_style))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .style(text_style);

        frame.render_widget(content, area);
    }
}
```

**Step 2: Update widgets/mod.rs to export TodoAddWidget**

Add to imports:
```rust
mod todo_add;
```

Add to exports:
```rust
pub use todo_add::TodoAddWidget;
```

**Step 3: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/tui/widgets/
git commit -m "feat(tui): add TodoAddWidget for full add mode"
```

---

## Phase 2: UI Integration

### Task 3: Render Add Modal in UI

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Import TodoAddWidget**

Update the import:
```rust
use super::widgets::{CategoryListWidget, SettingsWidget, TodoAddWidget, TodoDetailWidget, TodoEditorWidget};
```

**Step 2: Add modal rendering for AddingFull mode**

In the `render` function, after the EditingFull modal check, add:

```rust
if app.mode == Mode::AddingFull
    && let Some(ref state) = app.add_state
{
    let area = centered_rect(65, 60, frame.area());
    TodoAddWidget::new(state).render(frame, area);
}
```

**Step 3: Update render_help for AddingFull mode**

In the `render_help` function, add a case:

```rust
Mode::AddingFull => "Tab:next  Shift+Tab:prev  Enter:save  Esc:cancel",
```

**Step 4: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "feat(tui): render TodoAddWidget in UI"
```

---

### Task 4: Handle AddingFull Mode Input

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs`

**Step 1: Import AddField**

Update imports:
```rust
use super::app::{App, AddField, EditField, EditState, Mode, SettingsSection, SortBy, SortOrder, View};
```

**Step 2: Add handler call in handle_key_event**

Add case for AddingFull mode:
```rust
Mode::AddingFull => handle_adding_full_mode(app, key).await?,
```

**Step 3: Update handle_todos_view to enter AddingFull**

Change the 'a' key handler:
```rust
KeyCode::Char('a') => {
    app.add_state = Some(AddState::new());
    app.mode = Mode::AddingFull;
}
```

Also import AddState at the top.

**Step 4: Create handle_adding_full_mode function**

```rust
async fn handle_adding_full_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    let Some(ref mut state) = app.add_state else {
        app.mode = Mode::Normal;
        return Ok(());
    };

    match key.code {
        KeyCode::Esc => {
            app.add_state = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            state.active_field = match state.active_field {
                AddField::Title => AddField::Description,
                AddField::Description => AddField::Priority,
                AddField::Priority => AddField::DueDate,
                AddField::DueDate => AddField::Reminder,
                AddField::Reminder => AddField::Category,
                AddField::Category => AddField::Title,
            };
        }
        KeyCode::BackTab => {
            state.active_field = match state.active_field {
                AddField::Title => AddField::Category,
                AddField::Description => AddField::Title,
                AddField::Priority => AddField::Description,
                AddField::DueDate => AddField::Priority,
                AddField::Reminder => AddField::DueDate,
                AddField::Category => AddField::Reminder,
            };
        }
        KeyCode::Enter => {
            if state.is_valid() {
                app.create_todo_from_add_state().await?;
                app.add_state = None;
                app.mode = Mode::Normal;
            } else {
                app.status_message = Some("Title is required".to_string());
            }
        }
        KeyCode::Char(c) => {
            match state.active_field {
                AddField::Title => state.title.push(c),
                AddField::Description => state.description.push(c),
                AddField::Priority => {
                    state.priority = match c {
                        '1' => Priority::Low,
                        '2' => Priority::Medium,
                        '3' => Priority::High,
                        _ => state.priority,
                    };
                }
                AddField::DueDate => {
                    let due = state.due_date.get_or_insert_with(String::new);
                    if c.is_ascii_alphanumeric() || c == '-' || c == '+' {
                        due.push(c);
                    }
                }
                AddField::Reminder => {
                    let rem = state.reminder.get_or_insert_with(String::new);
                    if c.is_ascii_digit() || c == '-' || c == ':' || c == ' ' {
                        rem.push(c);
                    }
                }
                AddField::Category => {
                    // Cycle through categories with any key
                    let cat_names: Vec<_> = app.categories.iter().map(|c| c.name.clone()).collect();
                    if !cat_names.is_empty() {
                        state.category_name = match &state.category_name {
                            None => Some(cat_names[0].clone()),
                            Some(current) => {
                                let idx = cat_names.iter().position(|n| n == current).unwrap_or(0);
                                if idx + 1 < cat_names.len() {
                                    Some(cat_names[idx + 1].clone())
                                } else {
                                    None
                                }
                            }
                        };
                    }
                }
            }
        }
        KeyCode::Backspace => {
            match state.active_field {
                AddField::Title => { state.title.pop(); }
                AddField::Description => { state.description.pop(); }
                AddField::Priority => {} // Can't backspace priority
                AddField::DueDate => {
                    if let Some(ref mut due) = state.due_date {
                        due.pop();
                        if due.is_empty() {
                            state.due_date = None;
                        }
                    }
                }
                AddField::Reminder => {
                    if let Some(ref mut rem) = state.reminder {
                        rem.pop();
                        if rem.is_empty() {
                            state.reminder = None;
                        }
                    }
                }
                AddField::Category => {
                    state.category_name = None;
                }
            }
        }
        _ => {}
    }

    Ok(())
}
```

**Step 5: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 6: Commit**

```bash
git add crates/todoee-cli/src/tui/handler.rs
git commit -m "feat(tui): handle AddingFull mode input"
```

---

## Phase 3: Todo Creation Logic

### Task 5: Add create_todo_from_add_state Method

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Add imports for date parsing**

Make sure chrono is available:
```rust
use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc, Local, Duration};
```

**Step 2: Add helper function to parse relative dates**

```rust
/// Parse a date string that can be:
/// - Absolute: "2026-01-30"
/// - Relative: "today", "tomorrow", "+3d", "+1w"
fn parse_due_date(input: &str) -> Option<chrono::DateTime<Utc>> {
    let input = input.trim().to_lowercase();
    let today = Local::now().date_naive();

    let date = match input.as_str() {
        "today" => today,
        "tomorrow" => today + Duration::days(1),
        s if s.starts_with('+') && s.ends_with('d') => {
            let days: i64 = s[1..s.len()-1].parse().ok()?;
            today + Duration::days(days)
        }
        s if s.starts_with('+') && s.ends_with('w') => {
            let weeks: i64 = s[1..s.len()-1].parse().ok()?;
            today + Duration::weeks(weeks)
        }
        _ => NaiveDate::parse_from_str(&input, "%Y-%m-%d").ok()?,
    };

    date.and_hms_opt(12, 0, 0)
        .map(|dt| Utc.from_utc_datetime(&dt))
}

/// Parse a reminder datetime string: "2026-01-30 14:00"
fn parse_reminder(input: &str) -> Option<chrono::DateTime<Utc>> {
    let input = input.trim();
    NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M")
        .ok()
        .map(|dt| Utc.from_utc_datetime(&dt))
}
```

**Step 3: Add create_todo_from_add_state method**

```rust
/// Create a todo from the current add state
pub async fn create_todo_from_add_state(&mut self) -> Result<()> {
    let Some(ref state) = self.add_state else {
        return Ok(());
    };

    self.set_loading("Creating task...");

    let mut todo = Todo::new(state.title.trim().to_string(), None);

    // Description
    if !state.description.is_empty() {
        todo.description = Some(state.description.clone());
    }

    // Priority
    todo.priority = state.priority;

    // Due date
    if let Some(ref due_str) = state.due_date {
        todo.due_date = parse_due_date(due_str);
    }

    // Reminder
    if let Some(ref rem_str) = state.reminder {
        todo.reminder_at = parse_reminder(rem_str);
    }

    // Category
    if let Some(ref cat_name) = state.category_name {
        if let Some(cat) = self.categories.iter().find(|c| &c.name == cat_name) {
            todo.category_id = Some(cat.id);
        }
    }

    let title = todo.title.clone();
    self.db.create_todo(&todo).await?;
    self.clear_loading();
    self.status_message = Some(format!("✓ Added: {}", title));
    self.refresh_todos().await?;

    Ok(())
}
```

**Step 4: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "feat(tui): add create_todo_from_add_state method"
```

---

## Phase 4: Quick Add Option

### Task 6: Keep Quick Add as Alternative

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs`

**Step 1: Make 'a' open full add, 'A' (shift+a) for quick add**

Update handle_todos_view:

```rust
KeyCode::Char('a') => {
    app.add_state = Some(AddState::new());
    app.mode = Mode::AddingFull;
}
KeyCode::Char('A') => {
    // Quick add with AI parsing
    app.mode = Mode::Adding;
    app.input.reset();
    app.pending_priority = None;
}
```

**Step 2: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/handler.rs
git commit -m "feat(tui): add quick add mode with Shift+A"
```

---

## Phase 5: Help Updates

### Task 7: Update Help Modal

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Update Todos View section in help modal**

Find the Todos View section and update the 'a' line:

```rust
Line::from("  a           Add task (full editor)"),
Line::from("  A           Quick add (AI-powered)"),
```

**Step 2: Update the "Adding Task" section title to "Quick Add Mode"**

Find the "Adding Task" section and rename:

```rust
Line::from(Span::styled(
    "Quick Add Mode (Shift+A)",
    Style::default().fg(Color::Yellow),
)),
```

**Step 3: Add a "Full Add Mode" section**

```rust
Line::from(""),
Line::from(Span::styled(
    "Full Add Mode (a)",
    Style::default().fg(Color::Yellow),
)),
Line::from("  Tab         Next field"),
Line::from("  Shift+Tab   Previous field"),
Line::from("  1/2/3       Set priority (on Priority field)"),
Line::from("  Enter       Save (requires title)"),
Line::from("  Esc         Cancel"),
```

**Step 4: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "docs(tui): update help modal with full add mode"
```

---

## Phase 6: Final Testing

### Task 8: Final Testing and Cleanup

**Step 1: Run all tests**

```bash
cargo test --workspace
```

**Step 2: Build release**

```bash
cargo build --release -p todoee-cli
```

**Step 3: Run clippy**

```bash
cargo clippy -p todoee-cli -- -D warnings 2>&1 || true
```

**Step 4: Fix any issues and commit if needed**

```bash
git add -A
git commit -m "chore(tui): final cleanup for full add mode"
```

---

## Summary

This plan adds:
1. **AddState and AddField** - State management for multi-field task creation
2. **TodoAddWidget** - Modal widget with 6 editable fields
3. **UI Integration** - Rendering the add modal
4. **Input Handling** - Tab navigation, field-specific input
5. **Date Parsing** - Support for "today", "tomorrow", "+3d", "+1w", and absolute dates
6. **Quick Add Option** - Shift+A for the original AI-powered quick add
7. **Updated Help** - Documentation of all new features

**Fields Available in Full Add Mode:**
- Title (required)
- Description
- Priority (1/2/3 keys)
- Due Date (YYYY-MM-DD, "today", "tomorrow", "+3d", "+1w")
- Reminder (YYYY-MM-DD HH:MM)
- Category (cycle with any key)

Total: 8 tasks
