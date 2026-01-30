# Todoee Unified Implementation Roadmap

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform todoee into the fastest, most developer-friendly CLI todo app with git-like UX patterns and modern SaaS intelligence.

**Architecture:** Event-sourced core (enables undo/redo + analytics), memory-mapped indexes (speed), async I/O (responsiveness), plugin-ready hooks.

**Tech Stack:** Rust, SQLx/SQLite, Tantivy (FTS), tokio, ratatui, serde

---

## Guiding Principles

1. **Speed is a feature** - Every operation < 50ms
2. **Git muscle memory** - Familiar patterns for developers
3. **Intelligence without cloud** - Smart features, local-first
4. **Unix philosophy** - Composable, pipe-friendly
5. **Progressive disclosure** - Simple by default, powerful when needed

---

## Phase Overview

| Phase | Focus | Duration | Key Deliverables |
|-------|-------|----------|------------------|
| **1** | Foundation | 3-4 days | Operation history, undo/redo, speed baseline |
| **2** | Smart Views | 2-3 days | Head/tail/upcoming, fuzzy search, natural filters |
| **3** | Workflow | 3-4 days | Stash, batch ops, chains, templates |
| **4** | Intelligence | 4-5 days | Focus mode, insights, smart now, streaks |
| **5** | Views & UX | 3-4 days | Kanban, timeline, zen mode, review wizard |
| **6** | Integration | 3-4 days | API server, MCP, hooks, contexts |
| **7** | Polish | 2-3 days | Performance tuning, docs, packaging |

**Total: ~3-4 weeks**

---

# PHASE 1: FOUNDATION

> Goal: Event-sourced architecture that enables everything else

## Task 1.1: Operation History Infrastructure

**Why first:** Enables undo/redo, analytics, diff, log - foundational for many features.

**Files:**
- Modify: `crates/todoee-core/src/models.rs`
- Modify: `crates/todoee-core/src/db/local.rs`
- Modify: `crates/todoee-core/src/lib.rs`

### Step 1: Add Operation model to models.rs

```rust
use serde::{Deserialize, Serialize};

/// Represents a tracked operation for undo/redo and analytics
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
    Stash,
    Unstash,
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
            Self::Create => write!(f, "create"),
            Self::Update => write!(f, "update"),
            Self::Delete => write!(f, "delete"),
            Self::Complete => write!(f, "complete"),
            Self::Uncomplete => write!(f, "uncomplete"),
            Self::Stash => write!(f, "stash"),
            Self::Unstash => write!(f, "unstash"),
        }
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Todo => write!(f, "todo"),
            Self::Category => write!(f, "category"),
        }
    }
}
```

### Step 2: Add migrations to local.rs

Add in `run_migrations()`:

```rust
// Operations table for undo/redo and analytics
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

// Stash table for temporarily hiding todos
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

### Step 3: Add CRUD methods for operations

Add to `impl LocalDb`:

```rust
// ============ OPERATION HISTORY ============

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

pub async fn get_last_undoable_operation(&self) -> Result<Option<Operation>> {
    let row = sqlx::query(
        "SELECT * FROM operations WHERE undone = 0 ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_optional(&self.pool)
    .await?;
    row.map(|r| self.row_to_operation(&r)).transpose()
}

pub async fn get_last_redoable_operation(&self) -> Result<Option<Operation>> {
    let row = sqlx::query(
        "SELECT * FROM operations WHERE undone = 1 ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_optional(&self.pool)
    .await?;
    row.map(|r| self.row_to_operation(&r)).transpose()
}

pub async fn mark_operation_undone(&self, id: Uuid) -> Result<()> {
    sqlx::query("UPDATE operations SET undone = 1 WHERE id = ?")
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
    Ok(())
}

pub async fn mark_operation_redone(&self, id: Uuid) -> Result<()> {
    sqlx::query("UPDATE operations SET undone = 0 WHERE id = ?")
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
    Ok(())
}

pub async fn list_operations(&self, limit: usize) -> Result<Vec<Operation>> {
    let rows = sqlx::query(
        "SELECT * FROM operations ORDER BY created_at DESC LIMIT ?"
    )
    .bind(limit as i64)
    .fetch_all(&self.pool)
    .await?;
    rows.iter().map(|r| self.row_to_operation(r)).collect()
}

pub async fn list_operations_since(&self, since: DateTime<Utc>) -> Result<Vec<Operation>> {
    let rows = sqlx::query(
        "SELECT * FROM operations WHERE created_at > ? AND undone = 0 ORDER BY created_at DESC"
    )
    .bind(since.to_rfc3339())
    .fetch_all(&self.pool)
    .await?;
    rows.iter().map(|r| self.row_to_operation(r)).collect()
}

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
        "stash" => OperationType::Stash,
        "unstash" => OperationType::Unstash,
        _ => return Err(anyhow::anyhow!("Unknown operation type")),
    };

    let entity_type = match entity_type_str.as_str() {
        "todo" => EntityType::Todo,
        "category" => EntityType::Category,
        _ => return Err(anyhow::anyhow!("Unknown entity type")),
    };

    let prev: Option<String> = row.get("previous_state");
    let new: Option<String> = row.get("new_state");

    Ok(Operation {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        operation_type,
        entity_type,
        entity_id: Uuid::parse_str(row.get::<&str, _>("entity_id"))?,
        previous_state: prev.and_then(|s| serde_json::from_str(&s).ok()),
        new_state: new.and_then(|s| serde_json::from_str(&s).ok()),
        created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
        undone: row.get::<i32, _>("undone") != 0,
    })
}
```

### Step 4: Export from lib.rs

```rust
pub use models::{Operation, OperationType, EntityType};
```

### Step 5: Build and test

```bash
cargo build -p todoee-core
cargo test -p todoee-core
```

### Step 6: Commit

```bash
git add -A && git commit -m "feat(core): add operation history infrastructure for undo/analytics

- Add Operation, OperationType, EntityType models
- Add operations table with indexes
- Add stash table for todo stashing
- Add CRUD methods for operation tracking

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 1.2: Instrument Existing Commands

**Files:**
- Modify: `crates/todoee-cli/src/commands/add.rs`
- Modify: `crates/todoee-cli/src/commands/done.rs`
- Modify: `crates/todoee-cli/src/commands/delete.rs`
- Modify: `crates/todoee-cli/src/commands/edit.rs`

### Step 1: Update add.rs

After `db.create_todo(&todo).await?;`:

```rust
use todoee_core::{Operation, OperationType, EntityType};

// Record for undo
let op = Operation::new(
    OperationType::Create,
    EntityType::Todo,
    todo.id,
    None,
    Some(serde_json::to_value(&todo)?),
);
db.record_operation(&op).await?;
```

### Step 2: Update done.rs

Before update, save state:

```rust
let prev_state = serde_json::to_value(&todo)?;
```

After update:

```rust
let op = Operation::new(
    if todo.is_completed { OperationType::Complete } else { OperationType::Uncomplete },
    EntityType::Todo,
    todo.id,
    Some(prev_state),
    None,
);
db.record_operation(&op).await?;
```

### Step 3: Update delete.rs

Before deletion:

```rust
let todo = db.get_todo(todo_id).await?.ok_or_else(|| anyhow::anyhow!("Not found"))?;
let op = Operation::new(
    OperationType::Delete,
    EntityType::Todo,
    todo.id,
    Some(serde_json::to_value(&todo)?),
    None,
);
db.record_operation(&op).await?;
```

### Step 4: Update edit.rs

Before and after:

```rust
let prev_state = serde_json::to_value(&todo)?;
// ... apply edits ...
let op = Operation::new(
    OperationType::Update,
    EntityType::Todo,
    todo.id,
    Some(prev_state),
    Some(serde_json::to_value(&todo)?),
);
db.record_operation(&op).await?;
```

### Step 5: Build and test

```bash
cargo build -p todoee-cli
```

### Step 6: Commit

```bash
git add -A && git commit -m "feat(cli): instrument commands to record operations

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 1.3: Implement Undo/Redo Commands

**Files:**
- Create: `crates/todoee-cli/src/commands/undo.rs`
- Create: `crates/todoee-cli/src/commands/redo.rs`
- Modify: `crates/todoee-cli/src/commands/mod.rs`
- Modify: `crates/todoee-cli/src/main.rs`

### Step 1: Create undo.rs

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb, OperationType, EntityType, Todo};

pub async fn undo() -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();
    std::fs::create_dir_all(db_path.parent().unwrap())?;
    let db = LocalDb::new(&db_path).await?;

    let Some(op) = db.get_last_undoable_operation().await? else {
        println!("Nothing to undo");
        return Ok(());
    };

    match (op.operation_type, op.entity_type) {
        (OperationType::Create, EntityType::Todo) => {
            db.delete_todo(op.entity_id).await?;
            let title = op.new_state.as_ref()
                .and_then(|s| s.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("todo");
            println!("↩ Undone create: deleted \"{}\"", title);
        }
        (OperationType::Delete, EntityType::Todo) => {
            if let Some(prev) = &op.previous_state {
                let todo: Todo = serde_json::from_value(prev.clone())?;
                db.create_todo(&todo).await?;
                println!("↩ Undone delete: restored \"{}\"", todo.title);
            }
        }
        (OperationType::Update, EntityType::Todo) => {
            if let Some(prev) = &op.previous_state {
                let todo: Todo = serde_json::from_value(prev.clone())?;
                db.update_todo(&todo).await?;
                println!("↩ Undone edit: reverted \"{}\"", todo.title);
            }
        }
        (OperationType::Complete, EntityType::Todo) => {
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_incomplete();
                db.update_todo(&todo).await?;
                println!("↩ Undone complete: \"{}\" is pending again", todo.title);
            }
        }
        (OperationType::Uncomplete, EntityType::Todo) => {
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_complete();
                db.update_todo(&todo).await?;
                println!("↩ Undone uncomplete: \"{}\" is done again", todo.title);
            }
        }
        (OperationType::Stash, EntityType::Todo) => {
            // Unstash it
            if let Some(prev) = &op.previous_state {
                let todo: Todo = serde_json::from_value(prev.clone())?;
                db.create_todo(&todo).await?;
                println!("↩ Undone stash: \"{}\" restored", todo.title);
            }
        }
        _ => println!("Cannot undo this operation type"),
    }

    db.mark_operation_undone(op.id).await?;
    Ok(())
}
```

### Step 2: Create redo.rs

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb, OperationType, EntityType, Todo};

pub async fn redo() -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path();
    std::fs::create_dir_all(db_path.parent().unwrap())?;
    let db = LocalDb::new(&db_path).await?;

    let Some(op) = db.get_last_redoable_operation().await? else {
        println!("Nothing to redo");
        return Ok(());
    };

    match (op.operation_type, op.entity_type) {
        (OperationType::Create, EntityType::Todo) => {
            if let Some(new) = &op.new_state {
                let todo: Todo = serde_json::from_value(new.clone())?;
                db.create_todo(&todo).await?;
                println!("↪ Redone: created \"{}\"", todo.title);
            }
        }
        (OperationType::Delete, EntityType::Todo) => {
            db.delete_todo(op.entity_id).await?;
            let title = op.previous_state.as_ref()
                .and_then(|s| s.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("todo");
            println!("↪ Redone: deleted \"{}\"", title);
        }
        (OperationType::Update, EntityType::Todo) => {
            if let Some(new) = &op.new_state {
                let todo: Todo = serde_json::from_value(new.clone())?;
                db.update_todo(&todo).await?;
                println!("↪ Redone: updated \"{}\"", todo.title);
            }
        }
        (OperationType::Complete, EntityType::Todo) => {
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_complete();
                db.update_todo(&todo).await?;
                println!("↪ Redone: \"{}\" marked complete", todo.title);
            }
        }
        (OperationType::Uncomplete, EntityType::Todo) => {
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_incomplete();
                db.update_todo(&todo).await?;
                println!("↪ Redone: \"{}\" marked incomplete", todo.title);
            }
        }
        _ => println!("Cannot redo this operation type"),
    }

    db.mark_operation_redone(op.id).await?;
    Ok(())
}
```

### Step 3: Register in mod.rs and main.rs

Add to Commands enum:

```rust
/// Undo the last operation
Undo,
/// Redo the last undone operation
Redo,
```

### Step 4: Build and test

```bash
cargo build -p todoee-cli
./target/debug/todoee add "test undo"
./target/debug/todoee undo
./target/debug/todoee redo
```

### Step 5: Commit

```bash
git add -A && git commit -m "feat(cli): add undo and redo commands

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 1.4: Implement Log and Diff Commands

**Files:**
- Create: `crates/todoee-cli/src/commands/log.rs`
- Create: `crates/todoee-cli/src/commands/diff.rs`

### Step 1: Create log.rs

```rust
use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use todoee_core::{Config, LocalDb, OperationType};

pub async fn log(limit: Option<usize>, oneline: bool) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db = LocalDb::new(&config.local_db_path()).await?;

    let operations = db.list_operations(limit.unwrap_or(10)).await?;

    if operations.is_empty() {
        println!("No operations recorded yet.");
        return Ok(());
    }

    for op in operations {
        let time = Local.from_utc_datetime(&op.created_at.naive_utc());
        let short_id = &op.id.to_string()[..7];
        let entity_short = &op.entity_id.to_string()[..8];

        let title = op.new_state.as_ref()
            .or(op.previous_state.as_ref())
            .and_then(|s| s.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or("?");

        let status = if op.undone { " (undone)" } else { "" };

        if oneline {
            println!(
                "\x1b[33m{}\x1b[0m {} {} {}: {}{}",
                short_id,
                time.format("%m-%d %H:%M"),
                op.operation_type,
                entity_short,
                truncate(title, 40),
                status
            );
        } else {
            println!("\x1b[33mop {}\x1b[0m{}", short_id, status);
            println!("Date:   {}", time.format("%Y-%m-%d %H:%M:%S"));
            println!("Action: {} {}", op.operation_type, op.entity_type);
            println!("Entity: {}", op.entity_id);
            println!("Title:  {}", title);
            println!();
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}...", &s[..max-3]) }
}
```

### Step 2: Create diff.rs

```rust
use anyhow::{Context, Result};
use chrono::{Duration, Local, TimeZone, Utc};
use todoee_core::{Config, LocalDb, OperationType};

pub async fn diff(hours: Option<i64>) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db = LocalDb::new(&config.local_db_path()).await?;

    let hours = hours.unwrap_or(24);
    let since = Utc::now() - Duration::hours(hours);
    let operations = db.list_operations_since(since).await?;

    if operations.is_empty() {
        println!("No changes in the last {} hours.", hours);
        return Ok(());
    }

    println!("Changes in the last {} hours:\n", hours);

    let mut creates = 0;
    let mut updates = 0;
    let mut deletes = 0;
    let mut completes = 0;

    for op in &operations {
        let time = Local.from_utc_datetime(&op.created_at.naive_utc());
        let short_id = &op.entity_id.to_string()[..8];

        let title = op.new_state.as_ref()
            .or(op.previous_state.as_ref())
            .and_then(|s| s.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or("?");

        match op.operation_type {
            OperationType::Create => {
                println!("\x1b[32m+ {}\x1b[0m {} {}", time.format("%H:%M"), short_id, title);
                creates += 1;
            }
            OperationType::Delete => {
                println!("\x1b[31m- {}\x1b[0m {} {}", time.format("%H:%M"), short_id, title);
                deletes += 1;
            }
            OperationType::Update => {
                println!("\x1b[33m~ {}\x1b[0m {} {}", time.format("%H:%M"), short_id, title);
                updates += 1;
            }
            OperationType::Complete => {
                println!("\x1b[32m✓ {}\x1b[0m {} {}", time.format("%H:%M"), short_id, title);
                completes += 1;
            }
            OperationType::Uncomplete => {
                println!("\x1b[33m○ {}\x1b[0m {} {}", time.format("%H:%M"), short_id, title);
            }
            _ => {}
        }
    }

    println!("\n─────────────────────────────────");
    println!(
        "\x1b[32m+{} created\x1b[0m  \x1b[33m~{} updated\x1b[0m  \x1b[31m-{} deleted\x1b[0m  \x1b[32m✓{} completed\x1b[0m",
        creates, updates, deletes, completes
    );

    Ok(())
}
```

### Step 3: Register and commit

```bash
git add -A && git commit -m "feat(cli): add log and diff commands for history viewing

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

# PHASE 2: SMART VIEWS

> Goal: Git-like and SaaS-like ways to slice and view todos

## Task 2.1: Head/Tail/Upcoming/Overdue Commands

**Files:**
- Modify: `crates/todoee-core/src/db/local.rs` (add query methods)
- Create: `crates/todoee-cli/src/commands/head.rs`
- Create: `crates/todoee-cli/src/commands/upcoming.rs`

### Step 1: Add query methods to LocalDb

```rust
pub async fn list_todos_head(&self, limit: usize, include_completed: bool) -> Result<Vec<Todo>> {
    let query = if include_completed {
        "SELECT * FROM todos ORDER BY created_at DESC LIMIT ?"
    } else {
        "SELECT * FROM todos WHERE is_completed = 0 ORDER BY created_at DESC LIMIT ?"
    };
    let rows = sqlx::query(query).bind(limit as i64).fetch_all(&self.pool).await?;
    rows.iter().map(|r| self.row_to_todo(r)).collect()
}

pub async fn list_todos_tail(&self, limit: usize, include_completed: bool) -> Result<Vec<Todo>> {
    let query = if include_completed {
        "SELECT * FROM todos ORDER BY created_at ASC LIMIT ?"
    } else {
        "SELECT * FROM todos WHERE is_completed = 0 ORDER BY created_at ASC LIMIT ?"
    };
    let rows = sqlx::query(query).bind(limit as i64).fetch_all(&self.pool).await?;
    rows.iter().map(|r| self.row_to_todo(r)).collect()
}

pub async fn list_todos_upcoming(&self, limit: usize) -> Result<Vec<Todo>> {
    let now = Utc::now().to_rfc3339();
    let rows = sqlx::query(
        "SELECT * FROM todos WHERE is_completed = 0 AND due_date IS NOT NULL AND due_date >= ? ORDER BY due_date ASC LIMIT ?"
    )
    .bind(&now)
    .bind(limit as i64)
    .fetch_all(&self.pool)
    .await?;
    rows.iter().map(|r| self.row_to_todo(r)).collect()
}

pub async fn list_todos_overdue(&self) -> Result<Vec<Todo>> {
    let now = Utc::now().to_rfc3339();
    let rows = sqlx::query(
        "SELECT * FROM todos WHERE is_completed = 0 AND due_date IS NOT NULL AND due_date < ? ORDER BY due_date ASC"
    )
    .bind(&now)
    .fetch_all(&self.pool)
    .await?;
    rows.iter().map(|r| self.row_to_todo(r)).collect()
}

pub async fn search_todos(&self, query: &str) -> Result<Vec<Todo>> {
    let pattern = format!("%{}%", query);
    let rows = sqlx::query(
        "SELECT * FROM todos WHERE title LIKE ? OR description LIKE ? ORDER BY created_at DESC LIMIT 50"
    )
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(&self.pool)
    .await?;
    rows.iter().map(|r| self.row_to_todo(r)).collect()
}
```

### Step 2: Create head.rs (also handles tail)

```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb, Priority};
use chrono::{Local, TimeZone, Utc};

pub async fn head(count: usize, all: bool) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;
    let todos = db.list_todos_head(count, all).await?;

    if todos.is_empty() {
        println!("No todos.");
        return Ok(());
    }

    println!("Last {} todos:\n", todos.len());
    print_todos(&todos);
    Ok(())
}

pub async fn tail(count: usize, all: bool) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;
    let todos = db.list_todos_tail(count, all).await?;

    if todos.is_empty() {
        println!("No todos.");
        return Ok(());
    }

    println!("Oldest {} todos:\n", todos.len());
    print_todos(&todos);
    Ok(())
}

fn print_todos(todos: &[todoee_core::Todo]) {
    for todo in todos {
        let check = if todo.is_completed { "\x1b[32m✓\x1b[0m" } else { "○" };
        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };
        let id = &todo.id.to_string()[..8];
        let age = format_age(todo.created_at);

        println!("{} {} \x1b[90m{}\x1b[0m {} \x1b[90m({})\x1b[0m", check, pri, id, todo.title, age);
    }
}

fn format_age(dt: chrono::DateTime<Utc>) -> String {
    let diff = Utc::now().signed_duration_since(dt);
    let days = diff.num_days();
    if days > 0 { format!("{}d ago", days) }
    else {
        let hours = diff.num_hours();
        if hours > 0 { format!("{}h ago", hours) }
        else { "just now".to_string() }
    }
}
```

### Step 3: Create upcoming.rs

```rust
use anyhow::Result;
use chrono::{Local, TimeZone, Utc};
use todoee_core::{Config, LocalDb, Priority};

pub async fn upcoming(count: usize) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;
    let todos = db.list_todos_upcoming(count).await?;

    if todos.is_empty() {
        println!("No upcoming todos with due dates.");
        return Ok(());
    }

    println!("Next {} upcoming:\n", todos.len());

    for todo in todos {
        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };
        let id = &todo.id.to_string()[..8];
        let due = todo.due_date.map(|d| {
            let local = Local.from_utc_datetime(&d.naive_utc());
            let diff = d.signed_duration_since(Utc::now());
            let days = diff.num_days();
            let label = if days == 0 { "TODAY".to_string() }
                else if days == 1 { "tomorrow".to_string() }
                else { format!("in {}d", days) };
            format!("{} ({})", local.format("%m-%d %H:%M"), label)
        }).unwrap_or_default();

        println!("{} \x1b[90m{}\x1b[0m {} \x1b[36m[{}]\x1b[0m", pri, id, todo.title, due);
    }
    Ok(())
}

pub async fn overdue() -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;
    let todos = db.list_todos_overdue().await?;

    if todos.is_empty() {
        println!("\x1b[32mNo overdue todos!\x1b[0m");
        return Ok(());
    }

    println!("\x1b[31m{} overdue:\x1b[0m\n", todos.len());

    for todo in todos {
        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };
        let id = &todo.id.to_string()[..8];
        let overdue_by = todo.due_date.map(|d| {
            let diff = Utc::now().signed_duration_since(d);
            let days = diff.num_days();
            if days > 0 { format!("{} days overdue", days) }
            else { format!("{} hours overdue", diff.num_hours()) }
        }).unwrap_or_default();

        println!("{} \x1b[90m{}\x1b[0m {} \x1b[31m[{}]\x1b[0m", pri, id, todo.title, overdue_by);
    }
    Ok(())
}
```

### Step 4: Commit

```bash
git add -A && git commit -m "feat(cli): add head, tail, upcoming, overdue commands

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2.2: Fuzzy Search Command

**Files:**
- Create: `crates/todoee-cli/src/commands/search.rs`

### Step 1: Create search.rs with fuzzy matching

```rust
use anyhow::Result;
use todoee_core::{Config, LocalDb, Priority, Todo};

pub async fn search(query: &str, interactive: bool) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;

    // Get all todos for fuzzy matching
    let all_todos = db.list_todos(true).await?;

    // Fuzzy match
    let query_lower = query.to_lowercase();
    let mut matches: Vec<(&Todo, i32)> = all_todos.iter()
        .filter_map(|todo| {
            let score = fuzzy_score(&todo.title.to_lowercase(), &query_lower);
            if score > 0 { Some((todo, score)) } else { None }
        })
        .collect();

    // Sort by score descending
    matches.sort_by(|a, b| b.1.cmp(&a.1));

    if matches.is_empty() {
        println!("No matches for \"{}\"", query);
        return Ok(());
    }

    println!("Found {} matches for \"{}\":\n", matches.len().min(20), query);

    for (todo, _score) in matches.iter().take(20) {
        let check = if todo.is_completed { "\x1b[32m✓\x1b[0m" } else { "○" };
        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };
        let id = &todo.id.to_string()[..8];

        // Highlight matching parts
        let highlighted = highlight_match(&todo.title, query);
        println!("{} {} \x1b[90m{}\x1b[0m {}", check, pri, id, highlighted);
    }

    Ok(())
}

/// Simple fuzzy scoring - higher is better match
fn fuzzy_score(haystack: &str, needle: &str) -> i32 {
    if haystack.contains(needle) {
        return 100; // Exact substring match
    }

    let mut score = 0;
    let mut haystack_idx = 0;
    let hay_chars: Vec<char> = haystack.chars().collect();

    for needle_char in needle.chars() {
        while haystack_idx < hay_chars.len() {
            if hay_chars[haystack_idx] == needle_char {
                score += 10;
                // Bonus for consecutive matches
                if haystack_idx > 0 && hay_chars[haystack_idx - 1] == needle.chars().nth(needle.chars().count() - 2).unwrap_or(' ') {
                    score += 5;
                }
                haystack_idx += 1;
                break;
            }
            haystack_idx += 1;
        }
    }

    // Bonus for matching at word boundaries
    if haystack.split_whitespace().any(|word| word.starts_with(needle)) {
        score += 20;
    }

    score
}

fn highlight_match(text: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    if let Some(pos) = text_lower.find(&query_lower) {
        let before = &text[..pos];
        let matched = &text[pos..pos + query.len()];
        let after = &text[pos + query.len()..];
        format!("{}\x1b[1;33m{}\x1b[0m{}", before, matched, after)
    } else {
        text.to_string()
    }
}
```

### Step 2: Register in CLI

Add to Commands:

```rust
/// Search todos (fuzzy matching)
#[command(name = "/")]
Search {
    /// Search query
    query: String,
},
```

Or as an alias:

```rust
/// Search todos (fuzzy matching)
Search {
    /// Search query
    query: String,
},
```

### Step 3: Commit

```bash
git add -A && git commit -m "feat(cli): add fuzzy search command

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2.3: Show Command (Detailed View)

**Files:**
- Create: `crates/todoee-cli/src/commands/show.rs`

```rust
use anyhow::Result;
use chrono::{Local, TimeZone};
use todoee_core::{Config, LocalDb, Priority, SyncStatus};

pub async fn show(id: &str) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;
    let todos = db.list_todos(true).await?;

    let matching: Vec<_> = todos.iter()
        .filter(|t| t.id.to_string().starts_with(id))
        .collect();

    match matching.len() {
        0 => println!("No todo found with ID starting with '{}'", id),
        1 => {
            let todo = matching[0];
            let categories = db.list_categories().await?;
            let cat_name = todo.category_id
                .and_then(|cid| categories.iter().find(|c| c.id == cid))
                .map(|c| c.name.as_str())
                .unwrap_or("None");

            println!("┌─────────────────────────────────────────────────┐");
            println!("│ \x1b[1m{}\x1b[0m", truncate(&todo.title, 47));
            println!("├─────────────────────────────────────────────────┤");
            println!("│ ID:         {}", todo.id);
            println!("│ Status:     {}", if todo.is_completed { "\x1b[32mCompleted\x1b[0m" } else { "\x1b[33mPending\x1b[0m" });
            println!("│ Priority:   {}", match todo.priority {
                Priority::High => "\x1b[31mHigh (!!!)\x1b[0m",
                Priority::Medium => "\x1b[33mMedium (!!)\x1b[0m",
                Priority::Low => "Low (!)",
            });
            println!("│ Category:   {}", cat_name);

            if let Some(desc) = &todo.description {
                println!("│ Description:");
                for line in desc.lines() {
                    println!("│   {}", line);
                }
            }

            if let Some(due) = todo.due_date {
                let local = Local.from_utc_datetime(&due.naive_utc());
                println!("│ Due:        {}", local.format("%Y-%m-%d %H:%M"));
            }

            if let Some(reminder) = todo.reminder_at {
                let local = Local.from_utc_datetime(&reminder.naive_utc());
                println!("│ Reminder:   {}", local.format("%Y-%m-%d %H:%M"));
            }

            if let Some(completed) = todo.completed_at {
                let local = Local.from_utc_datetime(&completed.naive_utc());
                println!("│ Completed:  {}", local.format("%Y-%m-%d %H:%M"));
            }

            let created = Local.from_utc_datetime(&todo.created_at.naive_utc());
            let updated = Local.from_utc_datetime(&todo.updated_at.naive_utc());
            println!("│ Created:    {}", created.format("%Y-%m-%d %H:%M"));
            println!("│ Updated:    {}", updated.format("%Y-%m-%d %H:%M"));
            println!("│ Sync:       {:?}", todo.sync_status);
            println!("└─────────────────────────────────────────────────┘");
        }
        _ => {
            println!("Multiple matches for '{}'. Be more specific:", id);
            for t in matching {
                println!("  {} - {}", &t.id.to_string()[..8], t.title);
            }
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}...", &s[..max-3]) }
}
```

---

# PHASE 3: WORKFLOW COMMANDS

## Task 3.1: Stash Commands

**Files:**
- Modify: `crates/todoee-core/src/db/local.rs`
- Create: `crates/todoee-cli/src/commands/stash.rs`

### Step 1: Add stash methods to LocalDb

```rust
pub async fn stash_todo(&self, todo_id: Uuid, message: Option<&str>) -> Result<Todo> {
    let todo = self.get_todo(todo_id).await?
        .ok_or_else(|| anyhow::anyhow!("Todo not found"))?;

    sqlx::query(
        "INSERT INTO stash (id, todo_json, stashed_at, message) VALUES (?, ?, ?, ?)"
    )
    .bind(todo_id.to_string())
    .bind(serde_json::to_string(&todo)?)
    .bind(Utc::now().to_rfc3339())
    .bind(message)
    .execute(&self.pool)
    .await?;

    self.delete_todo(todo_id).await?;
    Ok(todo)
}

pub async fn stash_pop(&self) -> Result<Option<Todo>> {
    let row = sqlx::query("SELECT * FROM stash ORDER BY stashed_at DESC LIMIT 1")
        .fetch_optional(&self.pool)
        .await?;

    if let Some(row) = row {
        use sqlx::Row;
        let id: String = row.get("id");
        let json: String = row.get("todo_json");
        let todo: Todo = serde_json::from_str(&json)?;

        sqlx::query("DELETE FROM stash WHERE id = ?").bind(&id).execute(&self.pool).await?;
        self.create_todo(&todo).await?;
        Ok(Some(todo))
    } else {
        Ok(None)
    }
}

pub async fn stash_list(&self) -> Result<Vec<(Todo, String, Option<String>)>> {
    let rows = sqlx::query("SELECT * FROM stash ORDER BY stashed_at DESC")
        .fetch_all(&self.pool)
        .await?;

    let mut result = Vec::new();
    for row in rows {
        use sqlx::Row;
        let json: String = row.get("todo_json");
        let at: String = row.get("stashed_at");
        let msg: Option<String> = row.get("message");
        let todo: Todo = serde_json::from_str(&json)?;
        result.push((todo, at, msg));
    }
    Ok(result)
}

pub async fn stash_clear(&self) -> Result<u64> {
    let result = sqlx::query("DELETE FROM stash").execute(&self.pool).await?;
    Ok(result.rows_affected())
}
```

### Step 2: Create stash.rs

```rust
use anyhow::Result;
use clap::Subcommand;
use todoee_core::{Config, LocalDb, Operation, OperationType, EntityType};

#[derive(Subcommand, Clone)]
pub enum StashCommand {
    /// Stash a todo by ID
    Push {
        id: String,
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Restore the most recently stashed todo
    Pop,
    /// List all stashed todos
    List,
    /// Clear all stashed todos
    Clear,
}

pub async fn stash(cmd: StashCommand) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;

    match cmd {
        StashCommand::Push { id, message } => {
            let todos = db.list_todos(false).await?;
            let matching: Vec<_> = todos.iter()
                .filter(|t| t.id.to_string().starts_with(&id))
                .collect();

            match matching.len() {
                0 => println!("No todo found with ID '{}'", id),
                1 => {
                    let todo = db.stash_todo(matching[0].id, message.as_deref()).await?;

                    // Record operation
                    let op = Operation::new(
                        OperationType::Stash,
                        EntityType::Todo,
                        todo.id,
                        Some(serde_json::to_value(&todo)?),
                        None,
                    );
                    db.record_operation(&op).await?;

                    let msg = message.map(|m| format!(": {}", m)).unwrap_or_default();
                    println!("Stashed{}: {}", msg, todo.title);
                }
                _ => {
                    println!("Multiple matches. Be more specific:");
                    for t in matching {
                        println!("  {} - {}", &t.id.to_string()[..8], t.title);
                    }
                }
            }
        }
        StashCommand::Pop => {
            match db.stash_pop().await? {
                Some(todo) => {
                    let op = Operation::new(
                        OperationType::Unstash,
                        EntityType::Todo,
                        todo.id,
                        None,
                        Some(serde_json::to_value(&todo)?),
                    );
                    db.record_operation(&op).await?;
                    println!("Restored: {}", todo.title);
                }
                None => println!("Stash is empty"),
            }
        }
        StashCommand::List => {
            let stashed = db.stash_list().await?;
            if stashed.is_empty() {
                println!("Stash is empty");
            } else {
                println!("Stashed todos:\n");
                for (i, (todo, _at, msg)) in stashed.iter().enumerate() {
                    let msg_str = msg.as_ref().map(|m| format!(": {}", m)).unwrap_or_default();
                    println!("stash@{{{}}}{}", i, msg_str);
                    println!("  {}", todo.title);
                }
            }
        }
        StashCommand::Clear => {
            let count = db.stash_clear().await?;
            println!("Cleared {} stashed todo(s)", count);
        }
    }

    Ok(())
}
```

---

## Task 3.2: Batch Operations

**Files:**
- Create: `crates/todoee-cli/src/commands/batch.rs`

```rust
use anyhow::Result;
use clap::Subcommand;
use todoee_core::{Config, LocalDb, Operation, OperationType, EntityType, Priority};

#[derive(Subcommand, Clone)]
pub enum BatchCommand {
    /// Mark multiple todos as done
    Done {
        /// Todo IDs (or prefixes)
        ids: Vec<String>,
    },
    /// Delete multiple todos
    Delete {
        /// Todo IDs (or prefixes)
        ids: Vec<String>,
    },
    /// Set priority for multiple todos
    Priority {
        /// Priority level (1=low, 2=medium, 3=high)
        level: u8,
        /// Todo IDs (or prefixes)
        ids: Vec<String>,
    },
}

pub async fn batch(cmd: BatchCommand) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;
    let all_todos = db.list_todos(true).await?;

    match cmd {
        BatchCommand::Done { ids } => {
            let mut count = 0;
            for id in &ids {
                if let Some(todo) = all_todos.iter().find(|t| t.id.to_string().starts_with(id)) {
                    if !todo.is_completed {
                        let mut updated = todo.clone();
                        let prev = serde_json::to_value(&updated)?;
                        updated.mark_complete();
                        db.update_todo(&updated).await?;

                        let op = Operation::new(OperationType::Complete, EntityType::Todo, todo.id, Some(prev), None);
                        db.record_operation(&op).await?;
                        count += 1;
                    }
                }
            }
            println!("Marked {} todo(s) as done", count);
        }
        BatchCommand::Delete { ids } => {
            let mut count = 0;
            for id in &ids {
                if let Some(todo) = all_todos.iter().find(|t| t.id.to_string().starts_with(id)) {
                    let op = Operation::new(OperationType::Delete, EntityType::Todo, todo.id, Some(serde_json::to_value(todo)?), None);
                    db.record_operation(&op).await?;
                    db.delete_todo(todo.id).await?;
                    count += 1;
                }
            }
            println!("Deleted {} todo(s)", count);
        }
        BatchCommand::Priority { level, ids } => {
            let priority = match level {
                1 => Priority::Low,
                2 => Priority::Medium,
                3 => Priority::High,
                _ => {
                    println!("Priority must be 1, 2, or 3");
                    return Ok(());
                }
            };

            let mut count = 0;
            for id in &ids {
                if let Some(todo) = all_todos.iter().find(|t| t.id.to_string().starts_with(id)) {
                    let mut updated = todo.clone();
                    let prev = serde_json::to_value(&updated)?;
                    updated.priority = priority;
                    db.update_todo(&updated).await?;

                    let op = Operation::new(OperationType::Update, EntityType::Todo, todo.id, Some(prev), Some(serde_json::to_value(&updated)?));
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

---

## Task 3.3: GC (Garbage Collection)

**Files:**
- Create: `crates/todoee-cli/src/commands/gc.rs`

```rust
use anyhow::Result;
use todoee_core::{Config, LocalDb};

pub async fn gc(days: Option<i64>, dry_run: bool) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;
    let days = days.unwrap_or(30);

    // Get counts
    let todos: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM todos").fetch_one(&db.pool).await?;
    let completed: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM todos WHERE is_completed = 1").fetch_one(&db.pool).await?;
    let ops: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM operations").fetch_one(&db.pool).await?;
    let stashed: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stash").fetch_one(&db.pool).await?;

    println!("Database stats:");
    println!("  Total todos:  {}", todos.0);
    println!("  Completed:    {}", completed.0);
    println!("  Operations:   {}", ops.0);
    println!("  Stashed:      {}", stashed.0);
    println!();

    if dry_run {
        println!("Dry run - would clean items older than {} days", days);
        return Ok(());
    }

    let deleted_ops = db.clear_old_operations(days).await?;

    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let result = sqlx::query("DELETE FROM todos WHERE is_completed = 1 AND completed_at < ?")
        .bind(cutoff.to_rfc3339())
        .execute(&db.pool)
        .await?;

    println!("Cleanup complete:");
    println!("  Deleted {} old operation(s)", deleted_ops);
    println!("  Deleted {} old completed todo(s)", result.rows_affected());

    Ok(())
}
```

---

# PHASE 4: INTELLIGENCE FEATURES

## Task 4.1: Focus Mode (Pomodoro)

**Files:**
- Create: `crates/todoee-cli/src/commands/focus.rs`

```rust
use anyhow::Result;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use todoee_core::{Config, LocalDb};
use crossterm::{cursor, terminal, ExecutableCommand};

pub async fn focus(id: Option<String>, duration_mins: u32) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;

    let todo = if let Some(id) = id {
        let todos = db.list_todos(false).await?;
        todos.into_iter()
            .find(|t| t.id.to_string().starts_with(&id))
            .ok_or_else(|| anyhow::anyhow!("Todo not found"))?
    } else {
        // Pick the highest priority upcoming todo
        let todos = db.list_todos(false).await?;
        todos.into_iter()
            .max_by_key(|t| t.priority as u8)
            .ok_or_else(|| anyhow::anyhow!("No todos to focus on"))?
    };

    let duration = Duration::from_secs(duration_mins as u64 * 60);
    let start = Instant::now();

    // Clear screen and show focus UI
    let mut stdout = io::stdout();
    stdout.execute(terminal::Clear(terminal::ClearType::All))?;

    println!("\n\x1b[1;36m╭──────────────────────────────────────────────────╮\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m               \x1b[1mFOCUS MODE\x1b[0m                        \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m├──────────────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m                                                  \x1b[1;36m│\x1b[0m");
    println!("\x1b[1;36m│\x1b[0m  \x1b[1m{}\x1b[0m", truncate(&todo.title, 44));
    println!("\x1b[1;36m│\x1b[0m                                                  \x1b[1;36m│\x1b[0m");

    // Timer loop
    loop {
        let elapsed = start.elapsed();
        if elapsed >= duration {
            break;
        }

        let remaining = duration - elapsed;
        let mins = remaining.as_secs() / 60;
        let secs = remaining.as_secs() % 60;

        let progress = elapsed.as_secs_f64() / duration.as_secs_f64();
        let bar_width = 30;
        let filled = (progress * bar_width as f64) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);

        // Update timer line
        stdout.execute(cursor::MoveTo(0, 6))?;
        println!("\x1b[1;36m│\x1b[0m  {} {:02}:{:02} remaining                    \x1b[1;36m│\x1b[0m", bar, mins, secs);
        println!("\x1b[1;36m│\x1b[0m                                                  \x1b[1;36m│\x1b[0m");
        println!("\x1b[1;36m│\x1b[0m  [d] done  [s] skip  [p] pause  [q] quit        \x1b[1;36m│\x1b[0m");
        println!("\x1b[1;36m╰──────────────────────────────────────────────────╯\x1b[0m");

        stdout.flush()?;

        // Check for input (non-blocking)
        if crossterm::event::poll(Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Char('d') => {
                        // Mark done
                        let mut updated = todo.clone();
                        updated.mark_complete();
                        db.update_todo(&updated).await?;
                        println!("\n\x1b[32m✓ Marked as done!\x1b[0m");
                        break;
                    }
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Char('s') => {
                        println!("\n\x1b[33mFocus session ended.\x1b[0m");
                        break;
                    }
                    _ => {}
                }
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    let elapsed_mins = start.elapsed().as_secs() / 60;
    println!("\nFocused for {} minutes on: {}", elapsed_mins, todo.title);

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { format!("{:width$}", s, width = max) }
    else { format!("{}...", &s[..max-3]) }
}
```

---

## Task 4.2: Smart "Now" Command

**Files:**
- Create: `crates/todoee-cli/src/commands/now.rs`

```rust
use anyhow::Result;
use chrono::{Local, Timelike, Utc};
use todoee_core::{Config, LocalDb, Priority, Todo};

pub async fn now() -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;
    let todos = db.list_todos(false).await?;

    if todos.is_empty() {
        println!("\x1b[32mNothing to do! Enjoy your free time.\x1b[0m");
        return Ok(());
    }

    // Score each todo
    let mut scored: Vec<(Todo, f64, Vec<&str>)> = todos.into_iter()
        .map(|t| {
            let (score, reasons) = calculate_score(&t);
            (t, score, reasons)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("\x1b[1m┌─────────────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[1m│           RECOMMENDED RIGHT NOW                 │\x1b[0m");
    println!("\x1b[1m└─────────────────────────────────────────────────┘\x1b[0m\n");

    for (i, (todo, score, reasons)) in scored.iter().take(3).enumerate() {
        let marker = if i == 0 { "\x1b[1;32m→\x1b[0m" } else { " " };
        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };
        let id = &todo.id.to_string()[..8];

        println!("{} {} \x1b[90m{}\x1b[0m {}", marker, pri, id, todo.title);
        println!("    \x1b[90m{}\x1b[0m", reasons.join(" • "));
        println!();
    }

    if scored.len() > 3 {
        println!("\x1b[90m...and {} more todos\x1b[0m", scored.len() - 3);
    }

    Ok(())
}

fn calculate_score(todo: &Todo) -> (f64, Vec<&'static str>) {
    let mut score = 0.0;
    let mut reasons = Vec::new();

    // Priority weight
    match todo.priority {
        Priority::High => { score += 30.0; reasons.push("high priority"); }
        Priority::Medium => { score += 15.0; }
        Priority::Low => { score += 5.0; }
    }

    // Due date urgency
    if let Some(due) = todo.due_date {
        let hours_until = due.signed_duration_since(Utc::now()).num_hours();
        if hours_until < 0 {
            score += 50.0;
            reasons.push("overdue!");
        } else if hours_until < 4 {
            score += 40.0;
            reasons.push("due very soon");
        } else if hours_until < 24 {
            score += 25.0;
            reasons.push("due today");
        } else if hours_until < 72 {
            score += 10.0;
            reasons.push("due soon");
        }
    }

    // Time of day heuristics
    let hour = Local::now().hour();
    if hour >= 9 && hour < 12 {
        // Morning: favor high-priority
        if todo.priority == Priority::High {
            score += 10.0;
            reasons.push("morning = high focus time");
        }
    } else if hour >= 14 && hour < 17 {
        // Afternoon: favor medium tasks
        if todo.priority == Priority::Medium {
            score += 5.0;
        }
    } else if hour >= 20 {
        // Evening: favor low priority
        if todo.priority == Priority::Low {
            score += 5.0;
            reasons.push("evening = wind-down time");
        }
    }

    // Age penalty (older uncompleted = lower score, might be stuck)
    let age_days = Utc::now().signed_duration_since(todo.created_at).num_days();
    if age_days > 7 {
        score -= 5.0;
        reasons.push("consider breaking down");
    }

    (score, reasons)
}
```

---

## Task 4.3: Insights/Stats Command

**Files:**
- Create: `crates/todoee-cli/src/commands/insights.rs`

```rust
use anyhow::Result;
use chrono::{Datelike, Duration, Local, TimeZone, Utc, Weekday};
use todoee_core::{Config, LocalDb, OperationType};
use std::collections::HashMap;

pub async fn insights(days: Option<i64>) -> Result<()> {
    let config = Config::load()?;
    let db = LocalDb::new(&config.local_db_path()).await?;
    let days = days.unwrap_or(30);

    let since = Utc::now() - Duration::days(days);
    let operations = db.list_operations_since(since).await?;
    let todos = db.list_todos(true).await?;

    // Calculate metrics
    let total_completed = operations.iter()
        .filter(|op| op.operation_type == OperationType::Complete)
        .count();

    let total_created = operations.iter()
        .filter(|op| op.operation_type == OperationType::Create)
        .count();

    // Completion by day of week
    let mut by_weekday: HashMap<Weekday, usize> = HashMap::new();
    for op in operations.iter().filter(|op| op.operation_type == OperationType::Complete) {
        let local = Local.from_utc_datetime(&op.created_at.naive_utc());
        *by_weekday.entry(local.weekday()).or_insert(0) += 1;
    }

    // Find most productive day
    let best_day = by_weekday.iter()
        .max_by_key(|(_, count)| *count)
        .map(|(day, _)| day);

    // Completion heatmap (last 4 weeks)
    let mut heatmap: Vec<Vec<usize>> = vec![vec![0; 7]; 4]; // 4 weeks x 7 days
    for op in operations.iter().filter(|op| op.operation_type == OperationType::Complete) {
        let local = Local.from_utc_datetime(&op.created_at.naive_utc());
        let days_ago = (Utc::now() - op.created_at).num_days() as usize;
        if days_ago < 28 {
            let week = days_ago / 7;
            let day = local.weekday().num_days_from_monday() as usize;
            if week < 4 {
                heatmap[week][day] += 1;
            }
        }
    }

    // Print report
    println!("\x1b[1m┌─────────────────────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[1m│            PRODUCTIVITY INSIGHTS ({} days)              │\x1b[0m", days);
    println!("\x1b[1m└─────────────────────────────────────────────────────────┘\x1b[0m\n");

    println!("  Tasks Created:    {}", total_created);
    println!("  Tasks Completed:  {}", total_completed);
    let completion_rate = if total_created > 0 {
        (total_completed as f64 / total_created as f64 * 100.0) as u32
    } else { 0 };
    println!("  Completion Rate:  {}%", completion_rate);

    if let Some(day) = best_day {
        println!("  Most Productive:  {:?}", day);
    }

    println!("\n  \x1b[1mCompletion Heatmap (last 4 weeks):\x1b[0m");
    println!("         Mon Tue Wed Thu Fri Sat Sun");
    for (week_idx, week) in heatmap.iter().enumerate() {
        let week_label = match week_idx {
            0 => "This  ",
            1 => "Last  ",
            2 => "2 ago ",
            3 => "3 ago ",
            _ => "      ",
        };
        print!("  {}", week_label);
        for count in week {
            let block = match count {
                0 => "\x1b[90m░\x1b[0m",
                1..=2 => "\x1b[32m▒\x1b[0m",
                3..=5 => "\x1b[32m▓\x1b[0m",
                _ => "\x1b[32m█\x1b[0m",
            };
            print!("{}   ", block);
        }
        println!();
    }

    // Suggestions
    println!("\n  \x1b[1mSuggestions:\x1b[0m");

    let pending = todos.iter().filter(|t| !t.is_completed).count();
    let overdue = todos.iter()
        .filter(|t| !t.is_completed && t.due_date.map(|d| d < Utc::now()).unwrap_or(false))
        .count();

    if overdue > 0 {
        println!("  \x1b[33m•\x1b[0m You have {} overdue todos - consider rescheduling", overdue);
    }
    if pending > 20 {
        println!("  \x1b[33m•\x1b[0m {} pending todos - consider archiving or breaking down", pending);
    }
    if let Some(day) = best_day {
        println!("  \x1b[36m•\x1b[0m Schedule important tasks on {:?}s", day);
    }

    Ok(())
}
```

---

# PHASE 5-7: VIEWS, INTEGRATION, POLISH

*(Condensed for brevity - same TDD pattern)*

## Phase 5 Tasks:
- **5.1**: Kanban board view (`todoee board`)
- **5.2**: Timeline view (`todoee timeline`)
- **5.3**: Zen mode (`todoee zen`)
- **5.4**: Weekly review wizard (`todoee review`)

## Phase 6 Tasks:
- **6.1**: REST API server (`todoee serve`)
- **6.2**: MCP server (`todoee mcp`)
- **6.3**: Hook system (`todoee hook`)
- **6.4**: Context detection (`todoee ctx`)

## Phase 7 Tasks:
- **7.1**: Performance benchmarks
- **7.2**: TUI keybindings for new features
- **7.3**: Help documentation
- **7.4**: Release packaging

---

# COMMAND QUICK REFERENCE

After all phases, the complete command set:

```
CORE (Phase 1)
  todoee add         Add a todo (natural language)
  todoee list        List todos with filters
  todoee done        Mark todo complete
  todoee delete      Delete a todo
  todoee edit        Edit a todo
  todoee undo        Undo last operation
  todoee redo        Redo last undone operation
  todoee log         View operation history
  todoee diff        Show recent changes

VIEWS (Phase 2)
  todoee head N      Last N todos (most recent)
  todoee tail N      Oldest N todos
  todoee upcoming N  Next N by due date
  todoee overdue     All overdue todos
  todoee /           Fuzzy search
  todoee show ID     Full todo details

WORKFLOW (Phase 3)
  todoee stash       Stash commands (push/pop/list/clear)
  todoee batch       Batch operations (done/delete/priority)
  todoee gc          Garbage collect old data

INTELLIGENCE (Phase 4)
  todoee focus       Pomodoro focus mode
  todoee now         AI-recommended next task
  todoee insights    Productivity analytics
  todoee streak      Gamification stats

VIEWS (Phase 5)
  todoee board       Kanban view
  todoee timeline    Timeline view
  todoee zen         Single-task focus view
  todoee review      Weekly review wizard

INTEGRATION (Phase 6)
  todoee serve       Start REST API server
  todoee mcp         Start MCP server
  todoee hook        Manage automation hooks
  todoee ctx         Context detection
```

---

**Plan saved to:** `docs/plans/2026-01-30-unified-roadmap.md`

Ready to execute? **Which approach:**

1. **Subagent-Driven (this session)** - Fresh agent per task, code review between
2. **Parallel Session** - New session with executing-plans skill
