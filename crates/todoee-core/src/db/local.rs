//! Local SQLite database for offline-first storage.
//!
//! This module provides `LocalDb`, a wrapper around a SQLite connection pool
//! that handles CRUD operations for todos and categories.

use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::{Category, EntityType, Operation, OperationType, Priority, SyncStatus, Todo};

/// Helper struct for mapping todo rows from SQLite.
#[derive(Debug, FromRow)]
struct TodoRow {
    id: String,
    user_id: Option<String>,
    category_id: Option<String>,
    title: String,
    description: Option<String>,
    due_date: Option<String>,
    reminder_at: Option<String>,
    priority: i32,
    is_completed: i32,
    completed_at: Option<String>,
    ai_metadata: Option<String>,
    created_at: String,
    updated_at: String,
    sync_status: String,
}

impl TryFrom<TodoRow> for Todo {
    type Error = anyhow::Error;

    fn try_from(row: TodoRow) -> Result<Self> {
        Ok(Todo {
            id: Uuid::parse_str(&row.id).context("Invalid todo id")?,
            user_id: row
                .user_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .context("Invalid user_id")?,
            category_id: row
                .category_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .context("Invalid category_id")?,
            title: row.title,
            description: row.description,
            due_date: row
                .due_date
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()
                .context("Invalid due_date")?,
            reminder_at: row
                .reminder_at
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()
                .context("Invalid reminder_at")?,
            priority: Priority::from_i32(row.priority),
            is_completed: row.is_completed != 0,
            completed_at: row
                .completed_at
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()
                .context("Invalid completed_at")?,
            ai_metadata: row
                .ai_metadata
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .context("Invalid ai_metadata")?,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .context("Invalid created_at")?,
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .context("Invalid updated_at")?,
            sync_status: match row.sync_status.as_str() {
                "synced" => SyncStatus::Synced,
                "conflict" => SyncStatus::Conflict,
                _ => SyncStatus::Pending,
            },
        })
    }
}

/// Helper struct for mapping category rows from SQLite.
#[derive(Debug, FromRow)]
struct CategoryRow {
    id: String,
    user_id: Option<String>,
    name: String,
    color: Option<String>,
    is_ai_generated: i32,
    sync_status: String,
}

impl TryFrom<CategoryRow> for Category {
    type Error = anyhow::Error;

    fn try_from(row: CategoryRow) -> Result<Self> {
        Ok(Category {
            id: Uuid::parse_str(&row.id).context("Invalid category id")?,
            user_id: Uuid::parse_str(&row.user_id.unwrap_or_else(|| Uuid::nil().to_string()))
                .context("Invalid user_id")?,
            name: row.name,
            color: row.color,
            is_ai_generated: row.is_ai_generated != 0,
            sync_status: match row.sync_status.as_str() {
                "synced" => SyncStatus::Synced,
                "conflict" => SyncStatus::Conflict,
                _ => SyncStatus::Pending,
            },
        })
    }
}

/// Helper struct for mapping operation rows from SQLite.
#[derive(Debug, FromRow)]
struct OperationRow {
    id: String,
    operation_type: String,
    entity_type: String,
    entity_id: String,
    previous_state: Option<String>,
    new_state: Option<String>,
    created_at: String,
    undone: i32,
}

impl TryFrom<OperationRow> for Operation {
    type Error = anyhow::Error;

    fn try_from(row: OperationRow) -> Result<Self> {
        let operation_type = match row.operation_type.as_str() {
            "create" => OperationType::Create,
            "update" => OperationType::Update,
            "delete" => OperationType::Delete,
            "complete" => OperationType::Complete,
            "uncomplete" => OperationType::Uncomplete,
            "stash" => OperationType::Stash,
            "unstash" => OperationType::Unstash,
            _ => anyhow::bail!("Invalid operation type: {}", row.operation_type),
        };

        let entity_type = match row.entity_type.as_str() {
            "todo" => EntityType::Todo,
            "category" => EntityType::Category,
            _ => anyhow::bail!("Invalid entity type: {}", row.entity_type),
        };

        Ok(Operation {
            id: Uuid::parse_str(&row.id).context("Invalid operation id")?,
            operation_type,
            entity_type,
            entity_id: Uuid::parse_str(&row.entity_id).context("Invalid entity_id")?,
            previous_state: row
                .previous_state
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .context("Invalid previous_state JSON")?,
            new_state: row
                .new_state
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .context("Invalid new_state JSON")?,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .context("Invalid created_at")?,
            undone: row.undone != 0,
        })
    }
}

/// Local SQLite database for offline-first storage.
pub struct LocalDb {
    pool: SqlitePool,
}

impl LocalDb {
    /// Get a reference to the connection pool (for testing).
    #[cfg(test)]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    /// Create an in-memory database for testing.
    pub async fn new_in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::from_str(":memory:")?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .context("Failed to create in-memory database")?;

        Ok(Self { pool })
    }

    /// Create a file-based database at the specified path.
    pub async fn new(path: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .context("Failed to open database")?;

        Ok(Self { pool })
    }

    /// Run database migrations to create tables and indexes.
    pub async fn run_migrations(&self) -> Result<()> {
        // Create categories table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS categories (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                name TEXT NOT NULL,
                color TEXT,
                is_ai_generated INTEGER NOT NULL DEFAULT 0,
                sync_status TEXT NOT NULL DEFAULT 'pending'
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create categories table")?;

        // Create todos table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS todos (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                category_id TEXT REFERENCES categories(id),
                title TEXT NOT NULL,
                description TEXT,
                due_date TEXT,
                reminder_at TEXT,
                priority INTEGER NOT NULL DEFAULT 2,
                is_completed INTEGER NOT NULL DEFAULT 0,
                completed_at TEXT,
                ai_metadata TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                sync_status TEXT NOT NULL DEFAULT 'pending'
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create todos table")?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_todos_due_date ON todos(due_date)")
            .execute(&self.pool)
            .await
            .context("Failed to create due_date index")?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_todos_sync_status ON todos(sync_status)")
            .execute(&self.pool)
            .await
            .context("Failed to create sync_status index")?;

        // Create operations table for undo/redo and analytics
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
        .await
        .context("Failed to create operations table")?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_operations_created_at ON operations(created_at DESC)")
            .execute(&self.pool)
            .await
            .context("Failed to create operations created_at index")?;

        // Create stash table for temporarily storing todos
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
        .await
        .context("Failed to create stash table")?;

        // Create deleted_todos tracking table for sync
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS deleted_todos (
                id TEXT PRIMARY KEY,
                deleted_at TEXT NOT NULL,
                synced INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create deleted_todos table")?;

        Ok(())
    }

    // ==================== Todo CRUD Operations ====================

    /// Create a new todo in the database.
    pub async fn create_todo(&self, todo: &Todo) -> Result<()> {
        let priority_val = match todo.priority {
            Priority::Low => 1,
            Priority::Medium => 2,
            Priority::High => 3,
        };

        let sync_status = match todo.sync_status {
            SyncStatus::Pending => "pending",
            SyncStatus::Synced => "synced",
            SyncStatus::Conflict => "conflict",
        };

        sqlx::query(
            r#"
            INSERT INTO todos (
                id, user_id, category_id, title, description, due_date, reminder_at,
                priority, is_completed, completed_at, ai_metadata, created_at, updated_at, sync_status
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
            )
            "#,
        )
        .bind(todo.id.to_string())
        .bind(todo.user_id.map(|u| u.to_string()))
        .bind(todo.category_id.map(|c| c.to_string()))
        .bind(&todo.title)
        .bind(&todo.description)
        .bind(todo.due_date.map(|d| d.to_rfc3339()))
        .bind(todo.reminder_at.map(|r| r.to_rfc3339()))
        .bind(priority_val)
        .bind(if todo.is_completed { 1 } else { 0 })
        .bind(todo.completed_at.map(|c| c.to_rfc3339()))
        .bind(todo.ai_metadata.as_ref().map(|m| m.to_string()))
        .bind(todo.created_at.to_rfc3339())
        .bind(todo.updated_at.to_rfc3339())
        .bind(sync_status)
        .execute(&self.pool)
        .await
        .context("Failed to create todo")?;

        Ok(())
    }

    /// Get a todo by its ID.
    pub async fn get_todo(&self, id: Uuid) -> Result<Option<Todo>> {
        let row: Option<TodoRow> = sqlx::query_as(
            "SELECT * FROM todos WHERE id = ?1",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch todo")?;

        row.map(|r| r.try_into()).transpose()
    }

    /// List todos, optionally excluding completed ones.
    /// If `exclude_completed` is true, only non-completed todos are returned.
    pub async fn list_todos(&self, exclude_completed: bool) -> Result<Vec<Todo>> {
        let query = if exclude_completed {
            "SELECT * FROM todos WHERE is_completed = 0 ORDER BY created_at DESC"
        } else {
            "SELECT * FROM todos ORDER BY created_at DESC"
        };

        let rows: Vec<TodoRow> = sqlx::query_as(query)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list todos")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List all todos due today.
    pub async fn list_todos_due_today(&self) -> Result<Vec<Todo>> {
        let today = Utc::now().date_naive();
        let start = today.and_hms_opt(0, 0, 0).unwrap();
        let end = today.and_hms_opt(23, 59, 59).unwrap();

        let start_str = DateTime::<Utc>::from_naive_utc_and_offset(start, Utc).to_rfc3339();
        let end_str = DateTime::<Utc>::from_naive_utc_and_offset(end, Utc).to_rfc3339();

        let rows: Vec<TodoRow> = sqlx::query_as(
            "SELECT * FROM todos WHERE due_date >= ?1 AND due_date <= ?2 ORDER BY due_date ASC",
        )
        .bind(start_str)
        .bind(end_str)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list todos due today")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List all todos in a specific category.
    pub async fn list_todos_by_category(&self, category_id: Uuid) -> Result<Vec<Todo>> {
        let rows: Vec<TodoRow> = sqlx::query_as(
            "SELECT * FROM todos WHERE category_id = ?1 ORDER BY created_at DESC",
        )
        .bind(category_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to list todos by category")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List all todos pending sync.
    pub async fn list_pending_sync(&self) -> Result<Vec<Todo>> {
        let rows: Vec<TodoRow> = sqlx::query_as(
            "SELECT * FROM todos WHERE sync_status = 'pending' ORDER BY updated_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list pending sync todos")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Update an existing todo.
    pub async fn update_todo(&self, todo: &Todo) -> Result<()> {
        let priority_val = match todo.priority {
            Priority::Low => 1,
            Priority::Medium => 2,
            Priority::High => 3,
        };

        let sync_status = match todo.sync_status {
            SyncStatus::Pending => "pending",
            SyncStatus::Synced => "synced",
            SyncStatus::Conflict => "conflict",
        };

        sqlx::query(
            r#"
            UPDATE todos SET
                user_id = ?1,
                category_id = ?2,
                title = ?3,
                description = ?4,
                due_date = ?5,
                reminder_at = ?6,
                priority = ?7,
                is_completed = ?8,
                completed_at = ?9,
                ai_metadata = ?10,
                updated_at = ?11,
                sync_status = ?12
            WHERE id = ?13
            "#,
        )
        .bind(todo.user_id.map(|u| u.to_string()))
        .bind(todo.category_id.map(|c| c.to_string()))
        .bind(&todo.title)
        .bind(&todo.description)
        .bind(todo.due_date.map(|d| d.to_rfc3339()))
        .bind(todo.reminder_at.map(|r| r.to_rfc3339()))
        .bind(priority_val)
        .bind(if todo.is_completed { 1 } else { 0 })
        .bind(todo.completed_at.map(|c| c.to_rfc3339()))
        .bind(todo.ai_metadata.as_ref().map(|m| m.to_string()))
        .bind(todo.updated_at.to_rfc3339())
        .bind(sync_status)
        .bind(todo.id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to update todo")?;

        Ok(())
    }

    /// Mark a todo as synced.
    pub async fn mark_synced(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE todos SET sync_status = 'synced' WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to mark todo as synced")?;

        Ok(())
    }

    /// Delete a todo by its ID.
    pub async fn delete_todo(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM todos WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to delete todo")?;

        Ok(())
    }

    // ==================== Category CRUD Operations ====================

    /// Create a new category in the database.
    pub async fn create_category(&self, category: &Category) -> Result<()> {
        let sync_status = match category.sync_status {
            SyncStatus::Pending => "pending",
            SyncStatus::Synced => "synced",
            SyncStatus::Conflict => "conflict",
        };

        sqlx::query(
            r#"
            INSERT INTO categories (id, user_id, name, color, is_ai_generated, sync_status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(category.id.to_string())
        .bind(Some(category.user_id.to_string()))
        .bind(&category.name)
        .bind(&category.color)
        .bind(if category.is_ai_generated { 1 } else { 0 })
        .bind(sync_status)
        .execute(&self.pool)
        .await
        .context("Failed to create category")?;

        Ok(())
    }

    /// Get a category by its name.
    pub async fn get_category_by_name(&self, name: &str) -> Result<Option<Category>> {
        let row: Option<CategoryRow> = sqlx::query_as(
            "SELECT * FROM categories WHERE name = ?1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch category by name")?;

        row.map(|r| r.try_into()).transpose()
    }

    /// List all categories.
    pub async fn list_categories(&self) -> Result<Vec<Category>> {
        let rows: Vec<CategoryRow> = sqlx::query_as(
            "SELECT * FROM categories ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list categories")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Delete a category by ID.
    pub async fn delete_category(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM categories WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to delete category")?;

        Ok(())
    }

    /// List all categories pending sync.
    pub async fn list_pending_categories(&self) -> Result<Vec<Category>> {
        let rows: Vec<CategoryRow> = sqlx::query_as(
            r#"SELECT id, user_id, name, color, is_ai_generated, sync_status FROM categories WHERE sync_status = 'pending'"#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list pending sync categories")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Mark a category as synced.
    pub async fn mark_category_synced(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE categories SET sync_status = 'synced' WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to mark category as synced")?;

        Ok(())
    }

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

    // ==================== Operation CRUD Operations ====================

    /// Record an operation in the history.
    pub async fn record_operation(&self, op: &Operation) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO operations (
                id, operation_type, entity_type, entity_id,
                previous_state, new_state, created_at, undone
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(op.id.to_string())
        .bind(op.operation_type.to_string())
        .bind(op.entity_type.to_string())
        .bind(op.entity_id.to_string())
        .bind(op.previous_state.as_ref().map(|v| v.to_string()))
        .bind(op.new_state.as_ref().map(|v| v.to_string()))
        .bind(op.created_at.to_rfc3339())
        .bind(if op.undone { 1 } else { 0 })
        .execute(&self.pool)
        .await
        .context("Failed to record operation")?;

        Ok(())
    }

    /// Get the last operation that can be undone (not yet undone).
    pub async fn get_last_undoable_operation(&self) -> Result<Option<Operation>> {
        let row: Option<OperationRow> = sqlx::query_as(
            "SELECT * FROM operations WHERE undone = 0 ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch last undoable operation")?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get the last operation that can be redone (already undone).
    pub async fn get_last_redoable_operation(&self) -> Result<Option<Operation>> {
        let row: Option<OperationRow> = sqlx::query_as(
            "SELECT * FROM operations WHERE undone = 1 ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch last redoable operation")?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Mark an operation as undone.
    pub async fn mark_operation_undone(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE operations SET undone = 1 WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to mark operation as undone")?;

        Ok(())
    }

    /// Mark an operation as redone (not undone).
    pub async fn mark_operation_redone(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE operations SET undone = 0 WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to mark operation as redone")?;

        Ok(())
    }

    /// List recent operations, limited by count.
    pub async fn list_operations(&self, limit: usize) -> Result<Vec<Operation>> {
        let rows: Vec<OperationRow> = sqlx::query_as(
            "SELECT * FROM operations ORDER BY created_at DESC LIMIT ?1",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list operations")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List operations since a given timestamp.
    pub async fn list_operations_since(&self, since: DateTime<Utc>) -> Result<Vec<Operation>> {
        let rows: Vec<OperationRow> = sqlx::query_as(
            "SELECT * FROM operations WHERE created_at >= ?1 ORDER BY created_at DESC",
        )
        .bind(since.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to list operations since timestamp")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Clear operations older than the specified number of days.
    /// Returns the number of deleted operations.
    pub async fn clear_old_operations(&self, days: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let result = sqlx::query("DELETE FROM operations WHERE created_at < ?1")
            .bind(cutoff.to_rfc3339())
            .execute(&self.pool)
            .await
            .context("Failed to clear old operations")?;

        Ok(result.rows_affected())
    }

    // ==================== Head/Tail/Upcoming/Overdue Queries ====================

    /// List N most recently created todos.
    pub async fn list_todos_head(&self, limit: usize, include_completed: bool) -> Result<Vec<Todo>> {
        let query = if include_completed {
            "SELECT * FROM todos ORDER BY created_at DESC LIMIT ?1"
        } else {
            "SELECT * FROM todos WHERE is_completed = 0 ORDER BY created_at DESC LIMIT ?1"
        };

        let rows: Vec<TodoRow> = sqlx::query_as(query)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list head todos")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List N oldest todos.
    pub async fn list_todos_tail(&self, limit: usize, include_completed: bool) -> Result<Vec<Todo>> {
        let query = if include_completed {
            "SELECT * FROM todos ORDER BY created_at ASC LIMIT ?1"
        } else {
            "SELECT * FROM todos WHERE is_completed = 0 ORDER BY created_at ASC LIMIT ?1"
        };

        let rows: Vec<TodoRow> = sqlx::query_as(query)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list tail todos")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List N upcoming todos by due date (future only).
    pub async fn list_todos_upcoming(&self, limit: usize) -> Result<Vec<Todo>> {
        let now = Utc::now().to_rfc3339();

        let rows: Vec<TodoRow> = sqlx::query_as(
            "SELECT * FROM todos WHERE is_completed = 0 AND due_date IS NOT NULL AND due_date >= ?1 ORDER BY due_date ASC LIMIT ?2",
        )
        .bind(&now)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list upcoming todos")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List all overdue todos.
    pub async fn list_todos_overdue(&self) -> Result<Vec<Todo>> {
        let now = Utc::now().to_rfc3339();

        let rows: Vec<TodoRow> = sqlx::query_as(
            "SELECT * FROM todos WHERE is_completed = 0 AND due_date IS NOT NULL AND due_date < ?1 ORDER BY due_date ASC",
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list overdue todos")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List todos with reminders due within the specified time window.
    /// Only returns incomplete todos with reminder_at between now and now + window.
    /// Also includes reminders up to 5 minutes in the past to handle slight delays.
    pub async fn list_todos_with_reminders_due(
        &self,
        window: chrono::Duration,
    ) -> Result<Vec<Todo>> {
        let now = Utc::now();
        let until = now + window;

        let rows: Vec<TodoRow> = sqlx::query_as(
            r#"
            SELECT id, user_id, category_id, title, description, due_date,
                   reminder_at, priority, is_completed, completed_at,
                   ai_metadata, created_at, updated_at, sync_status
            FROM todos
            WHERE reminder_at IS NOT NULL
              AND reminder_at <= ?1
              AND reminder_at > ?2
              AND is_completed = 0
            ORDER BY reminder_at ASC
            "#,
        )
        .bind(until.to_rfc3339())
        .bind((now - chrono::Duration::minutes(5)).to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to list todos with reminders due")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Stash Operations ====================

    /// Stash a todo (hide it temporarily).
    ///
    /// # Errors
    /// Returns error if todo doesn't exist or is already stashed.
    pub async fn stash_todo(&self, todo_id: Uuid, message: Option<&str>) -> Result<Todo> {
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

    /// Pop the most recently stashed todo.
    pub async fn stash_pop(&self) -> Result<Option<Todo>> {
        let row = sqlx::query("SELECT * FROM stash ORDER BY stashed_at DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .context("Failed to fetch stashed todo")?;

        if let Some(row) = row {
            use sqlx::Row;
            let id: String = row.get("id");
            let json: String = row.get("todo_json");
            let todo: Todo = serde_json::from_str(&json).context("Failed to parse stashed todo")?;

            sqlx::query("DELETE FROM stash WHERE id = ?")
                .bind(&id)
                .execute(&self.pool)
                .await
                .context("Failed to delete from stash")?;

            self.create_todo(&todo).await?;
            Ok(Some(todo))
        } else {
            Ok(None)
        }
    }

    /// List all stashed todos.
    /// Returns a vector of tuples: (todo, stashed_at timestamp, optional message).
    pub async fn stash_list(&self) -> Result<Vec<(Todo, String, Option<String>)>> {
        let rows = sqlx::query("SELECT * FROM stash ORDER BY stashed_at DESC")
            .fetch_all(&self.pool)
            .await
            .context("Failed to list stashed todos")?;

        let mut result = Vec::new();
        for row in rows {
            use sqlx::Row;
            let json: String = row.get("todo_json");
            let at: String = row.get("stashed_at");
            let msg: Option<String> = row.get("message");
            let todo: Todo = serde_json::from_str(&json).context("Failed to parse stashed todo")?;
            result.push((todo, at, msg));
        }
        Ok(result)
    }

    /// Clear all stashed todos.
    /// Returns the number of cleared items.
    pub async fn stash_clear(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM stash")
            .execute(&self.pool)
            .await
            .context("Failed to clear stash")?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> LocalDb {
        let db = LocalDb::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_create_and_get_todo() {
        let db = setup_db().await;

        let todo = Todo::new("Buy groceries".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        let retrieved = db.get_todo(todo.id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, todo.id);
        assert_eq!(retrieved.title, "Buy groceries");
        assert!(!retrieved.is_completed);
        assert_eq!(retrieved.priority, Priority::Medium);
    }

    #[tokio::test]
    async fn test_list_todos_not_completed() {
        let db = setup_db().await;

        // Create some todos
        let todo1 = Todo::new("Task 1".to_string(), None);
        let mut todo2 = Todo::new("Task 2".to_string(), None);
        todo2.mark_complete();
        let todo3 = Todo::new("Task 3".to_string(), None);

        db.create_todo(&todo1).await.unwrap();
        db.create_todo(&todo2).await.unwrap();
        db.create_todo(&todo3).await.unwrap();

        // list_todos(true) returns only non-completed
        let todos = db.list_todos(true).await.unwrap();
        assert_eq!(todos.len(), 2);

        // list_todos(false) returns all todos
        let all_todos = db.list_todos(false).await.unwrap();
        assert_eq!(all_todos.len(), 3);

        // Verify none of them are completed
        for todo in &todos {
            assert!(!todo.is_completed);
        }
    }

    #[tokio::test]
    async fn test_update_todo() {
        let db = setup_db().await;

        let mut todo = Todo::new("Original title".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        // Update the todo
        todo.title = "Updated title".to_string();
        todo.description = Some("A description".to_string());
        todo.priority = Priority::High;
        db.update_todo(&todo).await.unwrap();

        // Verify the update
        let retrieved = db.get_todo(todo.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated title");
        assert_eq!(retrieved.description, Some("A description".to_string()));
        assert_eq!(retrieved.priority, Priority::High);
    }

    #[tokio::test]
    async fn test_delete_todo() {
        let db = setup_db().await;

        let todo = Todo::new("To be deleted".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        // Verify it exists
        assert!(db.get_todo(todo.id).await.unwrap().is_some());

        // Delete it
        db.delete_todo(todo.id).await.unwrap();

        // Verify it's gone
        assert!(db.get_todo(todo.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_pending_sync() {
        let db = setup_db().await;

        // Create some todos with different sync statuses
        let todo1 = Todo::new("Pending 1".to_string(), None);
        let todo2 = Todo::new("Pending 2".to_string(), None);

        db.create_todo(&todo1).await.unwrap();
        db.create_todo(&todo2).await.unwrap();

        // Mark one as synced
        db.mark_synced(todo1.id).await.unwrap();

        // List pending sync
        let pending = db.list_pending_sync().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, todo2.id);
    }

    #[tokio::test]
    async fn test_categories() {
        let db = setup_db().await;

        let user_id = Uuid::new_v4();
        let mut category = Category::new(user_id, "Work".to_string());
        category.color = Some("#ff0000".to_string());

        db.create_category(&category).await.unwrap();

        // Get by name
        let retrieved = db.get_category_by_name("Work").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "Work");
        assert_eq!(retrieved.color, Some("#ff0000".to_string()));

        // List categories
        let categories = db.list_categories().await.unwrap();
        assert_eq!(categories.len(), 1);
    }

    #[tokio::test]
    async fn test_list_todos_with_reminders_due() {
        let db = setup_db().await;

        // Create todo with reminder in 10 minutes
        let mut todo = Todo::new("Reminder task".to_string(), None);
        todo.reminder_at = Some(Utc::now() + chrono::Duration::minutes(10));
        db.create_todo(&todo).await.unwrap();

        // Create todo without reminder
        let todo2 = Todo::new("No reminder".to_string(), None);
        db.create_todo(&todo2).await.unwrap();

        // Get todos with reminders due in next 15 minutes
        let window = chrono::Duration::minutes(15);
        let results = db.list_todos_with_reminders_due(window).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Reminder task");
    }

    #[tokio::test]
    async fn test_list_pending_categories() {
        let db = setup_db().await;

        let user_id = Uuid::new_v4();
        let category = Category::new(user_id, "Work".to_string());
        db.create_category(&category).await.unwrap();

        let pending = db.list_pending_categories().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].name, "Work");
    }

    #[tokio::test]
    async fn test_mark_category_synced() {
        let db = setup_db().await;

        let user_id = Uuid::new_v4();
        let category = Category::new(user_id, "Work".to_string());
        db.create_category(&category).await.unwrap();

        let pending = db.list_pending_categories().await.unwrap();
        assert_eq!(pending.len(), 1);

        db.mark_category_synced(category.id).await.unwrap();

        let pending = db.list_pending_categories().await.unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_deleted_todos_table_exists() {
        let db = setup_db().await;

        // Table should exist after migrations
        let result = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='deleted_todos'")
            .fetch_optional(db.pool())
            .await
            .unwrap();

        assert!(result.is_some(), "deleted_todos table should exist");
    }
}
