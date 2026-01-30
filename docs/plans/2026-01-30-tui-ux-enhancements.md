# TUI UX Enhancements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add loading indicators for async operations, priority selection when adding tasks, and visual feedback for better user experience.

**Architecture:** Introduce a `loading` state in App, show spinners during async operations, and add an inline priority picker when creating tasks. Use optimistic UI for local-only operations where possible.

**Tech Stack:** ratatui 0.29, crossterm 0.28, Rust async

---

## Analysis

The slowdown when adding tasks comes from the AI API call to OpenRouter (`parse_task` in ai.rs:172). This is a network request that can take 1-3 seconds. Currently the UI freezes during this operation because `handle_adding_mode` awaits the async operation without visual feedback.

**Solution:** Add a loading indicator that shows while the AI is processing, and allow setting priority during the "Adding" mode.

---

## Phase 1: Loading State Infrastructure

### Task 1: Add Loading State to App

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Add loading field to App struct**

Add to the `App` struct (around line 116):

```rust
/// Whether an async operation is in progress
pub is_loading: bool,
/// Loading message to display
pub loading_message: Option<String>,
```

**Step 2: Initialize loading fields in App::new**

In `App::new()` (around line 154), add to the struct initialization:

```rust
is_loading: false,
loading_message: None,
```

**Step 3: Add helper methods**

Add these methods to `impl App`:

```rust
/// Set loading state with a message
pub fn set_loading(&mut self, message: &str) {
    self.is_loading = true;
    self.loading_message = Some(message.to_string());
}

/// Clear loading state
pub fn clear_loading(&mut self) {
    self.is_loading = false;
    self.loading_message = None;
}
```

**Step 4: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "feat(tui): add loading state infrastructure"
```

---

### Task 2: Render Loading Indicator in UI

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Create render_loading_overlay function**

Add this function after `render_help_modal`:

```rust
fn render_loading_overlay(app: &App, frame: &mut Frame) {
    let area = centered_rect(40, 15, frame.area());

    // Animated spinner characters
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    // Use tick count from frame for animation (approximate with timestamp)
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() / 100) as usize % spinner_chars.len();
    let spinner = spinner_chars[idx];

    let message = app.loading_message.as_deref().unwrap_or("Loading...");

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}  {}", spinner, message),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let loading = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Processing ")
        )
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(Clear, area);
    frame.render_widget(loading, area);
}
```

**Step 2: Call render_loading_overlay in render function**

Add at the end of the `render` function (after help modal check, around line 62):

```rust
// Loading overlay (always on top)
if app.is_loading {
    render_loading_overlay(app, frame);
}
```

**Step 3: Add Alignment import**

Update the import at the top of ui.rs to include `Alignment`:

```rust
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
```

**Step 4: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "feat(tui): add loading indicator overlay"
```

---

### Task 3: Show Loading During Task Creation

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Refactor add_todo_with_ai to support loading state**

The key insight: We need to show loading BEFORE the async call, not after. The current synchronous approach doesn't allow this. We need to split the operation.

Modify the `add_todo_with_ai` method:

```rust
/// Add a new todo with optional AI parsing
pub async fn add_todo_with_ai(&mut self, use_ai: bool) -> Result<()> {
    let description = self.input.value().trim().to_string();
    if description.is_empty() {
        self.status_message = Some("Cannot add empty task".to_string());
        return Ok(());
    }

    // Show loading indicator for AI parsing
    if use_ai && self.config.ai.model.is_some() {
        self.set_loading("Parsing with AI...");
    }

    let todo = if use_ai && self.config.ai.model.is_some() {
        match self.parse_with_ai(&description).await {
            Ok(t) => {
                self.clear_loading();
                t
            }
            Err(e) => {
                self.clear_loading();
                self.status_message = Some(format!("AI failed: {}, using plain text", e));
                Todo::new(description.clone(), None)
            }
        }
    } else {
        Todo::new(description.clone(), None)
    };

    // Apply pending priority if set
    let mut todo = todo;
    if let Some(priority) = self.pending_priority.take() {
        todo.priority = priority;
    }

    let title = todo.title.clone();
    self.db.create_todo(&todo).await?;
    self.status_message = Some(format!("✓ Added: {}", title));
    self.input.reset();
    self.refresh_todos().await?;

    Ok(())
}
```

**Step 2: Add pending_priority field to App struct**

Add to App struct:

```rust
/// Priority to apply when adding a task
pub pending_priority: Option<Priority>,
```

**Step 3: Initialize pending_priority in App::new**

Add to struct initialization:

```rust
pending_priority: None,
```

**Step 4: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "feat(tui): show loading indicator during AI parsing"
```

---

## Phase 2: Priority Selection When Adding Tasks

### Task 4: Add Priority Picker to Adding Mode

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Update render_input to show priority selection**

Replace the `render_input` function with:

```rust
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
```

**Step 2: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "feat(tui): show priority indicator in add mode"
```

---

### Task 5: Handle Priority Keys in Adding Mode

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs`

**Step 1: Update handle_adding_mode to support priority keys**

Replace the `handle_adding_mode` function:

```rust
async fn handle_adding_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
            app.pending_priority = None;
        }
        KeyCode::Enter => {
            // Use AI if available and Shift not held
            let use_ai = app.has_ai() && !key.modifiers.contains(KeyModifiers::SHIFT);
            app.add_todo_with_ai(use_ai).await?;
            app.mode = Mode::Normal;
            app.pending_priority = None;
        }
        // Priority shortcuts: Ctrl+1/2/3 or Alt+1/2/3
        KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) => {
            app.pending_priority = Some(Priority::Low);
        }
        KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) => {
            app.pending_priority = Some(Priority::Medium);
        }
        KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) => {
            app.pending_priority = Some(Priority::High);
        }
        // Tab cycles priority
        KeyCode::Tab => {
            app.pending_priority = match app.pending_priority {
                None => Some(Priority::Low),
                Some(Priority::Low) => Some(Priority::Medium),
                Some(Priority::Medium) => Some(Priority::High),
                Some(Priority::High) => None,
            };
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }

    Ok(())
}
```

**Step 2: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/handler.rs
git commit -m "feat(tui): add priority selection in add mode"
```

---

### Task 6: Update Help Text for Adding Mode

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Update render_help for Adding mode**

In the `render_help` function, update the `Mode::Adding` case (around line 295):

```rust
Mode::Adding => "Enter:submit  Shift+Enter:no-AI  Tab:priority  Esc:cancel",
```

**Step 2: Update render_help_modal with new shortcuts**

In the `render_help_modal` function, add a section after the Editor section:

```rust
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
```

**Step 3: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "docs(tui): update help with priority selection shortcuts"
```

---

## Phase 3: Loading Indicators for Other Operations

### Task 7: Add Loading for Category Operations

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Update add_category to show loading**

Replace the `add_category` method:

```rust
/// Add a new category
pub async fn add_category(&mut self, name: String, color: Option<String>) -> Result<()> {
    if name.is_empty() {
        self.status_message = Some("Category name cannot be empty".to_string());
        return Ok(());
    }

    self.set_loading("Creating category...");

    // Check if category exists
    if self.db.get_category_by_name(&name).await?.is_some() {
        self.clear_loading();
        self.status_message = Some(format!("Category '{}' already exists", name));
        return Ok(());
    }

    let mut category = Category::new(uuid::Uuid::nil(), name.clone());
    category.color = color;
    self.db.create_category(&category).await?;
    self.clear_loading();
    self.status_message = Some(format!("Created category: {}", name));
    self.refresh_categories().await?;
    Ok(())
}
```

**Step 2: Update delete_selected_category to show loading**

Replace the `delete_selected_category` method:

```rust
/// Delete selected category
pub async fn delete_selected_category(&mut self) -> Result<()> {
    if let Some(cat) = self.categories.get(self.category_selected) {
        let name = cat.name.clone();
        let id = cat.id;

        self.set_loading("Deleting category...");
        self.db.delete_category(id).await?;
        self.clear_loading();

        self.status_message = Some(format!("Deleted category: {}", name));
        self.refresh_categories().await?;
        if self.category_selected >= self.categories.len() && !self.categories.is_empty() {
            self.category_selected = self.categories.len() - 1;
        }
    }
    Ok(())
}
```

**Step 3: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "feat(tui): add loading indicators for category operations"
```

---

### Task 8: Add Loading for Todo Operations

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Update mark_selected_done to show loading**

Replace the `mark_selected_done` method:

```rust
/// Mark selected todo as done
pub async fn mark_selected_done(&mut self) -> Result<()> {
    if let Some(todo) = self.todos.get_mut(self.selected) {
        if !todo.is_completed {
            self.set_loading("Completing task...");
            todo.mark_complete();
            self.db.update_todo(todo).await?;
            self.clear_loading();
            self.status_message = Some(format!("✓ Completed: {}", todo.title));
            self.refresh_todos().await?;
        } else {
            self.status_message = Some("Already completed".to_string());
        }
    }
    Ok(())
}
```

**Step 2: Update delete_selected to show loading**

Replace the `delete_selected` method:

```rust
/// Delete selected todo
pub async fn delete_selected(&mut self) -> Result<()> {
    if let Some(todo) = self.todos.get(self.selected) {
        let title = todo.title.clone();
        self.set_loading("Deleting task...");
        self.db.delete_todo(todo.id).await?;
        self.clear_loading();
        self.status_message = Some(format!("✗ Deleted: {}", title));
        self.refresh_todos().await?;
    }
    Ok(())
}
```

**Step 3: Verify compilation**

Run: `cargo build -p todoee-cli`

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "feat(tui): add loading indicators for todo operations"
```

---

## Phase 4: Final Polish

### Task 9: Update Help Modal with All New Features

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Ensure help modal is complete**

Verify the help modal includes all the new features added. The previous task (Task 6) should have added the Adding Task section.

**Step 2: Run all tests**

Run: `cargo test --workspace`

**Step 3: Build release**

Run: `cargo build --release -p todoee-cli`

**Step 4: Commit if any changes**

```bash
git add -A
git commit -m "chore(tui): final polish for UX enhancements"
```

---

### Task 10: Manual Testing Checklist

**Test the following scenarios:**

1. **Loading indicator during AI task creation:**
   - Press `a` to add a task
   - Type "buy milk tomorrow"
   - Press `Enter` (with AI enabled)
   - Verify: Loading overlay appears with spinner
   - Verify: Task is created when loading completes

2. **Priority selection when adding:**
   - Press `a` to add a task
   - Press `Tab` multiple times
   - Verify: Priority cycles: None → Low → Medium → High → None
   - Verify: Priority indicator shows in input bar

3. **Priority with Ctrl/Alt shortcuts:**
   - Press `a` to add a task
   - Press `Ctrl+3`
   - Verify: Priority shows as High (!!!)

4. **Shift+Enter bypasses AI:**
   - Press `a` to add a task
   - Type "test task"
   - Press `Shift+Enter`
   - Verify: No loading indicator, task added immediately

5. **Loading during category creation:**
   - Go to Categories view (press `2`)
   - Press `a`, type a category name
   - Press `Enter`
   - Verify: Brief loading indicator appears

6. **Loading during todo completion:**
   - Select a todo
   - Press `d` to mark done
   - Verify: Brief loading indicator appears

7. **Help modal shows new shortcuts:**
   - Press `?`
   - Verify: "Adding Task" section exists with Tab, Ctrl+1/2/3 shortcuts

---

## Summary

This plan adds:
1. **Loading state infrastructure** - `is_loading` and `loading_message` fields in App
2. **Visual loading overlay** - Animated spinner with message
3. **Priority selection in add mode** - Tab cycles, Ctrl+1/2/3 for direct selection
4. **Loading indicators for all async operations** - AI parsing, category ops, todo ops
5. **Updated help documentation** - All new shortcuts documented

Total: 10 tasks
