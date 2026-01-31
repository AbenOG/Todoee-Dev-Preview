# Logic Safeguards Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add defensive safeguards to prevent data corruption, improve error handling, and ensure consistent state management across the todoee application.

**Architecture:** Add validation checks at critical points (before destructive operations, after state changes), handle edge cases gracefully with user-friendly messages, and ensure selection indices stay in bounds.

**Tech Stack:** Rust, SQLite (via SQLx), Ratatui TUI

---

## Summary of Safeguards

| Issue | Severity | Fix |
|-------|----------|-----|
| Category deletion orphans todos | HIGH | Clear category_id on todos before deleting category |
| Selection index out of bounds after delete | MEDIUM | Clamp selection after refresh |
| Focused todo deletion | HIGH | Cancel focus if todo deleted, prevent deletion during focus |
| Editing completed todos | MEDIUM | Block edits on completed todos |
| Stash duplicate collision | MEDIUM | Check if already stashed before stashing |
| Category filter with deleted category | MEDIUM | Clear filter when deleting filtered category |

---

### Task 1: Handle Category Deletion - Clear Orphaned Todos

**Files:**
- Modify: `crates/todoee-core/src/db/local.rs:565-574`
- Modify: `crates/todoee-cli/src/tui/app.rs:733-752`

**Step 1: Add method to clear category from todos in local.rs**

After line 574 in `local.rs`, add a new method:

```rust
/// Clear category_id for all todos that belong to a category.
/// Call this before deleting a category to prevent orphaned references.
pub async fn clear_category_from_todos(&self, category_id: Uuid) -> Result<u64> {
    let result = sqlx::query("UPDATE todos SET category_id = NULL WHERE category_id = ?")
        .bind(category_id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to clear category from todos")?;

    Ok(result.rows_affected())
}
```

**Step 2: Update delete_selected_category in app.rs**

Find `delete_selected_category` (around line 734) and add the clearing step:

```rust
pub async fn delete_selected_category(&mut self) -> Result<()> {
    if let Some(cat) = self.categories.get(self.category_selected) {
        let name = cat.name.clone();
        let id = cat.id;

        self.set_loading("Deleting category...");

        // Clear category from all todos first
        let affected = self.db.clear_category_from_todos(id).await?;

        // Clear filter if we're deleting the filtered category
        if self.filter.category.as_ref() == Some(&name) {
            self.filter.category = None;
        }

        self.db.delete_category(id).await?;
        self.clear_loading();

        let msg = if affected > 0 {
            format!("Deleted category '{}' ({} todos uncategorized)", name, affected)
        } else {
            format!("Deleted category: {}", name)
        };
        self.status_message = Some(msg);

        self.refresh_categories().await?;
        self.refresh_todos().await?; // Refresh todos to update their display

        if self.categories.is_empty() {
            self.category_selected = 0;
        } else if self.category_selected >= self.categories.len() {
            self.category_selected = self.categories.len() - 1;
        }
    }
    Ok(())
}
```

**Step 3: Build and verify**

Run: `cargo build --release 2>&1 | tail -5`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/todoee-core/src/db/local.rs crates/todoee-cli/src/tui/app.rs
git commit -m "fix(db): clear category from todos before deleting category

Prevents orphaned todo references by setting category_id to NULL
for all todos that belong to a category before deleting it.
Also clears the category filter if deleting the active filter category."
```

---

### Task 2: Fix Selection Index After Todo Deletion

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs:545-572`

**Step 1: Update delete_selected to clamp selection**

Find `delete_selected` function and add selection clamping after refresh:

```rust
pub async fn delete_selected(&mut self) -> Result<()> {
    // Extract necessary data before borrowing self mutably
    let todo_info = self
        .todos
        .get(self.selected)
        .map(|t| (t.id, t.title.clone(), serde_json::to_value(t).ok()));

    if let Some((id, title, previous_state)) = todo_info {
        self.set_loading("Deleting task...");
        self.db.delete_todo(id).await?;

        // Record operation for undo/redo
        let op = Operation::new(
            OperationType::Delete,
            EntityType::Todo,
            id,
            previous_state,
            None,
        );
        self.db.record_operation(&op).await?;

        self.clear_loading();
        self.status_message = Some(format!("✗ Deleted: {}", title));
        self.refresh_todos().await?;

        // Clamp selection to valid range
        if self.todos.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.todos.len() {
            self.selected = self.todos.len() - 1;
        }
    }
    Ok(())
}
```

**Step 2: Add helper method for clamping selection**

Add this helper method to the App impl block (can be placed after `delete_selected`):

```rust
/// Clamp the selection index to valid range after list changes
fn clamp_selection(&mut self) {
    if self.todos.is_empty() {
        self.selected = 0;
    } else if self.selected >= self.todos.len() {
        self.selected = self.todos.len() - 1;
    }
}
```

**Step 3: Use helper in mark_selected_done**

Find `mark_selected_done` and add clamping after refresh (around line 543):

```rust
// After refresh_todos().await?;
self.clamp_selection();
```

**Step 4: Build and verify**

Run: `cargo build --release 2>&1 | tail -5`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "fix(tui): clamp selection index after todo deletion

Prevents out-of-bounds selection when deleting the last todo
or when filtering changes the list size."
```

---

### Task 3: Prevent Focus Mode Issues

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs:545-572` (delete_selected)
- Modify: `crates/todoee-cli/src/tui/app.rs:1125-1145` (complete_focus)

**Step 1: Prevent deleting focused todo**

In `delete_selected`, add a check at the beginning:

```rust
pub async fn delete_selected(&mut self) -> Result<()> {
    // Prevent deleting a todo that's currently being focused on
    if let Some(ref focus) = self.focus_state {
        if let Some(todo) = self.todos.get(self.selected) {
            if todo.id == focus.todo_id {
                self.status_message = Some("Cannot delete: todo is in focus mode".to_string());
                return Ok(());
            }
        }
    }

    // ... rest of function
```

**Step 2: Handle focus completion when todo was deleted**

Find `complete_focus` (around line 1131) and update it:

```rust
pub fn complete_focus(&mut self) {
    if let Some(state) = self.focus_state.take() {
        // Check if the focused todo still exists
        let todo_exists = self.todos.iter().any(|t| t.id == state.todo_id);

        if todo_exists {
            self.status_message = Some("Focus complete! Press 'd' to mark done.".to_string());
            // Select the focused todo
            if let Some(idx) = self.todos.iter().position(|t| t.id == state.todo_id) {
                self.selected = idx;
            }
        } else {
            self.status_message = Some("Focus complete! (Todo was deleted)".to_string());
        }

        self.mode = Mode::Normal;
    }
}
```

**Step 3: Build and verify**

Run: `cargo build --release 2>&1 | tail -5`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "fix(tui): prevent deleting todo during focus mode

Blocks deletion of the currently focused todo to prevent
broken state. Also handles the edge case where focus completes
but the todo was somehow deleted."
```

---

### Task 4: Prevent Editing Completed Todos

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs:119-124`

**Step 1: Add check in edit handler**

Find the edit handler (KeyCode::Char('e')) around line 119 and add validation:

```rust
KeyCode::Char('e') => {
    if let Some(todo) = app.selected_todo() {
        if todo.is_completed {
            app.status_message = Some("Cannot edit completed todo (uncomplete first)".to_string());
        } else {
            app.edit_state = Some(EditState::from_todo(todo, &app.categories));
            app.mode = Mode::EditingFull;
        }
    }
}
```

**Step 2: Build and verify**

Run: `cargo build --release 2>&1 | tail -5`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/handler.rs
git commit -m "fix(tui): prevent editing completed todos

Completed todos should not be modified. Users must uncomplete
a todo first before editing it."
```

---

### Task 5: Prevent Stash Duplicates

**Files:**
- Modify: `crates/todoee-core/src/db/local.rs:755-775`

**Step 1: Add check for already stashed todo**

Update `stash_todo` to check if already stashed:

```rust
/// Stash a todo by ID. Returns the stashed todo.
///
/// # Errors
/// Returns error if todo doesn't exist or is already stashed.
pub async fn stash_todo(&self, todo_id: Uuid, message: Option<String>) -> Result<Todo> {
    // Check if already stashed
    let already_stashed: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM stash WHERE id = ?"
    )
        .bind(todo_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to check stash")?;

    if already_stashed.is_some() {
        anyhow::bail!("Todo is already stashed");
    }

    let todo = self
        .get_todo(todo_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Todo not found"))?;

    sqlx::query("INSERT INTO stash (id, todo_json, stashed_at, message) VALUES (?, ?, ?, ?)")
        .bind(todo_id.to_string())
        .bind(serde_json::to_string(&todo)?)
        .bind(Utc::now().to_rfc3339())
        .bind(message)
        .execute(&self.pool)
        .await
        .context("Failed to stash todo")?;

    self.delete_todo(todo_id).await?;
    Ok(todo)
}
```

**Step 2: Handle error gracefully in TUI**

Find `stash_selected` in app.rs and update error handling:

```rust
pub async fn stash_selected(&mut self) -> Result<()> {
    let todo_info = self.todos.get(self.selected).map(|t| (t.id, t.title.clone()));

    if let Some((id, title)) = todo_info {
        self.set_loading("Stashing...");
        match self.db.stash_todo(id, None).await {
            Ok(todo) => {
                // Record operation for undo
                let op = Operation::new(
                    OperationType::Stash,
                    EntityType::Todo,
                    id,
                    Some(serde_json::to_value(&todo)?),
                    None,
                );
                self.db.record_operation(&op).await?;
                self.clear_loading();
                self.status_message = Some(format!("Stashed: {}", title));
                self.refresh_todos().await?;
                self.clamp_selection();
            }
            Err(e) => {
                self.clear_loading();
                let msg = e.to_string();
                if msg.contains("already stashed") {
                    self.status_message = Some("Todo is already stashed".to_string());
                } else {
                    self.status_message = Some(format!("Stash failed: {}", msg));
                }
            }
        }
    } else {
        self.status_message = Some("No task selected".to_string());
    }
    Ok(())
}
```

**Step 3: Build and verify**

Run: `cargo build --release 2>&1 | tail -5`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add crates/todoee-core/src/db/local.rs crates/todoee-cli/src/tui/app.rs
git commit -m "fix(stash): prevent stashing an already stashed todo

Adds validation to check if a todo is already in the stash
before attempting to stash it again. Shows user-friendly
error message instead of database constraint violation."
```

---

### Task 6: Add Count of Affected Todos on Category Delete (CLI)

**Files:**
- Modify: `crates/todoee-cli/src/commands/delete.rs` (if category delete exists in CLI)

**Step 1: Check if CLI has category delete**

First check if the CLI supports category deletion. If not, skip this task.

Run: `grep -r "delete.*category\|category.*delete" crates/todoee-cli/src/commands/`

**Step 2: If exists, add affected count**

If category deletion exists in CLI, update it similar to TUI to show affected todos count.

**Step 3: Commit (if changes made)**

```bash
git add crates/todoee-cli/src/commands/
git commit -m "fix(cli): show affected todos count on category delete"
```

---

## Verification Checklist

After all tasks complete, verify:

1. **Category deletion**: Delete a category with todos, verify todos become uncategorized (not deleted)
2. **Selection after delete**: Delete the last todo in list, verify selection moves to new last item
3. **Focus mode protection**: Try to delete focused todo, verify it's blocked
4. **Completed todo edit**: Try to edit completed todo, verify it's blocked
5. **Double stash prevention**: Try to stash same todo twice, verify error message
6. All tests pass: `cargo test --workspace`
