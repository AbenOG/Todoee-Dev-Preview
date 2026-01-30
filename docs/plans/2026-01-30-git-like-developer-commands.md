# Git-Like Developer Commands Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add powerful, developer-oriented CLI commands inspired by git's UX patterns, enabling undo, history, batch operations, and smart querying.

**Architecture:** Extend the existing command system with new subcommands, add an operation history table to SQLite for undo/redo support, and implement smart filtering for time-based views.

**Tech Stack:** Rust, Clap 4 (subcommands), SQLx (history table), chrono (time calculations)

---

## Feature Overview

### Proposed Commands

| Command | Git Analog | Description |
|---------|------------|-------------|
| `todoee undo` | `git checkout --` | Reverse the last operation |
| `todoee redo` | `git reflog` + checkout | Re-apply an undone operation |
| `todoee log` | `git log` | View operation history |
| `todoee head -N` | `git log -N` | Show last N todos (by creation) |
| `todoee tail -N` | - | Show oldest N todos |
| `todoee upcoming -N` | - | Show next N todos by due date |
| `todoee overdue` | - | Show all overdue todos |
| `todoee stash` | `git stash` | Temporarily hide todos |
| `todoee stash pop` | `git stash pop` | Restore stashed todos |
| `todoee batch done` | - | Mark multiple todos done at once |
| `todoee batch delete` | - | Delete multiple todos at once |
| `todoee search` | `git grep` | Full-text search with regex support |
| `todoee show <id>` | `git show` | Display full todo details |
| `todoee diff` | `git diff` | Show what changed recently |
| `todoee reset` | `git reset` | Bulk reset completed todos to pending |
| `todoee archive` | - | Move completed todos to archive |
| `todoee gc` | `git gc` | Clean up old completed/deleted todos |

### TUI Enhancements

| Key | Action |
|-----|--------|
| `u` | Undo last action |
| `Ctrl+r` | Redo |
| `h` | Show head (recent) todos |
| `o` | Show overdue todos |
| `z` | Stash selected todo |
| `Z` | Stash pop |

---

## Task 1: Add Operation History Table

**Files:**
- Modify: `crates/todoee-core/src/db/local.rs`
- Modify: `crates/todoee-core/src/models.rs`
- Test: `crates/todoee-core/tests/integration.rs`

### Step 1.1: Define Operation model

Add to `crates/todoee-core/src/models.rs`:

```rust
/// Represents a tracked operation for undo/redo functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: Uuid,
    pub operation_type: OperationType,
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub previous_state: Option<serde_json::Value>,
    pub new_state: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub undone: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    Create,
    Update,
    Delete,
    Complete,
    Uncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Todo,
    Category,
}

impl Operation {
    pub fn new(
        operation_type: OperationType,
        entity_type: EntityType,
        entity_id: Uuid,
        previous_state: Option<serde_json::Value>,
        new_state: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            operation_type,
            entity_type,
            entity_id,
            previous_state,
            new_state,
            created_at: Utc::now(),
            undone: false,
        }
    }
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Create => write!(f, "create"),
            OperationType::Update => write!(f, "update"),
            OperationType::Delete => write!(f, "delete"),
            OperationType::Complete => write!(f, "complete"),
            OperationType::Uncomplete => write!(f, "uncomplete"),
        }
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Todo => write!(f, "todo"),
            EntityType::Category => write!(f, "category"),
        }
    }
}
```

### Step 1.2: Run tests to verify model compiles

Run: `cargo build -p todoee-core`
Expected: PASS

### Step 1.3: Add migration for operations table

Add to `crates/todoee-core/src/db/local.rs` in `run_migrations()`:

```rust
// Add after existing migrations
sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS operations (
        id TEXT PRIMARY KEY,
        operation_type TEXT NOT NULL,
        entity_type TEXT NOT NULL,
        entity_id TEXT NOT NULL,
        previous_state TEXT,
        new_state TEXT,
        created_at TEXT NOT NULL,
        undone INTEGER NOT NULL DEFAULT 0
    )
    "#,
)
.execute(&self.pool)
.await?;

sqlx::query(
    "CREATE INDEX IF NOT EXISTS idx_operations_created_at ON operations(created_at DESC)"
)
.execute(&self.pool)
.await?;
```

### Step 1.4: Add CRUD methods for operations

Add to `LocalDb` impl in `crates/todoee-core/src/db/local.rs`:

```rust
/// Record an operation for undo/redo history
pub async fn record_operation(&self, op: &Operation) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO operations (id, operation_type, entity_type, entity_id, previous_state, new_state, created_at, undone)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(op.id.to_string())
    .bind(op.operation_type.to_string())
    .bind(op.entity_type.to_string())
    .bind(op.entity_id.to_string())
    .bind(op.previous_state.as_ref().map(|v| v.to_string()))
    .bind(op.new_state.as_ref().map(|v| v.to_string()))
    .bind(op.created_at.to_rfc3339())
    .bind(op.undone as i32)
    .execute(&self.pool)
    .await?;
    Ok(())
}

/// Get the most recent undoable operation
pub async fn get_last_undoable_operation(&self) -> Result<Option<Operation>> {
    let row = sqlx::query(
        r#"
        SELECT id, operation_type, entity_type, entity_id, previous_state, new_state, created_at, undone
        FROM operations
        WHERE undone = 0
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(&self.pool)
    .await?;

    match row {
        Some(row) => Ok(Some(self.row_to_operation(&row)?)),
        None => Ok(None),
    }
}

/// Get the most recent undone operation (for redo)
pub async fn get_last_redoable_operation(&self) -> Result<Option<Operation>> {
    let row = sqlx::query(
        r#"
        SELECT id, operation_type, entity_type, entity_id, previous_state, new_state, created_at, undone
        FROM operations
        WHERE undone = 1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(&self.pool)
    .await?;

    match row {
        Some(row) => Ok(Some(self.row_to_operation(&row)?)),
        None => Ok(None),
    }
}

/// Mark an operation as undone
pub async fn mark_operation_undone(&self, id: Uuid) -> Result<()> {
    sqlx::query("UPDATE operations SET undone = 1 WHERE id = ?")
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
    Ok(())
}

/// Mark an operation as not undone (for redo)
pub async fn mark_operation_redone(&self, id: Uuid) -> Result<()> {
    sqlx::query("UPDATE operations SET undone = 0 WHERE id = ?")
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
    Ok(())
}

/// List recent operations for log command
pub async fn list_operations(&self, limit: usize) -> Result<Vec<Operation>> {
    let rows = sqlx::query(
        r#"
        SELECT id, operation_type, entity_type, entity_id, previous_state, new_state, created_at, undone
        FROM operations
        ORDER BY created_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit as i64)
    .fetch_all(&self.pool)
    .await?;

    rows.iter().map(|row| self.row_to_operation(row)).collect()
}

/// Clear operations older than N days
pub async fn clear_old_operations(&self, days: i64) -> Result<u64> {
    let cutoff = Utc::now() - chrono::Duration::days(days);
    let result = sqlx::query("DELETE FROM operations WHERE created_at < ?")
        .bind(cutoff.to_rfc3339())
        .execute(&self.pool)
        .await?;
    Ok(result.rows_affected())
}

fn row_to_operation(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Operation> {
    use sqlx::Row;

    let op_type_str: String = row.get("operation_type");
    let entity_type_str: String = row.get("entity_type");

    let operation_type = match op_type_str.as_str() {
        "create" => OperationType::Create,
        "update" => OperationType::Update,
        "delete" => OperationType::Delete,
        "complete" => OperationType::Complete,
        "uncomplete" => OperationType::Uncomplete,
        _ => return Err(anyhow::anyhow!("Unknown operation type: {}", op_type_str)),
    };

    let entity_type = match entity_type_str.as_str() {
        "todo" => EntityType::Todo,
        "category" => EntityType::Category,
        _ => return Err(anyhow::anyhow!("Unknown entity type: {}", entity_type_str)),
    };

    let prev_state: Option<String> = row.get("previous_state");
    let new_state: Option<String> = row.get("new_state");

    Ok(Operation {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        operation_type,
        entity_type,
        entity_id: Uuid::parse_str(row.get::<&str, _>("entity_id"))?,
        previous_state: prev_state.map(|s| serde_json::from_str(&s)).transpose()?,
        new_state: new_state.map(|s| serde_json::from_str(&s)).transpose()?,
        created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
        undone: row.get::<i32, _>("undone") != 0,
    })
}
```

### Step 1.5: Run tests

Run: `cargo test -p todoee-core`
Expected: PASS

### Step 1.6: Commit

```bash
git add crates/todoee-core/src/models.rs crates/todoee-core/src/db/local.rs
git commit -m "feat(core): add operation history table for undo/redo support

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Implement Undo/Redo Commands

**Files:**
- Create: `crates/todoee-cli/src/commands/undo.rs`
- Create: `crates/todoee-cli/src/commands/redo.rs`
- Modify: `crates/todoee-cli/src/commands/mod.rs`
- Modify: `crates/todoee-cli/src/main.rs`

### Step 2.1: Write failing test for undo

Create test in `crates/todoee-core/tests/integration.rs`:

```rust
#[tokio::test]
async fn test_undo_create_operation() {
    let db = setup_db().await;

    // Create a todo
    let todo = Todo::new("Test undo".to_string());
    let todo_json = serde_json::to_value(&todo).unwrap();
    db.create_todo(&todo).await.unwrap();

    // Record the operation
    let op = Operation::new(
        OperationType::Create,
        EntityType::Todo,
        todo.id,
        None,
        Some(todo_json),
    );
    db.record_operation(&op).await.unwrap();

    // Get last undoable
    let last_op = db.get_last_undoable_operation().await.unwrap();
    assert!(last_op.is_some());
    assert_eq!(last_op.unwrap().entity_id, todo.id);
}
```

### Step 2.2: Run test to verify it fails

Run: `cargo test -p todoee-core test_undo_create_operation`
Expected: FAIL (Operation not in scope)

### Step 2.3: Export Operation from lib.rs

Add to `crates/todoee-core/src/lib.rs`:

```rust
pub use models::{Operation, OperationType, EntityType};
```

### Step 2.4: Run test to verify it passes

Run: `cargo test -p todoee-core test_undo_create_operation`
Expected: PASS

### Step 2.5: Create undo command

Create `crates/todoee-cli/src/commands/undo.rs`:

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb, Operation, OperationType, EntityType, Todo};

pub async fn undo() -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;

    let Some(op) = db.get_last_undoable_operation().await? else {
        println!("Nothing to undo");
        return Ok(());
    };

    match (op.operation_type, op.entity_type) {
        (OperationType::Create, EntityType::Todo) => {
            // Undo create = delete the todo
            db.delete_todo(op.entity_id).await?;
            println!("Undone: created todo deleted");
        }
        (OperationType::Delete, EntityType::Todo) => {
            // Undo delete = restore the todo from previous_state
            if let Some(prev) = &op.previous_state {
                let todo: Todo = serde_json::from_value(prev.clone())?;
                db.create_todo(&todo).await?;
                println!("Undone: deleted todo '{}' restored", todo.title);
            }
        }
        (OperationType::Update, EntityType::Todo) => {
            // Undo update = restore previous state
            if let Some(prev) = &op.previous_state {
                let todo: Todo = serde_json::from_value(prev.clone())?;
                db.update_todo(&todo).await?;
                println!("Undone: todo '{}' reverted", todo.title);
            }
        }
        (OperationType::Complete, EntityType::Todo) => {
            // Undo complete = mark incomplete
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_incomplete();
                db.update_todo(&todo).await?;
                println!("Undone: '{}' marked incomplete", todo.title);
            }
        }
        (OperationType::Uncomplete, EntityType::Todo) => {
            // Undo uncomplete = mark complete
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_complete();
                db.update_todo(&todo).await?;
                println!("Undone: '{}' marked complete", todo.title);
            }
        }
        (_, EntityType::Category) => {
            println!("Category undo not yet implemented");
        }
    }

    db.mark_operation_undone(op.id).await?;
    Ok(())
}
```

### Step 2.6: Create redo command

Create `crates/todoee-cli/src/commands/redo.rs`:

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb, Operation, OperationType, EntityType, Todo};

pub async fn redo() -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;

    let Some(op) = db.get_last_redoable_operation().await? else {
        println!("Nothing to redo");
        return Ok(());
    };

    match (op.operation_type, op.entity_type) {
        (OperationType::Create, EntityType::Todo) => {
            // Redo create = recreate the todo
            if let Some(new) = &op.new_state {
                let todo: Todo = serde_json::from_value(new.clone())?;
                db.create_todo(&todo).await?;
                println!("Redone: todo '{}' created", todo.title);
            }
        }
        (OperationType::Delete, EntityType::Todo) => {
            // Redo delete = delete again
            db.delete_todo(op.entity_id).await?;
            println!("Redone: todo deleted");
        }
        (OperationType::Update, EntityType::Todo) => {
            // Redo update = apply new state
            if let Some(new) = &op.new_state {
                let todo: Todo = serde_json::from_value(new.clone())?;
                db.update_todo(&todo).await?;
                println!("Redone: todo '{}' updated", todo.title);
            }
        }
        (OperationType::Complete, EntityType::Todo) => {
            // Redo complete = mark complete again
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_complete();
                db.update_todo(&todo).await?;
                println!("Redone: '{}' marked complete", todo.title);
            }
        }
        (OperationType::Uncomplete, EntityType::Todo) => {
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_incomplete();
                db.update_todo(&todo).await?;
                println!("Redone: '{}' marked incomplete", todo.title);
            }
        }
        (_, EntityType::Category) => {
            println!("Category redo not yet implemented");
        }
    }

    db.mark_operation_redone(op.id).await?;
    Ok(())
}
```

### Step 2.7: Register commands in mod.rs

Add to `crates/todoee-cli/src/commands/mod.rs`:

```rust
pub mod undo;
pub mod redo;

pub use undo::undo;
pub use redo::redo;
```

### Step 2.8: Add to CLI in main.rs

Add to `Commands` enum in `crates/todoee-cli/src/main.rs`:

```rust
/// Undo the last operation
Undo,
/// Redo the last undone operation
Redo,
```

Add match arms:

```rust
Commands::Undo => commands::undo().await?,
Commands::Redo => commands::redo().await?,
```

### Step 2.9: Build and test

Run: `cargo build -p todoee-cli`
Expected: PASS

### Step 2.10: Commit

```bash
git add crates/todoee-cli/src/commands/undo.rs crates/todoee-cli/src/commands/redo.rs crates/todoee-cli/src/commands/mod.rs crates/todoee-cli/src/main.rs
git commit -m "feat(cli): add undo and redo commands

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Instrument Existing Commands to Record Operations

**Files:**
- Modify: `crates/todoee-cli/src/commands/add.rs`
- Modify: `crates/todoee-cli/src/commands/done.rs`
- Modify: `crates/todoee-cli/src/commands/delete.rs`
- Modify: `crates/todoee-cli/src/commands/edit.rs`

### Step 3.1: Update add.rs to record operation

After `db.create_todo(&todo).await?;` in `crates/todoee-cli/src/commands/add.rs`:

```rust
// Record operation for undo
let op = Operation::new(
    OperationType::Create,
    EntityType::Todo,
    todo.id,
    None,
    Some(serde_json::to_value(&todo)?),
);
db.record_operation(&op).await?;
```

Add imports at top:

```rust
use todoee_core::{Operation, OperationType, EntityType};
```

### Step 3.2: Update done.rs to record operation

Before marking complete in `crates/todoee-cli/src/commands/done.rs`:

```rust
// Record operation for undo
let op = Operation::new(
    OperationType::Complete,
    EntityType::Todo,
    todo.id,
    Some(serde_json::to_value(&todo)?),
    None,
);
```

After update:

```rust
db.record_operation(&op).await?;
```

### Step 3.3: Update delete.rs to record operation

Before deletion in `crates/todoee-cli/src/commands/delete.rs`:

```rust
// Fetch todo before deletion for undo
let todo = db.get_todo(todo_id).await?.ok_or_else(|| anyhow::anyhow!("Todo not found"))?;

// Record operation for undo
let op = Operation::new(
    OperationType::Delete,
    EntityType::Todo,
    todo.id,
    Some(serde_json::to_value(&todo)?),
    None,
);
db.record_operation(&op).await?;
```

### Step 3.4: Update edit.rs to record operation

Before update in `crates/todoee-cli/src/commands/edit.rs`:

```rust
// Save previous state for undo
let previous_state = serde_json::to_value(&todo)?;
```

After update:

```rust
// Record operation for undo
let op = Operation::new(
    OperationType::Update,
    EntityType::Todo,
    todo.id,
    Some(previous_state),
    Some(serde_json::to_value(&todo)?),
);
db.record_operation(&op).await?;
```

### Step 3.5: Build and test

Run: `cargo build -p todoee-cli`
Expected: PASS

### Step 3.6: Commit

```bash
git add crates/todoee-cli/src/commands/add.rs crates/todoee-cli/src/commands/done.rs crates/todoee-cli/src/commands/delete.rs crates/todoee-cli/src/commands/edit.rs
git commit -m "feat(cli): record operations for undo support in add/done/delete/edit

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Implement Log Command

**Files:**
- Create: `crates/todoee-cli/src/commands/log.rs`
- Modify: `crates/todoee-cli/src/commands/mod.rs`
- Modify: `crates/todoee-cli/src/main.rs`

### Step 4.1: Create log command

Create `crates/todoee-cli/src/commands/log.rs`:

```rust
use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use todoee_core::{Config, LocalDb};

pub async fn log(limit: Option<usize>) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;

    let limit = limit.unwrap_or(10);
    let operations = db.list_operations(limit).await?;

    if operations.is_empty() {
        println!("No operations recorded yet.");
        return Ok(());
    }

    println!("Operation History (most recent first):\n");

    for op in operations {
        let local_time = Local.from_utc_datetime(&op.created_at.naive_utc());
        let time_str = local_time.format("%Y-%m-%d %H:%M:%S");
        let status = if op.undone { " [undone]" } else { "" };

        let entity_info = match op.new_state.as_ref().or(op.previous_state.as_ref()) {
            Some(state) => {
                state.get("title")
                    .and_then(|t| t.as_str())
                    .map(|s| format!(" \"{}\"", truncate(s, 40)))
                    .unwrap_or_default()
            }
            None => String::new(),
        };

        let short_id = &op.entity_id.to_string()[..8];

        println!(
            "{} {} {} {}{}{}",
            time_str,
            op.operation_type,
            op.entity_type,
            short_id,
            entity_info,
            status
        );
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max.min(s.len())]
    }
}
```

### Step 4.2: Register in mod.rs

Add to `crates/todoee-cli/src/commands/mod.rs`:

```rust
pub mod log;
pub use log::log;
```

### Step 4.3: Add to CLI

Add to `Commands` enum:

```rust
/// Show operation history
Log {
    /// Number of operations to show
    #[arg(short = 'n', long, default_value = "10")]
    limit: Option<usize>,
},
```

Add match arm:

```rust
Commands::Log { limit } => commands::log(limit).await?,
```

### Step 4.4: Build and test

Run: `cargo build -p todoee-cli && ./target/debug/todoee log`
Expected: PASS (shows "No operations recorded yet." or history)

### Step 4.5: Commit

```bash
git add crates/todoee-cli/src/commands/log.rs crates/todoee-cli/src/commands/mod.rs crates/todoee-cli/src/main.rs
git commit -m "feat(cli): add log command to view operation history

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Implement Head/Tail/Upcoming Commands

**Files:**
- Create: `crates/todoee-cli/src/commands/head.rs`
- Create: `crates/todoee-cli/src/commands/tail.rs`
- Create: `crates/todoee-cli/src/commands/upcoming.rs`
- Modify: `crates/todoee-core/src/db/local.rs` (add query methods)

### Step 5.1: Add query methods to LocalDb

Add to `crates/todoee-core/src/db/local.rs`:

```rust
/// List N most recently created todos
pub async fn list_todos_head(&self, limit: usize, include_completed: bool) -> Result<Vec<Todo>> {
    let query = if include_completed {
        "SELECT * FROM todos ORDER BY created_at DESC LIMIT ?"
    } else {
        "SELECT * FROM todos WHERE is_completed = 0 ORDER BY created_at DESC LIMIT ?"
    };

    let rows = sqlx::query(query)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

    rows.iter().map(|row| self.row_to_todo(row)).collect()
}

/// List N oldest todos
pub async fn list_todos_tail(&self, limit: usize, include_completed: bool) -> Result<Vec<Todo>> {
    let query = if include_completed {
        "SELECT * FROM todos ORDER BY created_at ASC LIMIT ?"
    } else {
        "SELECT * FROM todos WHERE is_completed = 0 ORDER BY created_at ASC LIMIT ?"
    };

    let rows = sqlx::query(query)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

    rows.iter().map(|row| self.row_to_todo(row)).collect()
}

/// List N upcoming todos by due date
pub async fn list_todos_upcoming(&self, limit: usize) -> Result<Vec<Todo>> {
    let now = Utc::now().to_rfc3339();
    let rows = sqlx::query(
        r#"
        SELECT * FROM todos
        WHERE is_completed = 0 AND due_date IS NOT NULL AND due_date >= ?
        ORDER BY due_date ASC
        LIMIT ?
        "#,
    )
    .bind(&now)
    .bind(limit as i64)
    .fetch_all(&self.pool)
    .await?;

    rows.iter().map(|row| self.row_to_todo(row)).collect()
}

/// List overdue todos
pub async fn list_todos_overdue(&self) -> Result<Vec<Todo>> {
    let now = Utc::now().to_rfc3339();
    let rows = sqlx::query(
        r#"
        SELECT * FROM todos
        WHERE is_completed = 0 AND due_date IS NOT NULL AND due_date < ?
        ORDER BY due_date ASC
        "#,
    )
    .bind(&now)
    .fetch_all(&self.pool)
    .await?;

    rows.iter().map(|row| self.row_to_todo(row)).collect()
}
```

### Step 5.2: Create head command

Create `crates/todoee-cli/src/commands/head.rs`:

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb};

pub async fn head(count: usize, all: bool) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;
    let todos = db.list_todos_head(count, all).await?;

    if todos.is_empty() {
        println!("No todos found.");
        return Ok(());
    }

    println!("Last {} todos (most recent first):\n", todos.len());

    for todo in todos {
        let check = if todo.is_completed { "[x]" } else { "[ ]" };
        let priority = match todo.priority {
            todoee_core::Priority::High => "!!!",
            todoee_core::Priority::Medium => "!! ",
            todoee_core::Priority::Low => "!  ",
        };
        let short_id = &todo.id.to_string()[..8];

        println!("{} {} {} {}", check, priority, short_id, todo.title);
    }

    Ok(())
}
```

### Step 5.3: Create tail command

Create `crates/todoee-cli/src/commands/tail.rs`:

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb};

pub async fn tail(count: usize, all: bool) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;
    let todos = db.list_todos_tail(count, all).await?;

    if todos.is_empty() {
        println!("No todos found.");
        return Ok(());
    }

    println!("Oldest {} todos:\n", todos.len());

    for todo in todos {
        let check = if todo.is_completed { "[x]" } else { "[ ]" };
        let priority = match todo.priority {
            todoee_core::Priority::High => "!!!",
            todoee_core::Priority::Medium => "!! ",
            todoee_core::Priority::Low => "!  ",
        };
        let short_id = &todo.id.to_string()[..8];
        let age = chrono::Utc::now().signed_duration_since(todo.created_at);
        let age_str = format_duration(age);

        println!("{} {} {} {} ({})", check, priority, short_id, todo.title, age_str);
    }

    Ok(())
}

fn format_duration(d: chrono::Duration) -> String {
    let days = d.num_days();
    if days > 0 {
        format!("{}d ago", days)
    } else {
        let hours = d.num_hours();
        if hours > 0 {
            format!("{}h ago", hours)
        } else {
            "just now".to_string()
        }
    }
}
```

### Step 5.4: Create upcoming command

Create `crates/todoee-cli/src/commands/upcoming.rs`:

```rust
use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use todoee_core::{Config, LocalDb};

pub async fn upcoming(count: usize) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;
    let todos = db.list_todos_upcoming(count).await?;

    if todos.is_empty() {
        println!("No upcoming todos with due dates.");
        return Ok(());
    }

    println!("Next {} upcoming todos:\n", todos.len());

    for todo in todos {
        let priority = match todo.priority {
            todoee_core::Priority::High => "!!!",
            todoee_core::Priority::Medium => "!! ",
            todoee_core::Priority::Low => "!  ",
        };
        let short_id = &todo.id.to_string()[..8];

        let due_str = todo.due_date
            .map(|d| {
                let local = Local.from_utc_datetime(&d.naive_utc());
                local.format("%Y-%m-%d %H:%M").to_string()
            })
            .unwrap_or_default();

        println!("{} {} {} [due: {}]", priority, short_id, todo.title, due_str);
    }

    Ok(())
}

pub async fn overdue() -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;
    let todos = db.list_todos_overdue().await?;

    if todos.is_empty() {
        println!("No overdue todos!");
        return Ok(());
    }

    println!("{} overdue todos:\n", todos.len());

    for todo in todos {
        let priority = match todo.priority {
            todoee_core::Priority::High => "!!!",
            todoee_core::Priority::Medium => "!! ",
            todoee_core::Priority::Low => "!  ",
        };
        let short_id = &todo.id.to_string()[..8];

        let overdue_by = todo.due_date
            .map(|d| {
                let diff = chrono::Utc::now().signed_duration_since(d);
                let days = diff.num_days();
                if days > 0 {
                    format!("{} days overdue", days)
                } else {
                    let hours = diff.num_hours();
                    format!("{} hours overdue", hours)
                }
            })
            .unwrap_or_default();

        println!("{} {} {} [{}]", priority, short_id, todo.title, overdue_by);
    }

    Ok(())
}
```

### Step 5.5: Register in mod.rs

Add to `crates/todoee-cli/src/commands/mod.rs`:

```rust
pub mod head;
pub mod tail;
pub mod upcoming;

pub use head::head;
pub use tail::tail;
pub use upcoming::{upcoming, overdue};
```

### Step 5.6: Add to CLI

Add to `Commands` enum:

```rust
/// Show last N todos (most recent)
Head {
    /// Number of todos to show
    #[arg(default_value = "5")]
    count: usize,
    /// Include completed todos
    #[arg(short, long)]
    all: bool,
},
/// Show oldest N todos
Tail {
    /// Number of todos to show
    #[arg(default_value = "5")]
    count: usize,
    /// Include completed todos
    #[arg(short, long)]
    all: bool,
},
/// Show next N upcoming todos by due date
Upcoming {
    /// Number of todos to show
    #[arg(default_value = "5")]
    count: usize,
},
/// Show all overdue todos
Overdue,
```

Add match arms:

```rust
Commands::Head { count, all } => commands::head(count, all).await?,
Commands::Tail { count, all } => commands::tail(count, all).await?,
Commands::Upcoming { count } => commands::upcoming(count).await?,
Commands::Overdue => commands::overdue().await?,
```

### Step 5.7: Build and test

Run: `cargo build -p todoee-cli`
Expected: PASS

### Step 5.8: Commit

```bash
git add crates/todoee-core/src/db/local.rs crates/todoee-cli/src/commands/
git commit -m "feat(cli): add head, tail, upcoming, and overdue commands

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Implement Stash Commands

**Files:**
- Modify: `crates/todoee-core/src/db/local.rs` (add stash table)
- Create: `crates/todoee-cli/src/commands/stash.rs`

### Step 6.1: Add stash table migration

Add to `run_migrations()` in `crates/todoee-core/src/db/local.rs`:

```rust
sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS stash (
        id TEXT PRIMARY KEY,
        todo_json TEXT NOT NULL,
        stashed_at TEXT NOT NULL,
        message TEXT
    )
    "#,
)
.execute(&self.pool)
.await?;
```

### Step 6.2: Add stash methods to LocalDb

```rust
/// Stash a todo (hide it temporarily)
pub async fn stash_todo(&self, todo_id: Uuid, message: Option<&str>) -> Result<()> {
    let todo = self.get_todo(todo_id).await?
        .ok_or_else(|| anyhow::anyhow!("Todo not found"))?;

    let todo_json = serde_json::to_string(&todo)?;

    sqlx::query(
        "INSERT INTO stash (id, todo_json, stashed_at, message) VALUES (?, ?, ?, ?)"
    )
    .bind(todo_id.to_string())
    .bind(&todo_json)
    .bind(Utc::now().to_rfc3339())
    .bind(message)
    .execute(&self.pool)
    .await?;

    self.delete_todo(todo_id).await?;
    Ok(())
}

/// Pop the most recently stashed todo
pub async fn stash_pop(&self) -> Result<Option<Todo>> {
    let row = sqlx::query(
        "SELECT id, todo_json FROM stash ORDER BY stashed_at DESC LIMIT 1"
    )
    .fetch_optional(&self.pool)
    .await?;

    match row {
        Some(row) => {
            use sqlx::Row;
            let id: String = row.get("id");
            let json: String = row.get("todo_json");
            let todo: Todo = serde_json::from_str(&json)?;

            // Remove from stash
            sqlx::query("DELETE FROM stash WHERE id = ?")
                .bind(&id)
                .execute(&self.pool)
                .await?;

            // Restore to todos
            self.create_todo(&todo).await?;

            Ok(Some(todo))
        }
        None => Ok(None),
    }
}

/// List all stashed todos
pub async fn stash_list(&self) -> Result<Vec<(Todo, String, Option<String>)>> {
    let rows = sqlx::query(
        "SELECT todo_json, stashed_at, message FROM stash ORDER BY stashed_at DESC"
    )
    .fetch_all(&self.pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        use sqlx::Row;
        let json: String = row.get("todo_json");
        let stashed_at: String = row.get("stashed_at");
        let message: Option<String> = row.get("message");
        let todo: Todo = serde_json::from_str(&json)?;
        result.push((todo, stashed_at, message));
    }
    Ok(result)
}

/// Clear all stashed todos
pub async fn stash_clear(&self) -> Result<u64> {
    let result = sqlx::query("DELETE FROM stash")
        .execute(&self.pool)
        .await?;
    Ok(result.rows_affected())
}
```

### Step 6.3: Create stash command

Create `crates/todoee-cli/src/commands/stash.rs`:

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb};

#[derive(Clone)]
pub enum StashAction {
    Push { id: String, message: Option<String> },
    Pop,
    List,
    Clear,
}

pub async fn stash(action: StashAction) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;

    match action {
        StashAction::Push { id, message } => {
            let todos = db.list_todos(false).await?;
            let matching: Vec<_> = todos.iter()
                .filter(|t| t.id.to_string().starts_with(&id))
                .collect();

            match matching.len() {
                0 => println!("No todo found with ID starting with '{}'", id),
                1 => {
                    let todo = matching[0];
                    db.stash_todo(todo.id, message.as_deref()).await?;
                    println!("Stashed: {}", todo.title);
                }
                _ => {
                    println!("Multiple todos match '{}'. Be more specific:", id);
                    for t in matching {
                        println!("  {} - {}", &t.id.to_string()[..8], t.title);
                    }
                }
            }
        }
        StashAction::Pop => {
            match db.stash_pop().await? {
                Some(todo) => println!("Restored: {}", todo.title),
                None => println!("Stash is empty"),
            }
        }
        StashAction::List => {
            let stashed = db.stash_list().await?;
            if stashed.is_empty() {
                println!("Stash is empty");
            } else {
                println!("Stashed todos:\n");
                for (i, (todo, at, msg)) in stashed.iter().enumerate() {
                    let msg_str = msg.as_ref().map(|m| format!(": {}", m)).unwrap_or_default();
                    println!("stash@{{{}}}{} - {}", i, msg_str, todo.title);
                }
            }
        }
        StashAction::Clear => {
            let count = db.stash_clear().await?;
            println!("Cleared {} stashed todo(s)", count);
        }
    }

    Ok(())
}
```

### Step 6.4: Register in mod.rs and main.rs

Add to mod.rs:

```rust
pub mod stash;
pub use stash::{stash, StashAction};
```

Add to Commands enum:

```rust
/// Stash todos temporarily
Stash {
    #[command(subcommand)]
    action: StashSubcommand,
},
```

Add StashSubcommand:

```rust
#[derive(Subcommand, Clone)]
pub enum StashSubcommand {
    /// Stash a todo by ID
    Push {
        /// Todo ID (or prefix)
        id: String,
        /// Optional message
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Pop the most recently stashed todo
    Pop,
    /// List stashed todos
    List,
    /// Clear all stashed todos
    Clear,
}
```

Add match arm:

```rust
Commands::Stash { action } => {
    let action = match action {
        StashSubcommand::Push { id, message } => commands::StashAction::Push { id, message },
        StashSubcommand::Pop => commands::StashAction::Pop,
        StashSubcommand::List => commands::StashAction::List,
        StashSubcommand::Clear => commands::StashAction::Clear,
    };
    commands::stash(action).await?
}
```

### Step 6.5: Build and test

Run: `cargo build -p todoee-cli`
Expected: PASS

### Step 6.6: Commit

```bash
git add crates/todoee-core/src/db/local.rs crates/todoee-cli/src/commands/stash.rs crates/todoee-cli/src/commands/mod.rs crates/todoee-cli/src/main.rs
git commit -m "feat(cli): add stash command for temporarily hiding todos

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Implement Batch Commands

**Files:**
- Create: `crates/todoee-cli/src/commands/batch.rs`

### Step 7.1: Create batch command

Create `crates/todoee-cli/src/commands/batch.rs`:

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb, Operation, OperationType, EntityType};

#[derive(Clone)]
pub enum BatchAction {
    Done { ids: Vec<String> },
    Delete { ids: Vec<String> },
    Priority { ids: Vec<String>, priority: u8 },
}

pub async fn batch(action: BatchAction) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;
    let all_todos = db.list_todos(true).await?;

    match action {
        BatchAction::Done { ids } => {
            let mut count = 0;
            for id in &ids {
                let matching: Vec<_> = all_todos.iter()
                    .filter(|t| t.id.to_string().starts_with(id))
                    .collect();

                if matching.len() == 1 {
                    let mut todo = matching[0].clone();
                    if !todo.is_completed {
                        let prev = serde_json::to_value(&todo)?;
                        todo.mark_complete();
                        db.update_todo(&todo).await?;

                        let op = Operation::new(
                            OperationType::Complete,
                            EntityType::Todo,
                            todo.id,
                            Some(prev),
                            None,
                        );
                        db.record_operation(&op).await?;
                        count += 1;
                    }
                }
            }
            println!("Marked {} todo(s) as done", count);
        }
        BatchAction::Delete { ids } => {
            let mut count = 0;
            for id in &ids {
                let matching: Vec<_> = all_todos.iter()
                    .filter(|t| t.id.to_string().starts_with(id))
                    .collect();

                if matching.len() == 1 {
                    let todo = matching[0];
                    let op = Operation::new(
                        OperationType::Delete,
                        EntityType::Todo,
                        todo.id,
                        Some(serde_json::to_value(todo)?),
                        None,
                    );
                    db.record_operation(&op).await?;
                    db.delete_todo(todo.id).await?;
                    count += 1;
                }
            }
            println!("Deleted {} todo(s)", count);
        }
        BatchAction::Priority { ids, priority } => {
            let priority = match priority {
                1 => todoee_core::Priority::Low,
                2 => todoee_core::Priority::Medium,
                3 => todoee_core::Priority::High,
                _ => {
                    println!("Priority must be 1, 2, or 3");
                    return Ok(());
                }
            };

            let mut count = 0;
            for id in &ids {
                let matching: Vec<_> = all_todos.iter()
                    .filter(|t| t.id.to_string().starts_with(id))
                    .collect();

                if matching.len() == 1 {
                    let mut todo = matching[0].clone();
                    let prev = serde_json::to_value(&todo)?;
                    todo.priority = priority;
                    db.update_todo(&todo).await?;

                    let op = Operation::new(
                        OperationType::Update,
                        EntityType::Todo,
                        todo.id,
                        Some(prev),
                        Some(serde_json::to_value(&todo)?),
                    );
                    db.record_operation(&op).await?;
                    count += 1;
                }
            }
            println!("Updated priority for {} todo(s)", count);
        }
    }

    Ok(())
}
```

### Step 7.2: Register and add CLI

Add to mod.rs:

```rust
pub mod batch;
pub use batch::{batch, BatchAction};
```

Add to Commands enum and handle in match.

### Step 7.3: Commit

```bash
git add crates/todoee-cli/src/commands/batch.rs crates/todoee-cli/src/commands/mod.rs crates/todoee-cli/src/main.rs
git commit -m "feat(cli): add batch command for bulk operations

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 8: Implement Show and Diff Commands

**Files:**
- Create: `crates/todoee-cli/src/commands/show.rs`
- Create: `crates/todoee-cli/src/commands/diff.rs`

### Step 8.1: Create show command

Create `crates/todoee-cli/src/commands/show.rs`:

```rust
use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use todoee_core::{Config, LocalDb, Priority};

pub async fn show(id: String) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;
    let todos = db.list_todos(true).await?;

    let matching: Vec<_> = todos.iter()
        .filter(|t| t.id.to_string().starts_with(&id))
        .collect();

    match matching.len() {
        0 => println!("No todo found with ID starting with '{}'", id),
        1 => {
            let todo = matching[0];
            let categories = db.list_categories().await?;
            let category_name = todo.category_id
                .and_then(|cid| categories.iter().find(|c| c.id == cid))
                .map(|c| c.name.as_str())
                .unwrap_or("None");

            println!("Todo: {}", todo.id);
            println!("─────────────────────────────────────────");
            println!("Title:       {}", todo.title);
            if let Some(desc) = &todo.description {
                println!("Description: {}", desc);
            }
            println!("Priority:    {}", match todo.priority {
                Priority::Low => "Low (!)",
                Priority::Medium => "Medium (!!)",
                Priority::High => "High (!!!)",
            });
            println!("Category:    {}", category_name);
            println!("Status:      {}", if todo.is_completed { "Completed" } else { "Pending" });

            if let Some(due) = todo.due_date {
                let local = Local.from_utc_datetime(&due.naive_utc());
                println!("Due:         {}", local.format("%Y-%m-%d %H:%M"));
            }

            if let Some(reminder) = todo.reminder_at {
                let local = Local.from_utc_datetime(&reminder.naive_utc());
                println!("Reminder:    {}", local.format("%Y-%m-%d %H:%M"));
            }

            if let Some(completed) = todo.completed_at {
                let local = Local.from_utc_datetime(&completed.naive_utc());
                println!("Completed:   {}", local.format("%Y-%m-%d %H:%M"));
            }

            let created = Local.from_utc_datetime(&todo.created_at.naive_utc());
            let updated = Local.from_utc_datetime(&todo.updated_at.naive_utc());
            println!("Created:     {}", created.format("%Y-%m-%d %H:%M"));
            println!("Updated:     {}", updated.format("%Y-%m-%d %H:%M"));
            println!("Sync Status: {:?}", todo.sync_status);
        }
        _ => {
            println!("Multiple todos match '{}'. Be more specific:", id);
            for t in matching {
                println!("  {} - {}", &t.id.to_string()[..8], t.title);
            }
        }
    }

    Ok(())
}
```

### Step 8.2: Create diff command

Create `crates/todoee-cli/src/commands/diff.rs`:

```rust
use anyhow::{Context, Result};
use chrono::{Duration, Local, TimeZone, Utc};
use todoee_core::{Config, LocalDb, OperationType};

pub async fn diff(hours: Option<i64>) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;

    let hours = hours.unwrap_or(24);
    let since = Utc::now() - Duration::hours(hours);

    let operations = db.list_operations(100).await?;
    let recent: Vec<_> = operations.iter()
        .filter(|op| op.created_at > since && !op.undone)
        .collect();

    if recent.is_empty() {
        println!("No changes in the last {} hours.", hours);
        return Ok(());
    }

    println!("Changes in the last {} hours:\n", hours);

    for op in recent {
        let local = Local.from_utc_datetime(&op.created_at.naive_utc());
        let time = local.format("%H:%M");

        match op.operation_type {
            OperationType::Create => {
                if let Some(new) = &op.new_state {
                    let title = new.get("title").and_then(|t| t.as_str()).unwrap_or("?");
                    println!("+ {} [{}] Created: {}", time, &op.entity_id.to_string()[..8], title);
                }
            }
            OperationType::Delete => {
                if let Some(prev) = &op.previous_state {
                    let title = prev.get("title").and_then(|t| t.as_str()).unwrap_or("?");
                    println!("- {} [{}] Deleted: {}", time, &op.entity_id.to_string()[..8], title);
                }
            }
            OperationType::Update => {
                if let (Some(prev), Some(new)) = (&op.previous_state, &op.new_state) {
                    let title = new.get("title").and_then(|t| t.as_str()).unwrap_or("?");
                    print!("~ {} [{}] Updated: {}", time, &op.entity_id.to_string()[..8], title);

                    // Show what changed
                    let mut changes = Vec::new();
                    if prev.get("title") != new.get("title") {
                        changes.push("title");
                    }
                    if prev.get("priority") != new.get("priority") {
                        changes.push("priority");
                    }
                    if prev.get("due_date") != new.get("due_date") {
                        changes.push("due_date");
                    }
                    if prev.get("category_id") != new.get("category_id") {
                        changes.push("category");
                    }

                    if !changes.is_empty() {
                        print!(" ({})", changes.join(", "));
                    }
                    println!();
                }
            }
            OperationType::Complete => {
                if let Some(prev) = &op.previous_state {
                    let title = prev.get("title").and_then(|t| t.as_str()).unwrap_or("?");
                    println!("✓ {} [{}] Completed: {}", time, &op.entity_id.to_string()[..8], title);
                }
            }
            OperationType::Uncomplete => {
                if let Some(prev) = &op.previous_state {
                    let title = prev.get("title").and_then(|t| t.as_str()).unwrap_or("?");
                    println!("○ {} [{}] Uncompleted: {}", time, &op.entity_id.to_string()[..8], title);
                }
            }
        }
    }

    Ok(())
}
```

### Step 8.3: Register and add to CLI

### Step 8.4: Commit

```bash
git add crates/todoee-cli/src/commands/show.rs crates/todoee-cli/src/commands/diff.rs crates/todoee-cli/src/commands/mod.rs crates/todoee-cli/src/main.rs
git commit -m "feat(cli): add show and diff commands

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 9: Implement GC (Garbage Collection) Command

**Files:**
- Create: `crates/todoee-cli/src/commands/gc.rs`

### Step 9.1: Add methods to LocalDb for cleanup

Add to `crates/todoee-core/src/db/local.rs`:

```rust
/// Delete all completed todos older than N days
pub async fn delete_old_completed(&self, days: i64) -> Result<u64> {
    let cutoff = Utc::now() - chrono::Duration::days(days);
    let result = sqlx::query(
        "DELETE FROM todos WHERE is_completed = 1 AND completed_at < ?"
    )
    .bind(cutoff.to_rfc3339())
    .execute(&self.pool)
    .await?;
    Ok(result.rows_affected())
}

/// Get stats about the database
pub async fn get_stats(&self) -> Result<DbStats> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM todos")
        .fetch_one(&self.pool)
        .await?;

    let completed: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM todos WHERE is_completed = 1")
        .fetch_one(&self.pool)
        .await?;

    let pending: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM todos WHERE is_completed = 0")
        .fetch_one(&self.pool)
        .await?;

    let operations: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM operations")
        .fetch_one(&self.pool)
        .await?;

    let stashed: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stash")
        .fetch_one(&self.pool)
        .await?;

    Ok(DbStats {
        total_todos: total.0 as usize,
        completed_todos: completed.0 as usize,
        pending_todos: pending.0 as usize,
        operations: operations.0 as usize,
        stashed: stashed.0 as usize,
    })
}

pub struct DbStats {
    pub total_todos: usize,
    pub completed_todos: usize,
    pub pending_todos: usize,
    pub operations: usize,
    pub stashed: usize,
}
```

### Step 9.2: Create gc command

Create `crates/todoee-cli/src/commands/gc.rs`:

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb};

pub async fn gc(days: Option<i64>, dry_run: bool) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = LocalDb::new(&db_path).await.context("Failed to open database")?;

    let days = days.unwrap_or(30);

    // Get stats before
    let stats_before = db.get_stats().await?;

    println!("Database stats:");
    println!("  Total todos:     {}", stats_before.total_todos);
    println!("  Pending:         {}", stats_before.pending_todos);
    println!("  Completed:       {}", stats_before.completed_todos);
    println!("  Operations:      {}", stats_before.operations);
    println!("  Stashed:         {}", stats_before.stashed);
    println!();

    if dry_run {
        println!("Dry run - no changes made.");
        println!("Would clean up:");
        println!("  - Completed todos older than {} days", days);
        println!("  - Operation history older than {} days", days);
        return Ok(());
    }

    let deleted_todos = db.delete_old_completed(days).await?;
    let deleted_ops = db.clear_old_operations(days).await?;

    println!("Cleanup complete:");
    println!("  Deleted {} old completed todo(s)", deleted_todos);
    println!("  Deleted {} old operation(s)", deleted_ops);

    Ok(())
}
```

### Step 9.3: Register and add to CLI

Add to Commands:

```rust
/// Garbage collect old completed todos and operations
Gc {
    /// Delete items older than N days (default: 30)
    #[arg(short, long)]
    days: Option<i64>,
    /// Show what would be deleted without deleting
    #[arg(long)]
    dry_run: bool,
},
```

### Step 9.4: Commit

```bash
git add crates/todoee-core/src/db/local.rs crates/todoee-cli/src/commands/gc.rs crates/todoee-cli/src/commands/mod.rs crates/todoee-cli/src/main.rs
git commit -m "feat(cli): add gc command for cleaning old data

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 10: Add TUI Keybindings for New Features

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs`
- Modify: `crates/todoee-cli/src/tui/app.rs`

### Step 10.1: Add undo/redo methods to App

Add to `crates/todoee-cli/src/tui/app.rs`:

```rust
/// Undo the last operation
pub async fn undo(&mut self) -> Result<()> {
    if let Some(op) = self.db.get_last_undoable_operation().await? {
        match (op.operation_type, op.entity_type) {
            (OperationType::Create, EntityType::Todo) => {
                self.db.delete_todo(op.entity_id).await?;
                self.status_message = Some("Undone: todo deleted".to_string());
            }
            (OperationType::Delete, EntityType::Todo) => {
                if let Some(prev) = &op.previous_state {
                    let todo: Todo = serde_json::from_value(prev.clone())?;
                    self.db.create_todo(&todo).await?;
                    self.status_message = Some(format!("Undone: '{}' restored", todo.title));
                }
            }
            (OperationType::Complete, EntityType::Todo) => {
                if let Some(mut todo) = self.db.get_todo(op.entity_id).await? {
                    todo.mark_incomplete();
                    self.db.update_todo(&todo).await?;
                    self.status_message = Some(format!("Undone: '{}' marked incomplete", todo.title));
                }
            }
            _ => {}
        }
        self.db.mark_operation_undone(op.id).await?;
        self.refresh_todos().await;
    } else {
        self.status_message = Some("Nothing to undo".to_string());
    }
    Ok(())
}

/// Redo the last undone operation
pub async fn redo(&mut self) -> Result<()> {
    // Similar to undo but in reverse
    // ... implementation
    Ok(())
}

/// Stash the selected todo
pub async fn stash_selected(&mut self) -> Result<()> {
    if let Some(todo) = self.selected_todo() {
        self.db.stash_todo(todo.id, None).await?;
        self.status_message = Some(format!("Stashed: {}", todo.title));
        self.refresh_todos().await;
    }
    Ok(())
}

/// Pop from stash
pub async fn stash_pop(&mut self) -> Result<()> {
    if let Some(todo) = self.db.stash_pop().await? {
        self.status_message = Some(format!("Restored: {}", todo.title));
        self.refresh_todos().await;
    } else {
        self.status_message = Some("Stash is empty".to_string());
    }
    Ok(())
}
```

### Step 10.2: Add keybindings in handler.rs

Add to Todos view normal mode in `crates/todoee-cli/src/tui/handler.rs`:

```rust
KeyCode::Char('u') => {
    app.undo().await.ok();
}
KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    app.redo().await.ok();
}
KeyCode::Char('z') => {
    app.stash_selected().await.ok();
}
KeyCode::Char('Z') => {
    app.stash_pop().await.ok();
}
```

### Step 10.3: Update help modal

Update help text to include new keybindings.

### Step 10.4: Build and test

Run: `cargo build -p todoee-cli`
Expected: PASS

### Step 10.5: Commit

```bash
git add crates/todoee-cli/src/tui/handler.rs crates/todoee-cli/src/tui/app.rs
git commit -m "feat(tui): add keybindings for undo, redo, and stash

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Summary of New Commands

After implementation, users will have these git-like commands:

```bash
# History & Undo
todoee undo              # Undo last action
todoee redo              # Redo last undone action
todoee log -n 20         # View last 20 operations

# Smart Views
todoee head 10           # Last 10 created todos
todoee tail 5            # Oldest 5 todos
todoee upcoming 7        # Next 7 todos by due date
todoee overdue           # All overdue todos

# Stash
todoee stash push <id>   # Hide a todo temporarily
todoee stash pop         # Restore last stashed
todoee stash list        # Show all stashed
todoee stash clear       # Clear stash

# Batch Operations
todoee batch done <id1> <id2> ...    # Mark multiple done
todoee batch delete <id1> <id2> ...  # Delete multiple
todoee batch priority 3 <id1> <id2>  # Set priority for multiple

# Inspection
todoee show <id>         # Full todo details
todoee diff              # Show recent changes
todoee diff --hours 48   # Changes in last 48h

# Maintenance
todoee gc                # Clean old completed todos
todoee gc --days 7       # Clean older than 7 days
todoee gc --dry-run      # Preview what would be cleaned
```

## TUI Keybindings Added

| Key | Action |
|-----|--------|
| `u` | Undo last action |
| `Ctrl+r` | Redo |
| `z` | Stash selected todo |
| `Z` | Pop from stash |

---

## Execution Estimate

- **Tasks 1-4**: Core undo/redo infrastructure (~2h implementation)
- **Tasks 5-6**: View and stash commands (~1.5h)
- **Tasks 7-9**: Batch, show, diff, gc (~1.5h)
- **Task 10**: TUI integration (~1h)

**Total: ~6 hours of focused implementation**
