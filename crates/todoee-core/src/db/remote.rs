//! Remote PostgreSQL database for cloud sync with Neon.
//!
//! This module provides `RemoteDb`, a wrapper around a PostgreSQL connection pool
//! that handles CRUD operations for syncing todos with the cloud.

use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::models::{Category, Priority, SyncStatus, Todo};
use crate::{TodoeeError, Result as TodoeeResult};

/// Remote PostgreSQL database for cloud sync.
pub struct RemoteDb {
    pool: PgPool,
}

impl RemoteDb {
    /// Create a new RemoteDb connection to the specified database URL.
    pub async fn new(database_url: &str) -> TodoeeResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(TodoeeError::Database)?;

        let db = Self { pool };
        db.initialize().await?;
        Ok(db)
    }

    /// Initialize the database schema (create tables if they don't exist).
    async fn initialize(&self) -> TodoeeResult<()> {
        // Create categories table with soft delete
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS categories (
                id UUID PRIMARY KEY,
                user_id UUID,
                name TEXT NOT NULL,
                color TEXT,
                is_ai_generated BOOLEAN NOT NULL DEFAULT FALSE,
                sync_status TEXT NOT NULL DEFAULT 'synced',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                deleted_at TIMESTAMPTZ
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        // Create todos table with soft delete
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS todos (
                id UUID PRIMARY KEY,
                user_id UUID,
                category_id UUID REFERENCES categories(id),
                title TEXT NOT NULL,
                description TEXT,
                due_date TIMESTAMPTZ,
                reminder_at TIMESTAMPTZ,
                priority INTEGER NOT NULL DEFAULT 2,
                is_completed BOOLEAN NOT NULL DEFAULT FALSE,
                completed_at TIMESTAMPTZ,
                ai_metadata JSONB,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                deleted_at TIMESTAMPTZ
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        // Create indexes for efficient queries
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_todos_updated_at ON todos(updated_at)"
        )
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_todos_deleted_at ON todos(deleted_at)"
        )
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_todos_user_id ON todos(user_id)"
        )
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_categories_updated_at ON categories(updated_at)"
        )
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_categories_deleted_at ON categories(deleted_at)"
        )
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        Ok(())
    }

    /// Upsert a todo using last-write-wins conflict resolution.
    /// Only updates if the incoming `updated_at` is greater than the existing one.
    pub async fn upsert_todo(&self, todo: &Todo) -> TodoeeResult<()> {
        let priority_val = match todo.priority {
            Priority::Low => 1,
            Priority::Medium => 2,
            Priority::High => 3,
        };

        let ai_metadata = todo
            .ai_metadata
            .as_ref()
            .map(|v| v.to_string());

        sqlx::query(
            r#"
            INSERT INTO todos (
                id, user_id, category_id, title, description, due_date, reminder_at,
                priority, is_completed, completed_at, ai_metadata, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12, $13
            )
            ON CONFLICT (id) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                category_id = EXCLUDED.category_id,
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                due_date = EXCLUDED.due_date,
                reminder_at = EXCLUDED.reminder_at,
                priority = EXCLUDED.priority,
                is_completed = EXCLUDED.is_completed,
                completed_at = EXCLUDED.completed_at,
                ai_metadata = EXCLUDED.ai_metadata,
                updated_at = EXCLUDED.updated_at
            WHERE todos.updated_at < EXCLUDED.updated_at
            "#,
        )
        .bind(todo.id)
        .bind(todo.user_id)
        .bind(todo.category_id)
        .bind(&todo.title)
        .bind(&todo.description)
        .bind(todo.due_date)
        .bind(todo.reminder_at)
        .bind(priority_val)
        .bind(todo.is_completed)
        .bind(todo.completed_at)
        .bind(ai_metadata)
        .bind(todo.created_at)
        .bind(todo.updated_at)
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        Ok(())
    }

    /// Get all todos updated since the given timestamp (for incremental sync).
    /// Excludes soft-deleted todos.
    pub async fn get_todos_since(&self, since: DateTime<Utc>) -> TodoeeResult<Vec<Todo>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, user_id, category_id, title, description, due_date, reminder_at,
                priority, is_completed, completed_at, ai_metadata, created_at, updated_at
            FROM todos
            WHERE updated_at > $1 AND deleted_at IS NULL
            ORDER BY updated_at ASC
            "#,
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        let mut todos = Vec::with_capacity(rows.len());
        for row in rows {
            let ai_metadata: Option<serde_json::Value> = row.get("ai_metadata");

            let todo = Todo {
                id: row.get("id"),
                user_id: row.get("user_id"),
                category_id: row.get("category_id"),
                title: row.get("title"),
                description: row.get("description"),
                due_date: row.get("due_date"),
                reminder_at: row.get("reminder_at"),
                priority: Priority::from_i32(row.get("priority")),
                is_completed: row.get("is_completed"),
                completed_at: row.get("completed_at"),
                ai_metadata,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                sync_status: SyncStatus::Synced,
            };
            todos.push(todo);
        }

        Ok(todos)
    }

    /// Soft delete a todo by setting its deleted_at timestamp.
    pub async fn soft_delete_todo(&self, id: Uuid) -> TodoeeResult<()> {
        sqlx::query(
            "UPDATE todos SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        Ok(())
    }

    /// Upsert a category using last-write-wins conflict resolution.
    /// Only updates if the incoming `updated_at` is greater than the existing one.
    pub async fn upsert_category(&self, category: &Category, updated_at: DateTime<Utc>) -> TodoeeResult<()> {
        sqlx::query(
            r#"
            INSERT INTO categories (
                id, user_id, name, color, is_ai_generated, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6
            )
            ON CONFLICT (id) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                name = EXCLUDED.name,
                color = EXCLUDED.color,
                is_ai_generated = EXCLUDED.is_ai_generated,
                updated_at = EXCLUDED.updated_at
            WHERE categories.updated_at < EXCLUDED.updated_at
            "#,
        )
        .bind(category.id)
        .bind(category.user_id)
        .bind(&category.name)
        .bind(&category.color)
        .bind(category.is_ai_generated)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        Ok(())
    }

    /// Get all categories updated since the given timestamp.
    pub async fn get_categories_since(&self, since: DateTime<Utc>) -> TodoeeResult<Vec<Category>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, name, color, is_ai_generated
            FROM categories
            WHERE updated_at > $1 AND deleted_at IS NULL
            ORDER BY updated_at ASC
            "#,
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        let mut categories = Vec::with_capacity(rows.len());
        for row in rows {
            let category = Category {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                color: row.get("color"),
                is_ai_generated: row.get("is_ai_generated"),
                sync_status: SyncStatus::Synced,
            };
            categories.push(category);
        }

        Ok(categories)
    }

    /// Soft delete a category by setting its deleted_at timestamp.
    pub async fn soft_delete_category(&self, id: Uuid) -> TodoeeResult<()> {
        sqlx::query(
            "UPDATE categories SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(TodoeeError::Database)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires NEON_DATABASE_URL environment variable
    async fn test_remote_db_connection() {
        let url = std::env::var("NEON_DATABASE_URL")
            .expect("NEON_DATABASE_URL must be set");
        let db = RemoteDb::new(&url).await;
        assert!(db.is_ok(), "Failed to connect to remote database: {:?}", db.err());
    }

    #[tokio::test]
    #[ignore] // Requires NEON_DATABASE_URL environment variable
    async fn test_remote_db_upsert_and_get() {
        let url = std::env::var("NEON_DATABASE_URL")
            .expect("NEON_DATABASE_URL must be set");
        let db = RemoteDb::new(&url).await.expect("Failed to connect");

        // Create a test todo
        let todo = Todo::new("Test remote sync".to_string(), None);

        // Upsert it
        db.upsert_todo(&todo).await.expect("Failed to upsert todo");

        // Get todos since before it was created
        let since = todo.created_at - chrono::Duration::seconds(1);
        let todos = db.get_todos_since(since).await.expect("Failed to get todos");

        assert!(todos.iter().any(|t| t.id == todo.id), "Todo not found after upsert");

        // Clean up - soft delete
        db.soft_delete_todo(todo.id).await.expect("Failed to soft delete todo");
    }
}
