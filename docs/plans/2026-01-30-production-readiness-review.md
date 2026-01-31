# Production Readiness Review Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix bugs, inconsistencies, and quality issues to prepare the application for production release.

**Architecture:** Address issues in priority order - critical bugs first, then consistency improvements, then UX polish.

**Tech Stack:** Rust, Ratatui, SQLx, Tokio

---

## Issue Summary

| Category | Critical | High | Medium | Low |
|----------|----------|------|--------|-----|
| Unsafe unwrap/panics | 2 | - | - | - |
| Missing operation recording | 2 | - | - | - |
| State consistency bugs | - | 2 | 2 | - |
| Error handling gaps | - | 2 | 2 | - |
| UX inconsistencies | - | - | 4 | 2 |
| **Total** | **4** | **4** | **8** | **2** |

---

## Task 1: Fix Critical Unsafe Unwrap in mark_selected_done

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs:502-535`

**Problem:** Line 506 uses `.unwrap()` on `self.todos.get_mut(self.selected)` after a separate existence check. Race condition possible.

**Step 1: Replace unwrap with defensive pattern**

Find:
```rust
if should_complete {
    self.set_loading("Completing task...");
    let todo = self.todos.get_mut(self.selected).unwrap();
```

Replace with:
```rust
if should_complete {
    self.set_loading("Completing task...");
    let Some(todo) = self.todos.get_mut(self.selected) else {
        self.clear_loading();
        self.status_message = Some("Task no longer available".to_string());
        return Ok(());
    };
```

**Step 2: Run tests**

```bash
cargo test -p todoee-cli
```

**Step 3: Commit**

```bash
git add -A && git commit -m "fix(tui): replace unsafe unwrap in mark_selected_done"
```

---

## Task 2: Fix Event Handler Panics

**Files:**
- Modify: `crates/todoee-cli/src/tui/event.rs:40-50`

**Problem:** Lines 43-44 use `.expect()` which panics on I/O failures.

**Step 1: Replace expects with proper error handling**

Find:
```rust
if event::poll(timeout).expect("failed to poll events") {
    match event::read().expect("failed to read event") {
```

Replace with:
```rust
match event::poll(timeout) {
    Ok(true) => match event::read() {
        Ok(event) => {
```

Add error handling for poll failure and read failure - send error event or continue loop.

**Step 2: Run tests**

```bash
cargo test -p todoee-cli
```

**Step 3: Commit**

```bash
git add -A && git commit -m "fix(tui): handle event poll/read errors gracefully"
```

---

## Task 3: Add Missing Operation Recording for TUI Stash

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs:947-966`

**Problem:** TUI stash operations don't record Operations, breaking undo/redo.

**Step 1: Update stash_selected to record operation**

```rust
pub async fn stash_selected(&mut self) -> Result<()> {
    if let Some(todo) = self.selected_todo().cloned() {
        let title = todo.title.clone();
        let todo_id = todo.id;
        let previous_state = serde_json::to_value(&todo).ok();

        self.db.stash_todo(todo_id, None).await?;

        // Record operation for undo/redo
        let op = Operation::new(
            OperationType::Stash,
            EntityType::Todo,
            todo_id,
            previous_state,
            None,
        );
        self.db.record_operation(&op).await?;

        self.status_message = Some(format!("Stashed: {}", title));
        self.refresh_todos().await?;
    }
    Ok(())
}
```

**Step 2: Update stash_pop to record operation**

```rust
pub async fn stash_pop(&mut self) -> Result<()> {
    if let Some(todo) = self.db.stash_pop().await? {
        let new_state = serde_json::to_value(&todo).ok();

        // Record operation for undo/redo
        let op = Operation::new(
            OperationType::Unstash,
            EntityType::Todo,
            todo.id,
            None,
            new_state,
        );
        self.db.record_operation(&op).await?;

        self.status_message = Some(format!("Restored: {}", todo.title));
        self.refresh_todos().await?;
    } else {
        self.status_message = Some("Stash is empty".to_string());
    }
    Ok(())
}
```

**Step 3: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "fix(tui): record operations for stash undo/redo support"
```

---

## Task 4: Add Missing Operation Recording for TUI Edit

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs:477-506`

**Problem:** Full edit mode doesn't record operations for undo/redo.

**Step 1: Capture previous state before edit**

In `handle_editing_full_mode`, before the save logic:
```rust
KeyCode::Enter => {
    let todo_id = state.todo_id;
    let category_name = state.category_name.clone();

    // Find todo and capture previous state
    let Some(todo) = app.todos.iter_mut().find(|t| t.id == todo_id) else {
        app.edit_state = None;
        app.mode = Mode::Normal;
        app.status_message = Some("Todo no longer exists".to_string());
        return Ok(());
    };

    let previous_state = serde_json::to_value(&*todo).ok();

    // Apply updates
    todo.title = state.title.clone();
    // ... rest of field updates ...

    let new_state = serde_json::to_value(&*todo).ok();

    app.db.update_todo(todo).await?;

    // Record operation
    let op = Operation::new(
        OperationType::Update,
        EntityType::Todo,
        todo_id,
        previous_state,
        new_state,
    );
    app.db.record_operation(&op).await?;

    app.status_message = Some(format!("Updated: {}", todo.title));
    // ...
}
```

**Step 2: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "fix(tui): record operations for edit undo/redo support"
```

---

## Task 5: Fix Empty List Index Logic

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs:464-466`
- Modify: `crates/todoee-cli/src/tui/app.rs:739-741` (categories)

**Problem:** Index validation logic is incorrect - doesn't reset when list becomes empty.

**Step 1: Fix todo index validation**

Find:
```rust
if self.selected >= self.todos.len() && !self.todos.is_empty() {
    self.selected = self.todos.len() - 1;
}
```

Replace with:
```rust
if self.todos.is_empty() {
    self.selected = 0;
} else if self.selected >= self.todos.len() {
    self.selected = self.todos.len() - 1;
}
```

**Step 2: Fix category index validation**

Apply same fix to category_selected validation.

**Step 3: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "fix(tui): correct index validation for empty lists"
```

---

## Task 6: Fix Silent Operation Failures

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs` (stash_selected)
- Modify: `crates/todoee-cli/src/tui/handler.rs` (focus mode entry)

**Problem:** Several operations fail silently without user feedback.

**Step 1: Add feedback for stash on empty selection**

```rust
pub async fn stash_selected(&mut self) -> Result<()> {
    let Some(todo) = self.selected_todo().cloned() else {
        self.status_message = Some("No task selected".to_string());
        return Ok(());
    };
    // ... rest of implementation
}
```

**Step 2: Add feedback for focus on empty selection**

In handler.rs:
```rust
KeyCode::Char('f') => {
    if app.selected_todo().is_some() {
        app.start_focus(25);
    } else {
        app.status_message = Some("No task selected".to_string());
    }
}
KeyCode::Char('F') => {
    if app.selected_todo().is_some() {
        app.start_focus(5);
    } else {
        app.status_message = Some("No task selected".to_string());
    }
}
```

**Step 3: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "fix(tui): add feedback for operations on empty selection"
```

---

## Task 7: Standardize Status Message Format

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs` (multiple locations)
- Modify: `crates/todoee-cli/src/tui/handler.rs` (multiple locations)

**Problem:** Inconsistent status message formats (some have emojis, different verbs, etc.)

**Standard Format:**
- Success actions: `"✓ {Verb}: {title}"` (Completed, Deleted, Added, Updated, Stashed, Restored)
- Info messages: `"Showing {filter}"` or `"Sorted by: {field}"`
- Empty/Not found: `"No {thing} to {action}"`
- Errors: `"Cannot {action}: {reason}"`

**Step 1: Update app.rs status messages**

Examples:
- `"Completed: {}"` → `"✓ Completed: {}"`
- `"Deleted: {}"` → `"✗ Deleted: {}"`
- `"Stashed: {}"` → `"✓ Stashed: {}"`
- `"Restored: {}"` → `"✓ Restored: {}"`
- `"↶ Undone: {}"` → keep (unique symbol is appropriate)
- `"↷ Redone: {}"` → keep

**Step 2: Update handler.rs status messages**

Ensure consistency in filter messages.

**Step 3: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "fix(tui): standardize status message format"
```

---

## Task 8: Fix CLI Batch Command Error Codes

**Files:**
- Modify: `crates/todoee-cli/src/commands/batch.rs:98-101`
- Modify: `crates/todoee-cli/src/commands/batch.rs:64,91,132`

**Problem:** Invalid inputs return success exit code.

**Step 1: Fix priority validation**

Find:
```rust
_ => {
    println!("Priority must be 1, 2, or 3");
    return Ok(());
}
```

Replace with:
```rust
_ => {
    anyhow::bail!("Priority must be 1, 2, or 3");
}
```

**Step 2: Add empty IDs validation**

At start of each batch subcommand:
```rust
if ids.is_empty() {
    anyhow::bail!("No IDs provided. Usage: todoee batch done <id1> <id2> ...");
}
```

**Step 3: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "fix(cli): return proper error codes for batch command failures"
```

---

## Task 9: Fix CLI Edit Empty Title Validation

**Files:**
- Modify: `crates/todoee-cli/src/commands/edit.rs:14-20`

**Problem:** Edit command allows setting empty title.

**Step 1: Add empty title validation**

After existing validation:
```rust
if let Some(ref t) = title {
    if t.trim().is_empty() {
        anyhow::bail!("Title cannot be empty");
    }
}
```

**Step 2: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "fix(cli): validate empty title in edit command"
```

---

## Task 10: Fix CLI Operation Recording Consistency

**Files:**
- Modify: `crates/todoee-cli/src/commands/done.rs:48-58`
- Modify: `crates/todoee-cli/src/commands/batch.rs:50-56`

**Problem:** CLI done commands record operations with `new_state: None`, inconsistent with TUI.

**Step 1: Update done.rs to capture new_state**

```rust
// Before
let op = Operation::new(
    if todo.is_completed { OperationType::Complete } else { OperationType::Uncomplete },
    EntityType::Todo,
    todo.id,
    Some(prev_state),
    None,  // Missing new_state
);

// After
let new_state = serde_json::to_value(&todo)?;
let op = Operation::new(
    if todo.is_completed { OperationType::Complete } else { OperationType::Uncomplete },
    EntityType::Todo,
    todo.id,
    Some(prev_state),
    Some(new_state),
);
```

**Step 2: Update batch.rs similarly**

**Step 3: Run tests and commit**

```bash
cargo test -p todoee-cli
git add -A && git commit -m "fix(cli): record new_state in done operations for undo consistency"
```

---

## Task 11: Final Verification

**Files:** None (verification only)

**Step 1: Run full test suite**

```bash
cargo test
```

**Step 2: Run clippy**

```bash
cargo clippy -- -D warnings
```

**Step 3: Manual testing checklist**

Test each flow in TUI:
- [ ] Add todo → undo → redo
- [ ] Delete todo → undo → redo
- [ ] Complete todo → undo → redo
- [ ] Edit todo → undo → redo
- [ ] Stash todo → undo → redo
- [ ] Focus mode start → pause → resume → complete
- [ ] Empty list navigation
- [ ] Filter toggles (today, overdue, category, priority)
- [ ] Insights modal
- [ ] Now recommendation
- [ ] Fuzzy search

Test CLI commands:
- [ ] `todoee batch done` with no IDs (should error)
- [ ] `todoee batch priority 5 abc` (should error)
- [ ] `todoee edit abc --title ""` (should error)

**Step 4: Commit if any fixes needed**

```bash
git add -A && git commit -m "fix: address issues found in final verification"
```

---

## Summary of Fixes

| Task | Type | Impact |
|------|------|--------|
| 1 | Critical bug | Prevent panic |
| 2 | Critical bug | Prevent crash |
| 3 | Critical bug | Enable undo/redo |
| 4 | Critical bug | Enable undo/redo |
| 5 | High bug | Correct list behavior |
| 6 | High UX | User feedback |
| 7 | Medium UX | Consistency |
| 8 | High bug | Correct exit codes |
| 9 | Medium bug | Validation |
| 10 | Medium consistency | Undo/redo |
| 11 | Verification | Quality assurance |
