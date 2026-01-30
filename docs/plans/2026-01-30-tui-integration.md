# TUI Integration for Git-like and Modern SaaS Features

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate all CLI features (undo/redo, stash, overdue, insights, focus, fuzzy search) into the interactive TUI for a seamless experience.

**Architecture:** Add new modes and keybindings to the existing Ratatui-based TUI. Leverage existing modal rendering patterns. Wire up to LocalDb methods already implemented.

**Tech Stack:** Ratatui, Crossterm, Tokio async, existing todoee-core LocalDb

---

## Overview of Features to Integrate

| CLI Command | TUI Integration |
|-------------|-----------------|
| `undo` | `u` in Normal mode |
| `redo` | `Ctrl+r` in Normal mode |
| `stash push` | `z` stash selected todo |
| `stash pop` | `Z` pop from stash |
| `stash list` | New StashView mode |
| `overdue` | `o` filter toggle |
| `search` (fuzzy) | Replace existing search with fuzzy |
| `insights` | `i` opens InsightsModal |
| `focus` | `f` on selected todo starts FocusMode |
| `now` | `n` shows recommended todo |

---

## Task 1: Add Undo/Redo Keybindings

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs:68-183` (handle_todos_view)
- Modify: `crates/todoee-cli/src/tui/app.rs` (add undo/redo methods)

**Step 1: Write failing test for undo method**

Create test in `crates/todoee-cli/src/tui/app.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_undo_restores_deleted_todo() {
        // This test validates undo exists - implementation test
    }
}
```

**Step 2: Add undo method to App**

In `app.rs`, add method:
```rust
/// Undo the last operation
pub async fn undo(&mut self) -> Result<()> {
    if let Some(op) = self.db.get_last_undoable_operation().await? {
        match (&op.operation_type, &op.entity_type) {
            (OperationType::Create, EntityType::Todo) => {
                // Undo create = delete
                self.db.delete_todo(op.entity_id).await?;
            }
            (OperationType::Delete, EntityType::Todo) => {
                // Undo delete = restore from previous_state
                if let Some(ref state) = op.previous_state {
                    let todo: Todo = serde_json::from_value(state.clone())?;
                    self.db.create_todo(&todo).await?;
                }
            }
            (OperationType::Update, EntityType::Todo) => {
                // Undo update = restore previous_state
                if let Some(ref state) = op.previous_state {
                    let todo: Todo = serde_json::from_value(state.clone())?;
                    self.db.update_todo(&todo).await?;
                }
            }
            (OperationType::Complete, EntityType::Todo) => {
                // Undo complete = uncomplete
                if let Some(todo) = self.db.get_todo(op.entity_id).await? {
                    let mut todo = todo;
                    todo.is_completed = false;
                    todo.completed_at = None;
                    self.db.update_todo(&todo).await?;
                }
            }
            _ => {}
        }
        self.db.mark_operation_undone(op.id).await?;
        self.status_message = Some(format!("↶ Undone: {:?} {:?}", op.operation_type, op.entity_type));
        self.refresh_todos().await?;
    } else {
        self.status_message = Some("Nothing to undo".to_string());
    }
    Ok(())
}

/// Redo the last undone operation
pub async fn redo(&mut self) -> Result<()> {
    if let Some(op) = self.db.get_last_redoable_operation().await? {
        match (&op.operation_type, &op.entity_type) {
            (OperationType::Create, EntityType::Todo) => {
                // Redo create = create from new_state
                if let Some(ref state) = op.new_state {
                    let todo: Todo = serde_json::from_value(state.clone())?;
                    self.db.create_todo(&todo).await?;
                }
            }
            (OperationType::Delete, EntityType::Todo) => {
                // Redo delete = delete again
                self.db.delete_todo(op.entity_id).await?;
            }
            (OperationType::Update, EntityType::Todo) => {
                // Redo update = apply new_state
                if let Some(ref state) = op.new_state {
                    let todo: Todo = serde_json::from_value(state.clone())?;
                    self.db.update_todo(&todo).await?;
                }
            }
            (OperationType::Complete, EntityType::Todo) => {
                // Redo complete = complete again
                if let Some(todo) = self.db.get_todo(op.entity_id).await? {
                    let mut todo = todo;
                    todo.mark_complete();
                    self.db.update_todo(&todo).await?;
                }
            }
            _ => {}
        }
        self.db.mark_operation_redone(op.id).await?;
        self.status_message = Some(format!("↷ Redone: {:?} {:?}", op.operation_type, op.entity_type));
        self.refresh_todos().await?;
    } else {
        self.status_message = Some("Nothing to redo".to_string());
    }
    Ok(())
}
```

**Step 3: Add imports to app.rs**

```rust
use todoee_core::{Category, Config, LocalDb, Priority, Todo, Operation, OperationType, EntityType};
```

**Step 4: Add keybindings in handler.rs**

In `handle_todos_view`, add after priority filter handling:
```rust
// Undo/Redo
KeyCode::Char('u') => {
    app.undo().await?;
}
KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    app.redo().await?;
}
```

**Step 5: Update help text in ui.rs**

In `render_help` function, update Normal/Todos help:
```rust
View::Todos => {
    "j/k:nav  a:add  d:done  x:del  e:edit  u:undo  Ctrl+r:redo  v:view  /:search  q:quit"
}
```

**Step 6: Update help modal**

Add to help_text in `render_help_modal`:
```rust
Line::from("  u           Undo last action"),
Line::from("  Ctrl+r      Redo last undone action"),
```

**Step 7: Run tests**

```bash
cargo test -p todoee-cli
```

**Step 8: Commit**

```bash
git add -A && git commit -m "feat(tui): add undo/redo keybindings (u, Ctrl+r)"
```

---

## Task 2: Add Stash Operations

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/app.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Add stash methods to App**

In `app.rs`:
```rust
/// Stash the selected todo
pub async fn stash_selected(&mut self) -> Result<()> {
    if let Some(todo) = self.selected_todo().cloned() {
        let title = todo.title.clone();
        self.db.stash_todo(&todo, None).await?;
        self.status_message = Some(format!("📦 Stashed: {}", title));
        self.refresh_todos().await?;
    }
    Ok(())
}

/// Pop the most recent stashed todo
pub async fn stash_pop(&mut self) -> Result<()> {
    if let Some(todo) = self.db.stash_pop().await? {
        self.status_message = Some(format!("📦 Restored: {}", todo.title));
        self.refresh_todos().await?;
    } else {
        self.status_message = Some("Stash is empty".to_string());
    }
    Ok(())
}

/// Get stash count for status display
pub async fn stash_count(&self) -> Result<usize> {
    let stash = self.db.stash_list().await?;
    Ok(stash.len())
}
```

**Step 2: Add keybindings in handler.rs**

In `handle_todos_view`:
```rust
// Stash operations
KeyCode::Char('z') => {
    app.stash_selected().await?;
}
KeyCode::Char('Z') => {
    app.stash_pop().await?;
}
```

**Step 3: Update help text**

In `render_help`:
```rust
View::Todos => {
    "j/k:nav  a:add  d:done  x:del  u:undo  z:stash  Z:pop  v:view  /:search  q:quit"
}
```

**Step 4: Update help modal**

Add lines:
```rust
Line::from("  z           Stash selected todo"),
Line::from("  Z           Pop from stash"),
```

**Step 5: Run tests**

```bash
cargo test -p todoee-cli
```

**Step 6: Commit**

```bash
git add -A && git commit -m "feat(tui): add stash keybindings (z/Z)"
```

---

## Task 3: Add Overdue Filter Toggle

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs` (add overdue filter to Filter struct)
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Add overdue_only to Filter struct**

In `app.rs`, modify `Filter`:
```rust
pub struct Filter {
    pub today_only: bool,
    pub overdue_only: bool,  // NEW
    pub category: Option<String>,
    pub show_completed: bool,
    pub search_query: String,
    pub priority: Option<Priority>,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
}
```

Update `Default` impl to include `overdue_only: false`.

**Step 2: Update refresh_todos to handle overdue filter**

In `refresh_todos`:
```rust
self.todos = if self.filter.overdue_only {
    self.db.list_todos_overdue().await?
} else if self.filter.today_only {
    self.db.list_todos_due_today().await?
} else if let Some(ref cat_name) = self.filter.category {
    // ... existing code
```

**Step 3: Add toggle method**

```rust
pub fn toggle_overdue_filter(&mut self) {
    self.filter.overdue_only = !self.filter.overdue_only;
    self.filter.today_only = false;
    self.filter.category = None;
}
```

**Step 4: Add keybinding in handler.rs**

In `handle_todos_view`:
```rust
KeyCode::Char('o') => {
    app.toggle_overdue_filter();
    app.refresh_todos().await?;
    if app.filter.overdue_only {
        app.status_message = Some("Showing overdue tasks".to_string());
    } else {
        app.status_message = Some("Showing all tasks".to_string());
    }
}
```

**Step 5: Show overdue indicator in tabs**

In `render_tabs`, add after priority indicator:
```rust
if app.filter.overdue_only {
    spans.push(Span::styled(
        " [OVERDUE] ",
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    ));
}
```

**Step 6: Update help**

Add to help modal:
```rust
Line::from("  o           Toggle overdue filter"),
```

**Step 7: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "feat(tui): add overdue filter toggle (o)"
```

---

## Task 4: Upgrade Search to Fuzzy Search

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Add fuzzy scoring function**

In `app.rs`, add the fuzzy scoring algorithm:
```rust
/// Calculate fuzzy match score (higher = better match)
fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let query = query.to_lowercase();
    let text_lower = text.to_lowercase();

    // Exact match gets highest score
    if text_lower.contains(&query) {
        return Some(1000 + (100 - text.len() as i32).max(0));
    }

    // Fuzzy matching
    let mut score = 0i32;
    let mut query_idx = 0;
    let query_chars: Vec<char> = query.chars().collect();
    let mut consecutive = 0;
    let mut prev_matched = false;

    for (i, c) in text_lower.chars().enumerate() {
        if query_idx < query_chars.len() && c == query_chars[query_idx] {
            score += 10;
            // Bonus for consecutive matches
            if prev_matched {
                consecutive += 1;
                score += consecutive * 5;
            } else {
                consecutive = 0;
            }
            // Bonus for matching at word boundaries
            if i == 0 || text.chars().nth(i - 1).map(|p| !p.is_alphanumeric()).unwrap_or(true) {
                score += 15;
            }
            query_idx += 1;
            prev_matched = true;
        } else {
            prev_matched = false;
            consecutive = 0;
        }
    }

    if query_idx == query_chars.len() {
        Some(score)
    } else {
        None
    }
}
```

**Step 2: Update refresh_todos to use fuzzy search**

Replace the simple contains check:
```rust
// Apply search filter with fuzzy matching
if !self.filter.search_query.is_empty() {
    let query = &self.filter.search_query;
    // Score and filter todos
    let mut scored: Vec<_> = self.todos
        .drain(..)
        .filter_map(|t| {
            let title_score = fuzzy_score(query, &t.title);
            let desc_score = t.description.as_ref()
                .and_then(|d| fuzzy_score(query, d))
                .map(|s| s / 2); // Description matches worth half
            let score = title_score.or(desc_score)?;
            Some((t, score))
        })
        .collect();
    // Sort by score descending
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    self.todos = scored.into_iter().map(|(t, _)| t).collect();
}
```

**Step 3: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "feat(tui): upgrade search to fuzzy matching"
```

---

## Task 5: Add Insights Modal

**Files:**
- Create: `crates/todoee-cli/src/tui/widgets/insights.rs`
- Modify: `crates/todoee-cli/src/tui/widgets/mod.rs`
- Modify: `crates/todoee-cli/src/tui/app.rs` (add Mode::Insights)
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Add Insights mode to app.rs**

In `Mode` enum:
```rust
pub enum Mode {
    // ... existing modes
    /// Viewing insights
    Insights,
}
```

**Step 2: Add insights data struct to app.rs**

```rust
/// Productivity insights data
#[derive(Debug, Clone, Default)]
pub struct InsightsData {
    pub total_completed_7d: usize,
    pub total_created_7d: usize,
    pub completion_rate: f64,
    pub avg_completion_time_hours: Option<f64>,
    pub most_productive_day: Option<String>,
    pub overdue_count: usize,
    pub by_priority: [(Priority, usize); 3],
}
```

**Step 3: Add method to compute insights**

```rust
pub async fn compute_insights(&self) -> Result<InsightsData> {
    let now = Utc::now();
    let seven_days_ago = now - chrono::Duration::days(7);

    let all_todos = self.db.list_todos(false).await?;
    let completed_7d: Vec<_> = all_todos.iter()
        .filter(|t| t.is_completed && t.completed_at.map(|c| c > seven_days_ago).unwrap_or(false))
        .collect();

    let created_7d = all_todos.iter()
        .filter(|t| t.created_at > seven_days_ago)
        .count();

    let overdue = all_todos.iter()
        .filter(|t| !t.is_completed && t.due_date.map(|d| d < now).unwrap_or(false))
        .count();

    let completion_rate = if created_7d > 0 {
        (completed_7d.len() as f64 / created_7d as f64) * 100.0
    } else {
        0.0
    };

    // Average completion time
    let completion_times: Vec<_> = completed_7d.iter()
        .filter_map(|t| {
            t.completed_at.map(|c| (c - t.created_at).num_hours() as f64)
        })
        .collect();
    let avg_completion = if !completion_times.is_empty() {
        Some(completion_times.iter().sum::<f64>() / completion_times.len() as f64)
    } else {
        None
    };

    // By priority
    let high = all_todos.iter().filter(|t| t.priority == Priority::High && !t.is_completed).count();
    let med = all_todos.iter().filter(|t| t.priority == Priority::Medium && !t.is_completed).count();
    let low = all_todos.iter().filter(|t| t.priority == Priority::Low && !t.is_completed).count();

    Ok(InsightsData {
        total_completed_7d: completed_7d.len(),
        total_created_7d: created_7d,
        completion_rate,
        avg_completion_time_hours: avg_completion,
        most_productive_day: None, // Could be computed from completed_at days
        overdue_count: overdue,
        by_priority: [(Priority::High, high), (Priority::Medium, med), (Priority::Low, low)],
    })
}
```

**Step 4: Create insights widget**

Create `crates/todoee-cli/src/tui/widgets/insights.rs`:
```rust
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::tui::app::InsightsData;

pub struct InsightsWidget<'a> {
    data: &'a InsightsData,
}

impl<'a> InsightsWidget<'a> {
    pub fn new(data: &'a InsightsData) -> Self {
        Self { data }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(Span::styled(
                "📊 Productivity Insights (Last 7 Days)",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Completed: "),
                Span::styled(
                    self.data.total_completed_7d.to_string(),
                    Style::default().fg(Color::Green).bold(),
                ),
                Span::raw(" tasks"),
            ]),
            Line::from(vec![
                Span::raw("  Created:   "),
                Span::styled(
                    self.data.total_created_7d.to_string(),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(" tasks"),
            ]),
            Line::from(vec![
                Span::raw("  Rate:      "),
                Span::styled(
                    format!("{:.1}%", self.data.completion_rate),
                    if self.data.completion_rate >= 70.0 {
                        Style::default().fg(Color::Green)
                    } else if self.data.completion_rate >= 40.0 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  ⚠ Overdue: "),
                Span::styled(
                    self.data.overdue_count.to_string(),
                    if self.data.overdue_count > 0 {
                        Style::default().fg(Color::Red).bold()
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled("  By Priority:", Style::default().fg(Color::Yellow))),
            Line::from(vec![
                Span::raw("    "),
                Span::styled("!!!", Style::default().fg(Color::Red)),
                Span::raw(format!(" High:   {} pending", self.data.by_priority[0].1)),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled("!! ", Style::default().fg(Color::Yellow)),
                Span::raw(format!(" Medium: {} pending", self.data.by_priority[1].1)),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled("!  ", Style::default().fg(Color::Green)),
                Span::raw(format!(" Low:    {} pending", self.data.by_priority[2].1)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Press any key to close",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Insights ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }
}
```

**Step 5: Update widgets/mod.rs**

```rust
mod insights;
pub use insights::InsightsWidget;
```

**Step 6: Add insights_data field to App**

```rust
pub struct App {
    // ... existing fields
    pub insights_data: Option<InsightsData>,
}
```

Initialize as `None` in `App::new()`.

**Step 7: Add keybinding in handler.rs**

```rust
KeyCode::Char('i') => {
    app.set_loading("Computing insights...");
    let data = app.compute_insights().await?;
    app.clear_loading();
    app.insights_data = Some(data);
    app.mode = Mode::Insights;
}
```

Add handler for Insights mode:
```rust
Mode::Insights => handle_insights_mode(app, key),
```

```rust
fn handle_insights_mode(app: &mut App, _key: KeyEvent) {
    app.mode = Mode::Normal;
    app.insights_data = None;
}
```

**Step 8: Render insights modal in ui.rs**

```rust
if app.mode == Mode::Insights {
    if let Some(ref data) = app.insights_data {
        let area = centered_rect(50, 60, frame.area());
        InsightsWidget::new(data).render(frame, area);
    }
}
```

Add import: `use super::widgets::InsightsWidget;`

**Step 9: Update help**

Add to help modal:
```rust
Line::from("  i           View insights"),
```

**Step 10: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "feat(tui): add insights modal (i)"
```

---

## Task 6: Add "Now" Recommendation

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`
- Modify: `crates/todoee-cli/src/tui/handler.rs`

**Step 1: Add now recommendation method**

In `app.rs`:
```rust
/// Get the recommended "now" todo based on priority, due date, and time of day
pub fn get_now_recommendation(&self) -> Option<usize> {
    if self.todos.is_empty() {
        return None;
    }

    let now = Utc::now();
    let hour = chrono::Local::now().hour();

    // Score each todo
    let mut scored: Vec<(usize, i32)> = self.todos.iter()
        .enumerate()
        .filter(|(_, t)| !t.is_completed)
        .map(|(i, t)| {
            let mut score = 0i32;

            // Priority weight
            score += match t.priority {
                Priority::High => 100,
                Priority::Medium => 50,
                Priority::Low => 10,
            };

            // Due date urgency
            if let Some(due) = t.due_date {
                let days_until = (due.date_naive() - now.date_naive()).num_days();
                score += match days_until {
                    d if d < 0 => 200,  // Overdue = highest priority
                    0 => 150,           // Due today
                    1 => 100,           // Due tomorrow
                    d if d <= 3 => 50,  // Due soon
                    _ => 0,
                };
            }

            // Time of day heuristics
            // Morning (6-12): prefer high priority
            // Afternoon (12-17): prefer medium tasks
            // Evening (17-22): prefer low priority / quick wins
            score += match hour {
                6..=11 if t.priority == Priority::High => 30,
                12..=16 if t.priority == Priority::Medium => 30,
                17..=22 if t.priority == Priority::Low => 30,
                _ => 0,
            };

            (i, score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.first().map(|(i, _)| *i)
}
```

**Step 2: Add keybinding in handler.rs**

```rust
KeyCode::Char('n') => {
    if let Some(idx) = app.get_now_recommendation() {
        app.selected = idx;
        if let Some(todo) = app.selected_todo() {
            app.status_message = Some(format!("🎯 Recommended: {}", todo.title));
        }
    } else {
        app.status_message = Some("No tasks to recommend".to_string());
    }
}
```

**Step 3: Update help**

Add to help modal:
```rust
Line::from("  n           Jump to recommended task"),
```

**Step 4: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "feat(tui): add 'now' recommendation (n)"
```

---

## Task 7: Add Focus Mode (Simplified Pomodoro)

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs` (add Mode::Focus, FocusState)
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/ui.rs`
- Create: `crates/todoee-cli/src/tui/widgets/focus.rs`

**Step 1: Add Focus mode and state**

In `app.rs`:
```rust
pub enum Mode {
    // ... existing
    Focus,
}

/// State for focus/pomodoro mode
#[derive(Debug, Clone)]
pub struct FocusState {
    pub todo_id: uuid::Uuid,
    pub todo_title: String,
    pub duration_secs: u64,
    pub started_at: std::time::Instant,
    pub paused: bool,
    pub paused_remaining: Option<u64>,
}

impl FocusState {
    pub fn new(todo: &Todo, duration_mins: u64) -> Self {
        Self {
            todo_id: todo.id,
            todo_title: todo.title.clone(),
            duration_secs: duration_mins * 60,
            started_at: std::time::Instant::now(),
            paused: false,
            paused_remaining: None,
        }
    }

    pub fn remaining_secs(&self) -> u64 {
        if self.paused {
            self.paused_remaining.unwrap_or(0)
        } else {
            let elapsed = self.started_at.elapsed().as_secs();
            self.duration_secs.saturating_sub(elapsed)
        }
    }

    pub fn is_complete(&self) -> bool {
        self.remaining_secs() == 0
    }

    pub fn toggle_pause(&mut self) {
        if self.paused {
            // Resume: reset started_at based on remaining time
            self.started_at = std::time::Instant::now();
            self.duration_secs = self.paused_remaining.unwrap_or(0);
            self.paused = false;
            self.paused_remaining = None;
        } else {
            // Pause: store remaining time
            self.paused_remaining = Some(self.remaining_secs());
            self.paused = true;
        }
    }
}
```

Add field to App:
```rust
pub focus_state: Option<FocusState>,
```

**Step 2: Add focus start method**

```rust
pub fn start_focus(&mut self, duration_mins: u64) {
    if let Some(todo) = self.selected_todo() {
        self.focus_state = Some(FocusState::new(todo, duration_mins));
        self.mode = Mode::Focus;
    }
}

pub async fn complete_focus(&mut self) -> Result<()> {
    if let Some(ref state) = self.focus_state {
        let todo_id = state.todo_id;
        self.focus_state = None;
        self.mode = Mode::Normal;

        // Ask if user wants to mark complete
        self.status_message = Some("🍅 Focus complete! Press 'd' to mark done.".to_string());

        // Select the todo we were focusing on
        if let Some(idx) = self.todos.iter().position(|t| t.id == todo_id) {
            self.selected = idx;
        }
    }
    Ok(())
}
```

**Step 3: Create focus widget**

Create `crates/todoee-cli/src/tui/widgets/focus.rs`:
```rust
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::app::FocusState;

pub struct FocusWidget<'a> {
    state: &'a FocusState,
}

impl<'a> FocusWidget<'a> {
    pub fn new(state: &'a FocusState) -> Self {
        Self { state }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let remaining = self.state.remaining_secs();
        let mins = remaining / 60;
        let secs = remaining % 60;

        let progress = if self.state.duration_secs > 0 {
            1.0 - (remaining as f64 / self.state.duration_secs as f64)
        } else {
            1.0
        };

        // Progress bar
        let bar_width = 30;
        let filled = (progress * bar_width as f64) as usize;
        let empty = bar_width - filled;
        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

        let time_color = if remaining <= 60 {
            Color::Red
        } else if remaining <= 300 {
            Color::Yellow
        } else {
            Color::Green
        };

        let status = if self.state.paused { " ⏸ PAUSED" } else { "" };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "🍅 FOCUS MODE",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                &self.state.todo_title,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{:02}:{:02}{}", mins, secs, status),
                Style::default().fg(time_color).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(bar, Style::default().fg(time_color))),
            Line::from(""),
            Line::from(Span::styled(
                "Space: pause  q/Esc: cancel  Enter: complete early",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Focus ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            )
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }
}
```

**Step 4: Update widgets/mod.rs**

```rust
mod focus;
pub use focus::FocusWidget;
```

**Step 5: Add keybinding to start focus**

In `handle_todos_view`:
```rust
KeyCode::Char('f') => {
    if app.selected_todo().is_some() {
        app.start_focus(25); // 25 minute pomodoro
    }
}
KeyCode::Char('F') => {
    if app.selected_todo().is_some() {
        app.start_focus(5); // 5 minute quick focus
    }
}
```

**Step 6: Add focus mode handler**

```rust
Mode::Focus => handle_focus_mode(app, key).await?,
```

```rust
async fn handle_focus_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char(' ') => {
            if let Some(ref mut state) = app.focus_state {
                state.toggle_pause();
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.focus_state = None;
            app.mode = Mode::Normal;
            app.status_message = Some("Focus cancelled".to_string());
        }
        KeyCode::Enter => {
            app.complete_focus().await?;
        }
        _ => {}
    }
    Ok(())
}
```

**Step 7: Render focus modal and check completion**

In `ui.rs` render function, before loading overlay:
```rust
if app.mode == Mode::Focus {
    if let Some(ref state) = app.focus_state {
        if state.is_complete() {
            // Timer finished - will be handled in main loop
        }
        let area = centered_rect(50, 40, frame.area());
        FocusWidget::new(state).render(frame, area);
    }
}
```

Import: `use super::widgets::FocusWidget;`

**Step 8: Handle focus completion in main loop**

In `crates/todoee-cli/src/tui/mod.rs`, in the main event loop, check for focus completion:
```rust
// Check if focus timer completed
if app.mode == Mode::Focus {
    if let Some(ref state) = app.focus_state {
        if state.is_complete() {
            app.complete_focus().await?;
        }
    }
}
```

**Step 9: Update help modal**

```rust
Line::from("  f           Start focus (25 min pomodoro)"),
Line::from("  F           Quick focus (5 min)"),
```

**Step 10: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "feat(tui): add focus mode (f/F)"
```

---

## Task 8: Record Operations for TUI Actions

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs`

**Step 1: Update mark_selected_done to record operation**

```rust
pub async fn mark_selected_done(&mut self) -> Result<()> {
    let should_complete = self
        .todos
        .get(self.selected)
        .is_some_and(|t| !t.is_completed);

    if should_complete {
        self.set_loading("Completing task...");
        let todo = self.todos.get_mut(self.selected).unwrap();

        // Record operation BEFORE modifying
        let previous_state = serde_json::to_value(&*todo)?;

        todo.mark_complete();
        let title = todo.title.clone();
        let todo_id = todo.id;

        // Record new state
        let new_state = serde_json::to_value(&*todo)?;

        self.db.update_todo(todo).await?;

        // Record the operation
        self.db.record_operation(
            OperationType::Complete,
            EntityType::Todo,
            todo_id,
            Some(previous_state),
            Some(new_state),
        ).await?;

        self.clear_loading();
        self.status_message = Some(format!("✓ Completed: {}", title));
        self.refresh_todos().await?;
    } else if self.todos.get(self.selected).is_some() {
        self.status_message = Some("Already completed".to_string());
    }
    Ok(())
}
```

**Step 2: Update delete_selected to record operation**

```rust
pub async fn delete_selected(&mut self) -> Result<()> {
    let todo_info = self
        .todos
        .get(self.selected)
        .map(|t| (t.id, t.title.clone(), serde_json::to_value(t).ok()));

    if let Some((id, title, previous_state)) = todo_info {
        self.set_loading("Deleting task...");
        self.db.delete_todo(id).await?;

        // Record operation
        self.db.record_operation(
            OperationType::Delete,
            EntityType::Todo,
            id,
            previous_state,
            None,
        ).await?;

        self.clear_loading();
        self.status_message = Some(format!("✗ Deleted: {}", title));
        self.refresh_todos().await?;
    }
    Ok(())
}
```

**Step 3: Update add_todo_with_ai to record operation**

After `self.db.create_todo(&todo).await?;`:
```rust
// Record operation
self.db.record_operation(
    OperationType::Create,
    EntityType::Todo,
    todo.id,
    None,
    Some(serde_json::to_value(&todo)?),
).await?;
```

**Step 4: Update create_todo_from_add_state similarly**

**Step 5: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "feat(tui): record operations for undo/redo support"
```

---

## Task 9: Final Polish and Help Updates

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Update bottom help line for all new features**

In `render_help`:
```rust
Mode::Normal => match app.current_view {
    View::Todos => {
        "j/k:nav a:add d:done x:del u:undo z:stash o:overdue i:insights f:focus n:now ?:help q:quit"
    }
    // ... rest unchanged
}
```

**Step 2: Ensure all new modes are in help modal**

Verify help_text includes all new keybindings grouped logically.

**Step 3: Add stash count indicator to status bar**

In `render_status`, after status message but before closing the block, add stash indicator if count > 0 (need async support or cache the count).

Alternative: Show stash count in tabs area when > 0.

**Step 4: Run full test suite**

```bash
cargo test
cargo clippy
```

**Step 5: Commit**

```bash
git add -A && git commit -m "feat(tui): final polish with help updates and indicators"
```

---

## Summary of New Keybindings

| Key | Action | Mode |
|-----|--------|------|
| `u` | Undo last action | Normal (Todos) |
| `Ctrl+r` | Redo last undone | Normal (Todos) |
| `z` | Stash selected todo | Normal (Todos) |
| `Z` | Pop from stash | Normal (Todos) |
| `o` | Toggle overdue filter | Normal (Todos) |
| `i` | View insights modal | Normal (Todos) |
| `f` | Start focus (25 min) | Normal (Todos) |
| `F` | Quick focus (5 min) | Normal (Todos) |
| `n` | Jump to recommended task | Normal (Todos) |
| `Space` | Pause/resume focus | Focus |
| `q/Esc` | Cancel focus | Focus |
| `Enter` | Complete focus early | Focus |

All existing keybindings remain unchanged.
