# Todoee MVP Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build an offline-first Rust CLI todo app with AI-powered natural language parsing, cloud sync via Neon Postgres, and cross-platform notifications.

**Architecture:** Three-crate Rust workspace (CLI, core library, daemon). Offline-first with SQLite local cache syncing to Neon Postgres. OpenRouter API for AI parsing with user-configurable model string. JWT auth with email/password.

**Tech Stack:** Rust, clap, ratatui, reqwest, sqlx (Postgres + SQLite), notify-rust, tokio, serde, toml

---

## Phase 1: Foundation

### Task 1: Initialize Rust Workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/todoee-cli/Cargo.toml`
- Create: `crates/todoee-cli/src/main.rs`
- Create: `crates/todoee-core/Cargo.toml`
- Create: `crates/todoee-core/src/lib.rs`
- Create: `crates/todoee-daemon/Cargo.toml`
- Create: `crates/todoee-daemon/src/main.rs`
- Create: `.gitignore`

**Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/todoee-cli",
    "crates/todoee-core",
    "crates/todoee-daemon",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["Your Name <you@example.com>"]
license = "MIT"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

**Step 2: Create todoee-core Cargo.toml**

```toml
[package]
name = "todoee-core"
version.workspace = true
edition.workspace = true

[dependencies]
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
anyhow.workspace = true
tracing.workspace = true
uuid.workspace = true
chrono.workspace = true
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "sqlite", "uuid", "chrono", "json"] }
reqwest = { version = "0.12", features = ["json"] }
argon2 = "0.5"
jsonwebtoken = "9"
toml = "0.8"
dirs = "6"

[dev-dependencies]
tokio-test = "0.4"
```

**Step 3: Create todoee-cli Cargo.toml**

```toml
[package]
name = "todoee-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "todoee"
path = "src/main.rs"

[dependencies]
todoee-core = { path = "../todoee-core" }
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
clap = { version = "4", features = ["derive"] }
ratatui = "0.29"
crossterm = "0.28"
```

**Step 4: Create todoee-daemon Cargo.toml**

```toml
[package]
name = "todoee-daemon"
version.workspace = true
edition.workspace = true

[[bin]]
name = "todoee-daemon"
path = "src/main.rs"

[dependencies]
todoee-core = { path = "../todoee-core" }
tokio.workspace = true
serde.workspace = true
anyhow.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
notify-rust = "4"
```

**Step 5: Create minimal source files**

`crates/todoee-core/src/lib.rs`:
```rust
pub mod models;
pub mod config;
pub mod db;
pub mod ai;
pub mod auth;
pub mod sync;

pub use models::*;
pub use config::Config;
```

`crates/todoee-cli/src/main.rs`:
```rust
use anyhow::Result;

fn main() -> Result<()> {
    println!("todoee - AI-powered todo manager");
    Ok(())
}
```

`crates/todoee-daemon/src/main.rs`:
```rust
use anyhow::Result;

fn main() -> Result<()> {
    println!("todoee-daemon starting...");
    Ok(())
}
```

**Step 6: Create .gitignore**

```gitignore
/target
.env
*.db
*.db-journal
Cargo.lock
```

**Step 7: Verify workspace compiles**

Run: `cargo check`
Expected: Successful compilation with no errors

**Step 8: Commit**

```bash
git init
git add .
git commit -m "feat: initialize Rust workspace with three crates"
```

---

### Task 2: Define Core Data Models

**Files:**
- Create: `crates/todoee-core/src/models.rs`
- Test: `crates/todoee-core/src/models.rs` (inline tests)

**Step 1: Write failing test for Todo model**

Add to `crates/todoee-core/src/models.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_new_creates_valid_todo() {
        let todo = Todo::new("Buy milk".to_string(), None);

        assert_eq!(todo.title, "Buy milk");
        assert!(!todo.is_completed);
        assert!(todo.category_id.is_none());
        assert_eq!(todo.priority, Priority::Medium);
    }

    #[test]
    fn test_todo_mark_complete_sets_completed_at() {
        let mut todo = Todo::new("Test task".to_string(), None);
        assert!(!todo.is_completed);
        assert!(todo.completed_at.is_none());

        todo.mark_complete();

        assert!(todo.is_completed);
        assert!(todo.completed_at.is_some());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::High > Priority::Medium);
        assert!(Priority::Medium > Priority::Low);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p todoee-core`
Expected: FAIL with compilation errors (structs not defined)

**Step 3: Implement models**

Complete `crates/todoee-core/src/models.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low = 1,
    Medium = 2,
    High = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Pending,
    Synced,
    Conflict,
}

impl Default for SyncStatus {
    fn default() -> Self {
        SyncStatus::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub is_ai_generated: bool,
    pub sync_status: SyncStatus,
}

impl Category {
    pub fn new(user_id: Uuid, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            name,
            color: None,
            is_ai_generated: false,
            sync_status: SyncStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub reminder_at: Option<DateTime<Utc>>,
    pub priority: Priority,
    pub is_completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub ai_metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sync_status: SyncStatus,
}

impl Todo {
    pub fn new(title: String, user_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            category_id: None,
            title,
            description: None,
            due_date: None,
            reminder_at: None,
            priority: Priority::default(),
            is_completed: false,
            completed_at: None,
            ai_metadata: None,
            created_at: now,
            updated_at: now,
            sync_status: SyncStatus::Pending,
        }
    }

    pub fn mark_complete(&mut self) {
        self.is_completed = true;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.sync_status = SyncStatus::Pending;
    }

    pub fn mark_incomplete(&mut self) {
        self.is_completed = false;
        self.completed_at = None;
        self.updated_at = Utc::now();
        self.sync_status = SyncStatus::Pending;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub reminder_at: Option<DateTime<Utc>>,
    pub recurrence_rule: Option<String>,
    pub created_at: DateTime<Utc>,
    pub sync_status: SyncStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_new_creates_valid_todo() {
        let todo = Todo::new("Buy milk".to_string(), None);

        assert_eq!(todo.title, "Buy milk");
        assert!(!todo.is_completed);
        assert!(todo.category_id.is_none());
        assert_eq!(todo.priority, Priority::Medium);
    }

    #[test]
    fn test_todo_mark_complete_sets_completed_at() {
        let mut todo = Todo::new("Test task".to_string(), None);
        assert!(!todo.is_completed);
        assert!(todo.completed_at.is_none());

        todo.mark_complete();

        assert!(todo.is_completed);
        assert!(todo.completed_at.is_some());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::High > Priority::Medium);
        assert!(Priority::Medium > Priority::Low);
    }

    #[test]
    fn test_category_new() {
        let user_id = Uuid::new_v4();
        let cat = Category::new(user_id, "Work".to_string());

        assert_eq!(cat.name, "Work");
        assert_eq!(cat.user_id, user_id);
        assert!(!cat.is_ai_generated);
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p todoee-core`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add crates/todoee-core/src/models.rs
git commit -m "feat: add core data models (Todo, Category, User, Event)"
```

---

### Task 3: Implement Configuration System

**Files:**
- Create: `crates/todoee-core/src/config.rs`
- Test: `crates/todoee-core/src/config.rs` (inline tests)

**Step 1: Write failing test for config loading**

Add to `crates/todoee-core/src/config.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_config_default_values() {
        let config = Config::default();

        assert_eq!(config.ai.provider, "openrouter");
        assert!(config.ai.model.is_none());
        assert!(config.notifications.enabled);
        assert_eq!(config.notifications.advance_minutes, 15);
    }

    #[test]
    fn test_config_load_from_toml() {
        let toml_content = r#"
[ai]
provider = "openrouter"
model = "anthropic/claude-3-haiku"
api_key_env = "OPENROUTER_API_KEY"

[database]
url_env = "NEON_DATABASE_URL"

[notifications]
enabled = true
sound = false
advance_minutes = 30
"#;
        let config: Config = toml::from_str(toml_content).unwrap();

        assert_eq!(config.ai.model, Some("anthropic/claude-3-haiku".to_string()));
        assert_eq!(config.notifications.advance_minutes, 30);
        assert!(!config.notifications.sound);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p todoee-core config`
Expected: FAIL (Config struct not defined)

**Step 3: Implement configuration**

Complete `crates/todoee-core/src/config.rs`:
```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub display: DisplayConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ai: AiConfig::default(),
            database: DatabaseConfig::default(),
            notifications: NotificationConfig::default(),
            display: DisplayConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    /// User-specified model string (e.g., "anthropic/claude-3-haiku")
    pub model: Option<String>,
    #[serde(default = "default_ai_key_env")]
    pub api_key_env: String,
}

fn default_ai_provider() -> String {
    "openrouter".to_string()
}

fn default_ai_key_env() -> String {
    "OPENROUTER_API_KEY".to_string()
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: default_ai_provider(),
            model: None,
            api_key_env: default_ai_key_env(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_url_env")]
    pub url_env: String,
    #[serde(default = "default_local_db_name")]
    pub local_db_name: String,
}

fn default_db_url_env() -> String {
    "NEON_DATABASE_URL".to_string()
}

fn default_local_db_name() -> String {
    "cache.db".to_string()
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url_env: default_db_url_env(),
            local_db_name: default_local_db_name(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub sound: bool,
    #[serde(default = "default_advance_minutes")]
    pub advance_minutes: u32,
}

fn default_true() -> bool {
    true
}

fn default_advance_minutes() -> u32 {
    15
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sound: true,
            advance_minutes: 15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_date_format")]
    pub date_format: String,
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_date_format() -> String {
    "%Y-%m-%d".to_string()
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            date_format: default_date_format(),
        }
    }
}

impl Config {
    /// Get the config directory path (~/.config/todoee/)
    pub fn config_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("todoee");
        Ok(config_dir)
    }

    /// Get the config file path (~/.config/todoee/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Get the local database path (~/.config/todoee/cache.db)
    pub fn local_db_path(&self) -> Result<PathBuf> {
        Ok(Self::config_dir()?.join(&self.database.local_db_name))
    }

    /// Get the auth token path (~/.config/todoee/auth.json)
    pub fn auth_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("auth.json"))
    }

    /// Load config from file, or return default if file doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Get the AI API key from environment
    pub fn get_ai_api_key(&self) -> Result<String> {
        std::env::var(&self.ai.api_key_env).with_context(|| {
            format!(
                "AI API key not found. Set the {} environment variable or configure ai.api_key_env in config.toml",
                self.ai.api_key_env
            )
        })
    }

    /// Get the database URL from environment
    pub fn get_database_url(&self) -> Result<String> {
        std::env::var(&self.database.url_env).with_context(|| {
            format!(
                "Database URL not found. Set the {} environment variable or configure database.url_env in config.toml",
                self.database.url_env
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_values() {
        let config = Config::default();

        assert_eq!(config.ai.provider, "openrouter");
        assert!(config.ai.model.is_none());
        assert!(config.notifications.enabled);
        assert_eq!(config.notifications.advance_minutes, 15);
    }

    #[test]
    fn test_config_load_from_toml() {
        let toml_content = r#"
[ai]
provider = "openrouter"
model = "anthropic/claude-3-haiku"
api_key_env = "OPENROUTER_API_KEY"

[database]
url_env = "NEON_DATABASE_URL"

[notifications]
enabled = true
sound = false
advance_minutes = 30
"#;
        let config: Config = toml::from_str(toml_content).unwrap();

        assert_eq!(config.ai.model, Some("anthropic/claude-3-haiku".to_string()));
        assert_eq!(config.notifications.advance_minutes, 30);
        assert!(!config.notifications.sound);
    }

    #[test]
    fn test_config_partial_toml() {
        let toml_content = r#"
[ai]
model = "custom/model-name"
"#;
        let config: Config = toml::from_str(toml_content).unwrap();

        // Custom value
        assert_eq!(config.ai.model, Some("custom/model-name".to_string()));
        // Defaults still applied
        assert_eq!(config.ai.provider, "openrouter");
        assert!(config.notifications.enabled);
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p todoee-core config`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add crates/todoee-core/src/config.rs
git commit -m "feat: add configuration system with TOML support"
```

---

### Task 4: Implement Local SQLite Database

**Files:**
- Create: `crates/todoee-core/src/db/mod.rs`
- Create: `crates/todoee-core/src/db/local.rs`
- Modify: `crates/todoee-core/src/lib.rs`
- Test: `crates/todoee-core/src/db/local.rs` (inline tests)

**Step 1: Create db module structure**

Create `crates/todoee-core/src/db/mod.rs`:
```rust
pub mod local;

pub use local::LocalDb;
```

**Step 2: Write failing test for local database**

Create `crates/todoee-core/src/db/local.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Priority, Todo};

    #[tokio::test]
    async fn test_create_and_get_todo() {
        let db = LocalDb::new_in_memory().await.unwrap();

        let mut todo = Todo::new("Test task".to_string(), None);
        todo.priority = Priority::High;

        db.create_todo(&todo).await.unwrap();

        let retrieved = db.get_todo(todo.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Test task");
        assert_eq!(retrieved.priority, Priority::High);
    }

    #[tokio::test]
    async fn test_list_todos_not_completed() {
        let db = LocalDb::new_in_memory().await.unwrap();

        let todo1 = Todo::new("Task 1".to_string(), None);
        let mut todo2 = Todo::new("Task 2".to_string(), None);
        todo2.mark_complete();

        db.create_todo(&todo1).await.unwrap();
        db.create_todo(&todo2).await.unwrap();

        let todos = db.list_todos(false).await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].title, "Task 1");
    }

    #[tokio::test]
    async fn test_update_todo() {
        let db = LocalDb::new_in_memory().await.unwrap();

        let mut todo = Todo::new("Original".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        todo.title = "Updated".to_string();
        db.update_todo(&todo).await.unwrap();

        let retrieved = db.get_todo(todo.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated");
    }

    #[tokio::test]
    async fn test_delete_todo() {
        let db = LocalDb::new_in_memory().await.unwrap();

        let todo = Todo::new("To delete".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        db.delete_todo(todo.id).await.unwrap();

        let retrieved = db.get_todo(todo.id).await.unwrap();
        assert!(retrieved.is_none());
    }
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test -p todoee-core db::local`
Expected: FAIL (LocalDb not defined)

**Step 4: Implement LocalDb**

Complete `crates/todoee-core/src/db/local.rs`:
```rust
use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

use crate::models::{Category, Priority, SyncStatus, Todo};

pub struct LocalDb {
    pool: SqlitePool,
}

impl LocalDb {
    /// Create a new in-memory database (for testing)
    pub async fn new_in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::from_str(":memory:")?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .context("Failed to create in-memory SQLite database")?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    /// Create a new file-based database
    pub async fn new(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .with_context(|| format!("Failed to open SQLite database: {}", path.display()))?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    async fn run_migrations(&self) -> Result<()> {
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

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_todos_due_date ON todos(due_date)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_todos_sync_status ON todos(sync_status)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create sync status index")?;

        Ok(())
    }

    // --- Todo CRUD ---

    pub async fn create_todo(&self, todo: &Todo) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO todos (
                id, user_id, category_id, title, description, due_date,
                reminder_at, priority, is_completed, completed_at,
                ai_metadata, created_at, updated_at, sync_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(todo.id.to_string())
        .bind(todo.user_id.map(|u| u.to_string()))
        .bind(todo.category_id.map(|c| c.to_string()))
        .bind(&todo.title)
        .bind(&todo.description)
        .bind(todo.due_date.map(|d| d.to_rfc3339()))
        .bind(todo.reminder_at.map(|d| d.to_rfc3339()))
        .bind(todo.priority as i32)
        .bind(todo.is_completed)
        .bind(todo.completed_at.map(|d| d.to_rfc3339()))
        .bind(todo.ai_metadata.as_ref().map(|m| m.to_string()))
        .bind(todo.created_at.to_rfc3339())
        .bind(todo.updated_at.to_rfc3339())
        .bind(format!("{:?}", todo.sync_status).to_lowercase())
        .execute(&self.pool)
        .await
        .context("Failed to create todo")?;

        Ok(())
    }

    pub async fn get_todo(&self, id: Uuid) -> Result<Option<Todo>> {
        let row = sqlx::query_as::<_, TodoRow>(
            r#"
            SELECT id, user_id, category_id, title, description, due_date,
                   reminder_at, priority, is_completed, completed_at,
                   ai_metadata, created_at, updated_at, sync_status
            FROM todos WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get todo")?;

        row.map(|r| r.into_todo()).transpose()
    }

    pub async fn list_todos(&self, include_completed: bool) -> Result<Vec<Todo>> {
        let query = if include_completed {
            r#"
            SELECT id, user_id, category_id, title, description, due_date,
                   reminder_at, priority, is_completed, completed_at,
                   ai_metadata, created_at, updated_at, sync_status
            FROM todos ORDER BY due_date ASC NULLS LAST, priority DESC
            "#
        } else {
            r#"
            SELECT id, user_id, category_id, title, description, due_date,
                   reminder_at, priority, is_completed, completed_at,
                   ai_metadata, created_at, updated_at, sync_status
            FROM todos WHERE is_completed = 0
            ORDER BY due_date ASC NULLS LAST, priority DESC
            "#
        };

        let rows = sqlx::query_as::<_, TodoRow>(query)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list todos")?;

        rows.into_iter().map(|r| r.into_todo()).collect()
    }

    pub async fn list_todos_due_today(&self) -> Result<Vec<Todo>> {
        let today = chrono::Utc::now().date_naive();
        let tomorrow = today.succ_opt().unwrap();

        let rows = sqlx::query_as::<_, TodoRow>(
            r#"
            SELECT id, user_id, category_id, title, description, due_date,
                   reminder_at, priority, is_completed, completed_at,
                   ai_metadata, created_at, updated_at, sync_status
            FROM todos
            WHERE is_completed = 0
              AND due_date >= ?
              AND due_date < ?
            ORDER BY due_date ASC, priority DESC
            "#,
        )
        .bind(today.to_string())
        .bind(tomorrow.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to list today's todos")?;

        rows.into_iter().map(|r| r.into_todo()).collect()
    }

    pub async fn list_todos_by_category(&self, category_id: Uuid) -> Result<Vec<Todo>> {
        let rows = sqlx::query_as::<_, TodoRow>(
            r#"
            SELECT id, user_id, category_id, title, description, due_date,
                   reminder_at, priority, is_completed, completed_at,
                   ai_metadata, created_at, updated_at, sync_status
            FROM todos WHERE category_id = ? AND is_completed = 0
            ORDER BY due_date ASC NULLS LAST, priority DESC
            "#,
        )
        .bind(category_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to list todos by category")?;

        rows.into_iter().map(|r| r.into_todo()).collect()
    }

    pub async fn list_pending_sync(&self) -> Result<Vec<Todo>> {
        let rows = sqlx::query_as::<_, TodoRow>(
            r#"
            SELECT id, user_id, category_id, title, description, due_date,
                   reminder_at, priority, is_completed, completed_at,
                   ai_metadata, created_at, updated_at, sync_status
            FROM todos WHERE sync_status = 'pending'
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list pending sync todos")?;

        rows.into_iter().map(|r| r.into_todo()).collect()
    }

    pub async fn update_todo(&self, todo: &Todo) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE todos SET
                category_id = ?, title = ?, description = ?, due_date = ?,
                reminder_at = ?, priority = ?, is_completed = ?, completed_at = ?,
                ai_metadata = ?, updated_at = ?, sync_status = ?
            WHERE id = ?
            "#,
        )
        .bind(todo.category_id.map(|c| c.to_string()))
        .bind(&todo.title)
        .bind(&todo.description)
        .bind(todo.due_date.map(|d| d.to_rfc3339()))
        .bind(todo.reminder_at.map(|d| d.to_rfc3339()))
        .bind(todo.priority as i32)
        .bind(todo.is_completed)
        .bind(todo.completed_at.map(|d| d.to_rfc3339()))
        .bind(todo.ai_metadata.as_ref().map(|m| m.to_string()))
        .bind(todo.updated_at.to_rfc3339())
        .bind(format!("{:?}", todo.sync_status).to_lowercase())
        .bind(todo.id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to update todo")?;

        Ok(())
    }

    pub async fn mark_synced(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE todos SET sync_status = 'synced' WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to mark todo as synced")?;

        Ok(())
    }

    pub async fn delete_todo(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM todos WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to delete todo")?;

        Ok(())
    }

    // --- Category CRUD ---

    pub async fn create_category(&self, category: &Category) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO categories (id, user_id, name, color, is_ai_generated, sync_status)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(category.id.to_string())
        .bind(category.user_id.to_string())
        .bind(&category.name)
        .bind(&category.color)
        .bind(category.is_ai_generated)
        .bind(format!("{:?}", category.sync_status).to_lowercase())
        .execute(&self.pool)
        .await
        .context("Failed to create category")?;

        Ok(())
    }

    pub async fn get_category_by_name(&self, name: &str) -> Result<Option<Category>> {
        let row = sqlx::query_as::<_, CategoryRow>(
            "SELECT id, user_id, name, color, is_ai_generated, sync_status FROM categories WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get category")?;

        row.map(|r| r.into_category()).transpose()
    }

    pub async fn list_categories(&self) -> Result<Vec<Category>> {
        let rows = sqlx::query_as::<_, CategoryRow>(
            "SELECT id, user_id, name, color, is_ai_generated, sync_status FROM categories ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list categories")?;

        rows.into_iter().map(|r| r.into_category()).collect()
    }
}

// Helper structs for SQLite row mapping
#[derive(sqlx::FromRow)]
struct TodoRow {
    id: String,
    user_id: Option<String>,
    category_id: Option<String>,
    title: String,
    description: Option<String>,
    due_date: Option<String>,
    reminder_at: Option<String>,
    priority: i32,
    is_completed: bool,
    completed_at: Option<String>,
    ai_metadata: Option<String>,
    created_at: String,
    updated_at: String,
    sync_status: String,
}

impl TodoRow {
    fn into_todo(self) -> Result<Todo> {
        use chrono::DateTime;

        Ok(Todo {
            id: Uuid::parse_str(&self.id).context("Invalid todo id")?,
            user_id: self
                .user_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .context("Invalid user_id")?,
            category_id: self
                .category_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .context("Invalid category_id")?,
            title: self.title,
            description: self.description,
            due_date: self
                .due_date
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&chrono::Utc)))
                .transpose()
                .context("Invalid due_date")?,
            reminder_at: self
                .reminder_at
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&chrono::Utc)))
                .transpose()
                .context("Invalid reminder_at")?,
            priority: match self.priority {
                1 => Priority::Low,
                3 => Priority::High,
                _ => Priority::Medium,
            },
            is_completed: self.is_completed,
            completed_at: self
                .completed_at
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&chrono::Utc)))
                .transpose()
                .context("Invalid completed_at")?,
            ai_metadata: self
                .ai_metadata
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .context("Invalid ai_metadata")?,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)
                .context("Invalid created_at")?
                .with_timezone(&chrono::Utc),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at)
                .context("Invalid updated_at")?
                .with_timezone(&chrono::Utc),
            sync_status: match self.sync_status.as_str() {
                "synced" => SyncStatus::Synced,
                "conflict" => SyncStatus::Conflict,
                _ => SyncStatus::Pending,
            },
        })
    }
}

#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: String,
    user_id: String,
    name: String,
    color: Option<String>,
    is_ai_generated: bool,
    sync_status: String,
}

impl CategoryRow {
    fn into_category(self) -> Result<Category> {
        Ok(Category {
            id: Uuid::parse_str(&self.id).context("Invalid category id")?,
            user_id: Uuid::parse_str(&self.user_id).context("Invalid user_id")?,
            name: self.name,
            color: self.color,
            is_ai_generated: self.is_ai_generated,
            sync_status: match self.sync_status.as_str() {
                "synced" => SyncStatus::Synced,
                "conflict" => SyncStatus::Conflict,
                _ => SyncStatus::Pending,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Priority, Todo};

    #[tokio::test]
    async fn test_create_and_get_todo() {
        let db = LocalDb::new_in_memory().await.unwrap();

        let mut todo = Todo::new("Test task".to_string(), None);
        todo.priority = Priority::High;

        db.create_todo(&todo).await.unwrap();

        let retrieved = db.get_todo(todo.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Test task");
        assert_eq!(retrieved.priority, Priority::High);
    }

    #[tokio::test]
    async fn test_list_todos_not_completed() {
        let db = LocalDb::new_in_memory().await.unwrap();

        let todo1 = Todo::new("Task 1".to_string(), None);
        let mut todo2 = Todo::new("Task 2".to_string(), None);
        todo2.mark_complete();

        db.create_todo(&todo1).await.unwrap();
        db.create_todo(&todo2).await.unwrap();

        let todos = db.list_todos(false).await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].title, "Task 1");
    }

    #[tokio::test]
    async fn test_update_todo() {
        let db = LocalDb::new_in_memory().await.unwrap();

        let mut todo = Todo::new("Original".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        todo.title = "Updated".to_string();
        db.update_todo(&todo).await.unwrap();

        let retrieved = db.get_todo(todo.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated");
    }

    #[tokio::test]
    async fn test_delete_todo() {
        let db = LocalDb::new_in_memory().await.unwrap();

        let todo = Todo::new("To delete".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        db.delete_todo(todo.id).await.unwrap();

        let retrieved = db.get_todo(todo.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_pending_sync() {
        let db = LocalDb::new_in_memory().await.unwrap();

        let todo = Todo::new("Pending sync".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        let pending = db.list_pending_sync().await.unwrap();
        assert_eq!(pending.len(), 1);

        db.mark_synced(todo.id).await.unwrap();

        let pending = db.list_pending_sync().await.unwrap();
        assert_eq!(pending.len(), 0);
    }
}
```

**Step 5: Update lib.rs exports**

Modify `crates/todoee-core/src/lib.rs`:
```rust
pub mod models;
pub mod config;
pub mod db;

pub use models::*;
pub use config::Config;
pub use db::LocalDb;
```

**Step 6: Run tests to verify they pass**

Run: `cargo test -p todoee-core`
Expected: All tests PASS

**Step 7: Commit**

```bash
git add crates/todoee-core/src/db/ crates/todoee-core/src/lib.rs
git commit -m "feat: add local SQLite database with CRUD operations"
```

---

### Task 5: Implement Error Types

**Files:**
- Create: `crates/todoee-core/src/error.rs`
- Modify: `crates/todoee-core/src/lib.rs`

**Step 1: Create error types**

Create `crates/todoee-core/src/error.rs`:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TodoeeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("AI service error: {message}\n\nTo fix this:\n1. Check that your AI model string is correct in ~/.config/todoee/config.toml\n2. Verify your OPENROUTER_API_KEY environment variable is set\n3. Ensure you have API credits available\n\nExample config:\n[ai]\nmodel = \"anthropic/claude-3-haiku\"")]
    AiService { message: String },

    #[error("AI parsing failed: {message}\n\nThe AI could not parse your input. Please try:\n1. Being more specific (e.g., \"buy milk tomorrow at 9am\" instead of \"milk\")\n2. Adding the task manually with: todoee add --title \"your task\" --category \"Work\"")]
    AiParsing { message: String },

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Network error: {0}\n\nYou appear to be offline. Your changes are saved locally and will sync when you reconnect.")]
    Network(String),

    #[error("Sync conflict: {0}")]
    SyncConflict(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, TodoeeError>;
```

**Step 2: Update lib.rs**

Add to `crates/todoee-core/src/lib.rs`:
```rust
pub mod models;
pub mod config;
pub mod db;
pub mod error;

pub use models::*;
pub use config::Config;
pub use db::LocalDb;
pub use error::{TodoeeError, Result};
```

**Step 3: Verify compilation**

Run: `cargo check -p todoee-core`
Expected: Successful compilation

**Step 4: Commit**

```bash
git add crates/todoee-core/src/error.rs crates/todoee-core/src/lib.rs
git commit -m "feat: add custom error types with helpful messages"
```

---

## Phase 2: AI Integration

### Task 6: Implement OpenRouter AI Client

**Files:**
- Create: `crates/todoee-core/src/ai.rs`
- Modify: `crates/todoee-core/src/lib.rs`
- Test: `crates/todoee-core/src/ai.rs` (inline tests)

**Step 1: Write failing test for AI parsing**

Create `crates/todoee-core/src/ai.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ai_response_valid() {
        let response = r#"{
            "title": "Buy groceries",
            "due_date": "2026-01-31T09:00:00Z",
            "category": "Shopping",
            "priority": 3
        }"#;

        let parsed = ParsedTask::from_json(response).unwrap();

        assert_eq!(parsed.title, "Buy groceries");
        assert_eq!(parsed.category, Some("Shopping".to_string()));
        assert_eq!(parsed.priority, Some(3));
    }

    #[test]
    fn test_parse_ai_response_minimal() {
        let response = r#"{"title": "Simple task"}"#;

        let parsed = ParsedTask::from_json(response).unwrap();

        assert_eq!(parsed.title, "Simple task");
        assert!(parsed.category.is_none());
    }

    #[test]
    fn test_parse_ai_response_invalid() {
        let response = "not json at all";

        let result = ParsedTask::from_json(response);
        assert!(result.is_err());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p todoee-core ai`
Expected: FAIL (ParsedTask not defined)

**Step 3: Implement AI client**

Complete `crates/todoee-core/src/ai.rs`:
```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::TodoeeError;

/// Structured output from AI parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTask {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub reminder_at: Option<DateTime<Utc>>,
}

impl ParsedTask {
    pub fn from_json(json: &str) -> Result<Self> {
        // Try to extract JSON from the response (AI might include extra text)
        let json_str = extract_json(json).unwrap_or(json);

        serde_json::from_str(json_str).with_context(|| {
            format!("Failed to parse AI response as JSON: {}", json)
        })
    }
}

/// Extract JSON object from a string that might contain extra text
fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0;
    let mut end = start;

    for (i, c) in text[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth == 0 && end > start {
        Some(&text[start..end])
    } else {
        None
    }
}

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

pub struct AiClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AiClient {
    pub fn new(config: &Config) -> Result<Self, TodoeeError> {
        let api_key = config.get_ai_api_key().map_err(|e| {
            TodoeeError::AiService {
                message: e.to_string(),
            }
        })?;

        let model = config.ai.model.clone().ok_or_else(|| {
            TodoeeError::AiService {
                message: "No AI model configured. Add 'model = \"your-model-string\"' to [ai] section in ~/.config/todoee/config.toml".to_string(),
            }
        })?;

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        })
    }

    /// Parse natural language into a structured task
    pub async fn parse_task(&self, input: &str) -> Result<ParsedTask, TodoeeError> {
        let current_date = Utc::now().format("%Y-%m-%d").to_string();

        let system_prompt = format!(
            r#"You are a task parser. Extract structured data from natural language task descriptions.
Today's date is {}.

Respond ONLY with a JSON object (no other text) with these fields:
- title: string (required) - the task title, cleaned up and properly capitalized
- description: string or null - additional details if provided
- due_date: ISO 8601 datetime or null - when the task is due
- category: string or null - category like "Work", "Shopping", "Personal", "Health", "Finance", etc.
- priority: 1, 2, or 3 (1=low, 2=medium, 3=high) or null
- reminder_at: ISO 8601 datetime or null - when to remind

Examples:
Input: "buy groceries tomorrow morning"
Output: {{"title": "Buy groceries", "due_date": "2026-01-31T09:00:00Z", "category": "Shopping", "priority": 2}}

Input: "urgent: call dentist"
Output: {{"title": "Call dentist", "category": "Health", "priority": 3}}

Input: "review PR #123 for work by end of day"
Output: {{"title": "Review PR #123", "due_date": "2026-01-30T17:00:00Z", "category": "Work", "priority": 2}}"#,
            current_date
        );

        let request = OpenRouterRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                Message {
                    role: "user".to_string(),
                    content: input.to_string(),
                },
            ],
            temperature: 0.1,
            max_tokens: 500,
        };

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| TodoeeError::AiService {
                message: format!("Failed to connect to OpenRouter: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TodoeeError::AiService {
                message: format!("OpenRouter returned error {}: {}", status, body),
            });
        }

        let response: OpenRouterResponse = response.json().await.map_err(|e| {
            TodoeeError::AiService {
                message: format!("Failed to parse OpenRouter response: {}", e),
            }
        })?;

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .ok_or_else(|| TodoeeError::AiService {
                message: "No response from AI model".to_string(),
            })?;

        ParsedTask::from_json(content).map_err(|e| TodoeeError::AiParsing {
            message: format!("AI returned invalid format: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ai_response_valid() {
        let response = r#"{
            "title": "Buy groceries",
            "due_date": "2026-01-31T09:00:00Z",
            "category": "Shopping",
            "priority": 3
        }"#;

        let parsed = ParsedTask::from_json(response).unwrap();

        assert_eq!(parsed.title, "Buy groceries");
        assert_eq!(parsed.category, Some("Shopping".to_string()));
        assert_eq!(parsed.priority, Some(3));
    }

    #[test]
    fn test_parse_ai_response_minimal() {
        let response = r#"{"title": "Simple task"}"#;

        let parsed = ParsedTask::from_json(response).unwrap();

        assert_eq!(parsed.title, "Simple task");
        assert!(parsed.category.is_none());
    }

    #[test]
    fn test_parse_ai_response_invalid() {
        let response = "not json at all";

        let result = ParsedTask::from_json(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_json_with_extra_text() {
        let text = r#"Here's the parsed task:
{"title": "Test", "priority": 2}
Hope that helps!"#;

        let extracted = extract_json(text).unwrap();
        assert_eq!(extracted, r#"{"title": "Test", "priority": 2}"#);
    }

    #[test]
    fn test_extract_json_nested() {
        let text = r#"{"outer": {"inner": "value"}}"#;
        let extracted = extract_json(text).unwrap();
        assert_eq!(extracted, text);
    }
}
```

**Step 4: Update lib.rs**

Add to exports in `crates/todoee-core/src/lib.rs`:
```rust
pub mod models;
pub mod config;
pub mod db;
pub mod error;
pub mod ai;

pub use models::*;
pub use config::Config;
pub use db::LocalDb;
pub use error::{TodoeeError, Result};
pub use ai::{AiClient, ParsedTask};
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p todoee-core ai`
Expected: All tests PASS

**Step 6: Commit**

```bash
git add crates/todoee-core/src/ai.rs crates/todoee-core/src/lib.rs
git commit -m "feat: add OpenRouter AI client for task parsing"
```

---

## Phase 3: CLI Commands

### Task 7: Set Up CLI Structure with Clap

**Files:**
- Modify: `crates/todoee-cli/src/main.rs`
- Create: `crates/todoee-cli/src/commands/mod.rs`
- Create: `crates/todoee-cli/src/commands/add.rs`
- Create: `crates/todoee-cli/src/commands/list.rs`
- Create: `crates/todoee-cli/src/commands/done.rs`
- Create: `crates/todoee-cli/src/commands/delete.rs`

**Step 1: Create CLI argument structure**

Modify `crates/todoee-cli/src/main.rs`:
```rust
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;

#[derive(Parser)]
#[command(name = "todoee")]
#[command(author, version, about = "AI-powered todo manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new task (AI-powered natural language parsing)
    Add {
        /// Task description in natural language
        description: Vec<String>,

        /// Skip AI parsing and add as plain text
        #[arg(long)]
        no_ai: bool,

        /// Manually specify category
        #[arg(short, long)]
        category: Option<String>,

        /// Manually specify priority (1=low, 2=medium, 3=high)
        #[arg(short, long)]
        priority: Option<i32>,
    },

    /// List tasks
    List {
        /// Show only tasks due today
        #[arg(long)]
        today: bool,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Include completed tasks
        #[arg(short, long)]
        all: bool,
    },

    /// Mark a task as done
    Done {
        /// Task ID (partial match supported)
        id: String,
    },

    /// Delete a task
    Delete {
        /// Task ID (partial match supported)
        id: String,
    },

    /// Edit a task
    Edit {
        /// Task ID (partial match supported)
        id: String,

        /// New title
        #[arg(short, long)]
        title: Option<String>,

        /// New category
        #[arg(short, long)]
        category: Option<String>,

        /// New priority (1=low, 2=medium, 3=high)
        #[arg(short, long)]
        priority: Option<i32>,
    },

    /// Force sync with cloud
    Sync,

    /// Show configuration
    Config {
        /// Initialize default config file
        #[arg(long)]
        init: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "todoee=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Add { description, no_ai, category, priority }) => {
            commands::add::run(description.join(" "), no_ai, category, priority).await
        }
        Some(Commands::List { today, category, all }) => {
            commands::list::run(today, category, all).await
        }
        Some(Commands::Done { id }) => {
            commands::done::run(&id).await
        }
        Some(Commands::Delete { id }) => {
            commands::delete::run(&id).await
        }
        Some(Commands::Edit { id, title, category, priority }) => {
            commands::edit::run(&id, title, category, priority).await
        }
        Some(Commands::Sync) => {
            commands::sync::run().await
        }
        Some(Commands::Config { init }) => {
            commands::config::run(init).await
        }
        None => {
            // No command = show today's tasks as default
            commands::list::run(true, None, false).await
        }
    }
}
```

**Step 2: Create commands module**

Create `crates/todoee-cli/src/commands/mod.rs`:
```rust
pub mod add;
pub mod list;
pub mod done;
pub mod delete;
pub mod edit;
pub mod sync;
pub mod config;
```

**Step 3: Verify compilation**

Run: `cargo check -p todoee-cli`
Expected: Compilation errors (modules not yet created)

**Step 4: Create stub command files**

Create `crates/todoee-cli/src/commands/add.rs`:
```rust
use anyhow::Result;

pub async fn run(
    description: String,
    no_ai: bool,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
    println!("Add command: {} (no_ai: {}, category: {:?}, priority: {:?})",
             description, no_ai, category, priority);
    Ok(())
}
```

Create `crates/todoee-cli/src/commands/list.rs`:
```rust
use anyhow::Result;

pub async fn run(today: bool, category: Option<String>, all: bool) -> Result<()> {
    println!("List command: today={}, category={:?}, all={}", today, category, all);
    Ok(())
}
```

Create `crates/todoee-cli/src/commands/done.rs`:
```rust
use anyhow::Result;

pub async fn run(id: &str) -> Result<()> {
    println!("Done command: {}", id);
    Ok(())
}
```

Create `crates/todoee-cli/src/commands/delete.rs`:
```rust
use anyhow::Result;

pub async fn run(id: &str) -> Result<()> {
    println!("Delete command: {}", id);
    Ok(())
}
```

Create `crates/todoee-cli/src/commands/edit.rs`:
```rust
use anyhow::Result;

pub async fn run(
    id: &str,
    title: Option<String>,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
    println!("Edit command: {} (title: {:?}, category: {:?}, priority: {:?})",
             id, title, category, priority);
    Ok(())
}
```

Create `crates/todoee-cli/src/commands/sync.rs`:
```rust
use anyhow::Result;

pub async fn run() -> Result<()> {
    println!("Sync command");
    Ok(())
}
```

Create `crates/todoee-cli/src/commands/config.rs`:
```rust
use anyhow::Result;

pub async fn run(init: bool) -> Result<()> {
    println!("Config command: init={}", init);
    Ok(())
}
```

**Step 5: Verify compilation**

Run: `cargo build -p todoee-cli`
Expected: Successful compilation

**Step 6: Test CLI help**

Run: `cargo run -p todoee-cli -- --help`
Expected: Shows help text with all commands

**Step 7: Commit**

```bash
git add crates/todoee-cli/
git commit -m "feat: set up CLI structure with clap"
```

---

### Task 8: Implement Add Command

**Files:**
- Modify: `crates/todoee-cli/src/commands/add.rs`

**Step 1: Implement the add command**

Replace `crates/todoee-cli/src/commands/add.rs`:
```rust
use anyhow::{Context, Result};
use todoee_core::{AiClient, Config, LocalDb, Priority, Todo, Category};
use uuid::Uuid;

pub async fn run(
    description: String,
    no_ai: bool,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
    if description.trim().is_empty() {
        anyhow::bail!("Task description cannot be empty");
    }

    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;
    let db = LocalDb::new(&db_path).await?;

    let mut todo = if no_ai || config.ai.model.is_none() {
        // Manual mode: create task directly from input
        let mut todo = Todo::new(description.clone(), None);

        if let Some(p) = priority {
            todo.priority = match p {
                1 => Priority::Low,
                3 => Priority::High,
                _ => Priority::Medium,
            };
        }

        todo
    } else {
        // AI mode: parse natural language
        match parse_with_ai(&config, &description).await {
            Ok(todo) => todo,
            Err(e) => {
                eprintln!("AI parsing failed: {}\n", e);
                eprintln!("Creating task with original text instead.");
                Todo::new(description.clone(), None)
            }
        }
    };

    // Override category if manually specified
    if let Some(cat_name) = category {
        let cat_id = get_or_create_category(&db, &cat_name, None).await?;
        todo.category_id = Some(cat_id);
    }

    // Override priority if manually specified
    if let Some(p) = priority {
        todo.priority = match p {
            1 => Priority::Low,
            3 => Priority::High,
            _ => Priority::Medium,
        };
    }

    db.create_todo(&todo).await?;

    // Display result
    println!("✓ Created: {}", todo.title);

    if let Some(cat_id) = todo.category_id {
        if let Some(cat) = find_category_name(&db, cat_id).await? {
            println!("  Category: {}", cat);
        }
    }

    if let Some(due) = todo.due_date {
        println!("  Due: {}", due.format("%Y-%m-%d %H:%M"));
    }

    let priority_str = match todo.priority {
        Priority::Low => "Low",
        Priority::Medium => "Medium",
        Priority::High => "High",
    };
    println!("  Priority: {}", priority_str);
    println!("  ID: {}", &todo.id.to_string()[..8]);

    Ok(())
}

async fn parse_with_ai(config: &Config, description: &str) -> Result<Todo> {
    let client = AiClient::new(config)?;
    let parsed = client.parse_task(description).await?;

    let mut todo = Todo::new(parsed.title, None);
    todo.description = parsed.description;
    todo.due_date = parsed.due_date;
    todo.reminder_at = parsed.reminder_at;

    if let Some(p) = parsed.priority {
        todo.priority = match p {
            1 => Priority::Low,
            3 => Priority::High,
            _ => Priority::Medium,
        };
    }

    // Store AI metadata for debugging/learning
    todo.ai_metadata = Some(serde_json::json!({
        "original_input": description,
        "parsed_category": parsed.category,
    }));

    Ok(todo)
}

async fn get_or_create_category(db: &LocalDb, name: &str, user_id: Option<Uuid>) -> Result<Uuid> {
    if let Some(existing) = db.get_category_by_name(name).await? {
        return Ok(existing.id);
    }

    // Create new category
    let category = Category::new(
        user_id.unwrap_or_else(Uuid::new_v4),
        name.to_string(),
    );
    db.create_category(&category).await?;

    Ok(category.id)
}

async fn find_category_name(db: &LocalDb, id: Uuid) -> Result<Option<String>> {
    let categories = db.list_categories().await?;
    Ok(categories.into_iter().find(|c| c.id == id).map(|c| c.name))
}
```

**Step 2: Verify compilation**

Run: `cargo build -p todoee-cli`
Expected: Successful compilation

**Step 3: Test add command manually**

Run: `cargo run -p todoee-cli -- add "test task" --no-ai`
Expected: Task created with confirmation message

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/commands/add.rs
git commit -m "feat: implement add command with AI parsing support"
```

---

### Task 9: Implement List Command

**Files:**
- Modify: `crates/todoee-cli/src/commands/list.rs`

**Step 1: Implement the list command**

Replace `crates/todoee-cli/src/commands/list.rs`:
```rust
use anyhow::{Context, Result};
use chrono::Utc;
use todoee_core::{Config, LocalDb, Priority, Todo};

pub async fn run(today: bool, category: Option<String>, all: bool) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;
    let db = LocalDb::new(&db_path).await?;

    let todos = if today {
        db.list_todos_due_today().await?
    } else if let Some(cat_name) = &category {
        // Find category by name
        if let Some(cat) = db.get_category_by_name(cat_name).await? {
            db.list_todos_by_category(cat.id).await?
        } else {
            println!("Category '{}' not found.", cat_name);
            return Ok(());
        }
    } else {
        db.list_todos(!all).await?
    };

    if todos.is_empty() {
        if today {
            println!("No tasks due today. Great work!");
        } else if category.is_some() {
            println!("No tasks in this category.");
        } else {
            println!("No tasks. Add one with: todoee add \"your task\"");
        }
        return Ok(());
    }

    // Group by category
    let categories = db.list_categories().await?;
    let mut grouped: std::collections::HashMap<Option<String>, Vec<&Todo>> = std::collections::HashMap::new();

    for todo in &todos {
        let cat_name = todo.category_id
            .and_then(|cid| categories.iter().find(|c| c.id == cid))
            .map(|c| c.name.clone());
        grouped.entry(cat_name).or_default().push(todo);
    }

    // Sort categories alphabetically, with None (uncategorized) last
    let mut cat_names: Vec<_> = grouped.keys().cloned().collect();
    cat_names.sort_by(|a, b| match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(a), Some(b)) => a.cmp(b),
    });

    let now = Utc::now();

    for cat_name in cat_names {
        let display_name = cat_name.as_deref().unwrap_or("Uncategorized");
        println!("\n{}:", display_name);
        println!("{}", "-".repeat(display_name.len() + 1));

        if let Some(tasks) = grouped.get(&cat_name) {
            for todo in tasks {
                print_todo(todo, &now, &config.display.date_format);
            }
        }
    }

    println!();

    Ok(())
}

fn print_todo(todo: &Todo, now: &chrono::DateTime<Utc>, date_format: &str) {
    let id_short = &todo.id.to_string()[..8];

    let priority_marker = match todo.priority {
        Priority::High => "!!!",
        Priority::Medium => "!! ",
        Priority::Low => "!  ",
    };

    let status = if todo.is_completed { "✓" } else { " " };

    let due_str = if let Some(due) = todo.due_date {
        let days_until = (due.date_naive() - now.date_naive()).num_days();
        match days_until {
            d if d < 0 => format!(" [OVERDUE by {} days]", -d),
            0 => " [TODAY]".to_string(),
            1 => " [Tomorrow]".to_string(),
            d if d <= 7 => format!(" [in {} days]", d),
            _ => format!(" [{}]", due.format(date_format)),
        }
    } else {
        String::new()
    };

    println!(
        "  [{}] {} {} {}{}",
        status,
        priority_marker,
        todo.title,
        id_short,
        due_str
    );
}
```

**Step 2: Verify compilation**

Run: `cargo build -p todoee-cli`
Expected: Successful compilation

**Step 3: Test list command**

Run: `cargo run -p todoee-cli -- list`
Expected: Shows tasks grouped by category

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/commands/list.rs
git commit -m "feat: implement list command with grouping and filters"
```

---

### Task 10: Implement Done Command

**Files:**
- Modify: `crates/todoee-cli/src/commands/done.rs`

**Step 1: Implement the done command**

Replace `crates/todoee-cli/src/commands/done.rs`:
```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb};

pub async fn run(id: &str) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;
    let db = LocalDb::new(&db_path).await?;

    // Find todo by partial ID match
    let todos = db.list_todos(true).await?;
    let matches: Vec<_> = todos
        .iter()
        .filter(|t| t.id.to_string().starts_with(id))
        .collect();

    match matches.len() {
        0 => {
            anyhow::bail!(
                "No task found with ID starting with '{}'. Use 'todoee list --all' to see all tasks.",
                id
            );
        }
        1 => {
            let mut todo = matches[0].clone();

            if todo.is_completed {
                println!("Task '{}' is already completed.", todo.title);
                return Ok(());
            }

            todo.mark_complete();
            db.update_todo(&todo).await?;

            println!("✓ Completed: {}", todo.title);
        }
        _ => {
            println!("Multiple tasks match '{}':", id);
            for t in matches {
                println!("  {} - {}", &t.id.to_string()[..8], t.title);
            }
            println!("\nPlease provide more characters to uniquely identify the task.");
        }
    }

    Ok(())
}
```

**Step 2: Verify compilation and commit**

Run: `cargo build -p todoee-cli`

```bash
git add crates/todoee-cli/src/commands/done.rs
git commit -m "feat: implement done command with partial ID matching"
```

---

### Task 11: Implement Delete Command

**Files:**
- Modify: `crates/todoee-cli/src/commands/delete.rs`

**Step 1: Implement the delete command**

Replace `crates/todoee-cli/src/commands/delete.rs`:
```rust
use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb};

pub async fn run(id: &str) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;
    let db = LocalDb::new(&db_path).await?;

    // Find todo by partial ID match
    let todos = db.list_todos(true).await?;
    let matches: Vec<_> = todos
        .iter()
        .filter(|t| t.id.to_string().starts_with(id))
        .collect();

    match matches.len() {
        0 => {
            anyhow::bail!(
                "No task found with ID starting with '{}'. Use 'todoee list --all' to see all tasks.",
                id
            );
        }
        1 => {
            let todo = matches[0];
            db.delete_todo(todo.id).await?;
            println!("✗ Deleted: {}", todo.title);
        }
        _ => {
            println!("Multiple tasks match '{}':", id);
            for t in matches {
                println!("  {} - {}", &t.id.to_string()[..8], t.title);
            }
            println!("\nPlease provide more characters to uniquely identify the task.");
        }
    }

    Ok(())
}
```

**Step 2: Commit**

```bash
git add crates/todoee-cli/src/commands/delete.rs
git commit -m "feat: implement delete command"
```

---

### Task 12: Implement Edit Command

**Files:**
- Modify: `crates/todoee-cli/src/commands/edit.rs`

**Step 1: Implement the edit command**

Replace `crates/todoee-cli/src/commands/edit.rs`:
```rust
use anyhow::{Context, Result};
use chrono::Utc;
use todoee_core::{Config, LocalDb, Priority, SyncStatus, Category};
use uuid::Uuid;

pub async fn run(
    id: &str,
    title: Option<String>,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
    if title.is_none() && category.is_none() && priority.is_none() {
        anyhow::bail!("Nothing to edit. Specify --title, --category, or --priority.");
    }

    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;
    let db = LocalDb::new(&db_path).await?;

    // Find todo by partial ID match
    let todos = db.list_todos(true).await?;
    let matches: Vec<_> = todos
        .iter()
        .filter(|t| t.id.to_string().starts_with(id))
        .collect();

    match matches.len() {
        0 => {
            anyhow::bail!(
                "No task found with ID starting with '{}'. Use 'todoee list --all' to see all tasks.",
                id
            );
        }
        1 => {
            let mut todo = matches[0].clone();
            let mut changes = Vec::new();

            if let Some(new_title) = title {
                changes.push(format!("title: '{}' → '{}'", todo.title, new_title));
                todo.title = new_title;
            }

            if let Some(cat_name) = category {
                let cat_id = get_or_create_category(&db, &cat_name).await?;
                changes.push(format!("category: → '{}'", cat_name));
                todo.category_id = Some(cat_id);
            }

            if let Some(p) = priority {
                let new_priority = match p {
                    1 => Priority::Low,
                    3 => Priority::High,
                    _ => Priority::Medium,
                };
                let old_str = format!("{:?}", todo.priority);
                let new_str = format!("{:?}", new_priority);
                changes.push(format!("priority: {} → {}", old_str, new_str));
                todo.priority = new_priority;
            }

            todo.updated_at = Utc::now();
            todo.sync_status = SyncStatus::Pending;

            db.update_todo(&todo).await?;

            println!("✓ Updated task:");
            for change in changes {
                println!("  {}", change);
            }
        }
        _ => {
            println!("Multiple tasks match '{}':", id);
            for t in matches {
                println!("  {} - {}", &t.id.to_string()[..8], t.title);
            }
            println!("\nPlease provide more characters to uniquely identify the task.");
        }
    }

    Ok(())
}

async fn get_or_create_category(db: &LocalDb, name: &str) -> Result<Uuid> {
    if let Some(existing) = db.get_category_by_name(name).await? {
        return Ok(existing.id);
    }

    let category = Category::new(Uuid::new_v4(), name.to_string());
    db.create_category(&category).await?;

    Ok(category.id)
}
```

**Step 2: Commit**

```bash
git add crates/todoee-cli/src/commands/edit.rs
git commit -m "feat: implement edit command"
```

---

### Task 13: Implement Config Command

**Files:**
- Modify: `crates/todoee-cli/src/commands/config.rs`

**Step 1: Implement the config command**

Replace `crates/todoee-cli/src/commands/config.rs`:
```rust
use anyhow::{Context, Result};
use todoee_core::Config;

pub async fn run(init: bool) -> Result<()> {
    if init {
        init_config().await
    } else {
        show_config().await
    }
}

async fn init_config() -> Result<()> {
    let path = Config::config_path()?;

    if path.exists() {
        println!("Config file already exists at: {}", path.display());
        println!("Edit it directly or delete it to reinitialize.");
        return Ok(());
    }

    let config = Config::default();
    config.save().context("Failed to save config")?;

    println!("Created config file at: {}", path.display());
    println!();
    println!("Next steps:");
    println!("1. Set your AI model in the config file:");
    println!("   [ai]");
    println!("   model = \"anthropic/claude-3-haiku\"  # or any OpenRouter model");
    println!();
    println!("2. Set environment variables:");
    println!("   export OPENROUTER_API_KEY=\"your-key\"");
    println!("   export NEON_DATABASE_URL=\"your-neon-url\"  # optional, for sync");
    println!();
    println!("3. Start adding tasks:");
    println!("   todoee add \"buy groceries tomorrow\"");

    Ok(())
}

async fn show_config() -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let path = Config::config_path()?;

    println!("Config file: {}", path.display());
    println!();

    println!("[ai]");
    println!("  provider: {}", config.ai.provider);
    println!("  model: {}", config.ai.model.as_deref().unwrap_or("<not set>"));
    println!("  api_key_env: {}", config.ai.api_key_env);

    let api_key_set = std::env::var(&config.ai.api_key_env).is_ok();
    println!("  api_key_status: {}", if api_key_set { "✓ set" } else { "✗ not set" });

    println!();
    println!("[database]");
    println!("  url_env: {}", config.database.url_env);

    let db_url_set = std::env::var(&config.database.url_env).is_ok();
    println!("  url_status: {}", if db_url_set { "✓ set" } else { "✗ not set (offline mode)" });
    println!("  local_db: {}", config.database.local_db_name);

    println!();
    println!("[notifications]");
    println!("  enabled: {}", config.notifications.enabled);
    println!("  sound: {}", config.notifications.sound);
    println!("  advance_minutes: {}", config.notifications.advance_minutes);

    println!();
    println!("[display]");
    println!("  theme: {}", config.display.theme);
    println!("  date_format: {}", config.display.date_format);

    Ok(())
}
```

**Step 2: Commit**

```bash
git add crates/todoee-cli/src/commands/config.rs
git commit -m "feat: implement config command"
```

---

### Task 14: Implement Sync Stub

**Files:**
- Modify: `crates/todoee-cli/src/commands/sync.rs`

**Step 1: Implement sync stub (cloud sync is Phase 2)**

Replace `crates/todoee-cli/src/commands/sync.rs`:
```rust
use anyhow::{Context, Result};
use todoee_core::Config;

pub async fn run() -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    // Check if cloud is configured
    let db_url = std::env::var(&config.database.url_env);

    match db_url {
        Ok(_) => {
            println!("Cloud sync is not yet implemented.");
            println!("Your tasks are stored locally and will sync when this feature is ready.");
            println!();
            println!("Local database: {}", config.local_db_path()?.display());
        }
        Err(_) => {
            println!("Cloud sync is not configured.");
            println!();
            println!("To enable sync:");
            println!("1. Create a Neon database at https://neon.tech");
            println!("2. Set the connection URL:");
            println!("   export {}=\"postgres://...\"", config.database.url_env);
            println!();
            println!("Your tasks are safely stored locally at:");
            println!("  {}", config.local_db_path()?.display());
        }
    }

    Ok(())
}
```

**Step 2: Commit**

```bash
git add crates/todoee-cli/src/commands/sync.rs
git commit -m "feat: add sync command stub with helpful setup instructions"
```

---

## Phase 4: Testing & Documentation

### Task 15: Add Integration Tests

**Files:**
- Create: `crates/todoee-core/tests/integration.rs`

**Step 1: Write integration tests**

Create `crates/todoee-core/tests/integration.rs`:
```rust
use todoee_core::{Config, LocalDb, Priority, Todo, Category};
use uuid::Uuid;

#[tokio::test]
async fn test_full_todo_workflow() {
    // Setup
    let db = LocalDb::new_in_memory().await.unwrap();
    let user_id = Uuid::new_v4();

    // Create a category
    let mut category = Category::new(user_id, "Work".to_string());
    category.is_ai_generated = true;
    db.create_category(&category).await.unwrap();

    // Create a todo in that category
    let mut todo = Todo::new("Review pull request".to_string(), Some(user_id));
    todo.category_id = Some(category.id);
    todo.priority = Priority::High;
    db.create_todo(&todo).await.unwrap();

    // List todos
    let todos = db.list_todos(false).await.unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].title, "Review pull request");

    // List by category
    let work_todos = db.list_todos_by_category(category.id).await.unwrap();
    assert_eq!(work_todos.len(), 1);

    // Mark complete
    let mut todo = todos[0].clone();
    todo.mark_complete();
    db.update_todo(&todo).await.unwrap();

    // Verify not in incomplete list
    let incomplete = db.list_todos(false).await.unwrap();
    assert_eq!(incomplete.len(), 0);

    // But in all list
    let all = db.list_todos(true).await.unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].is_completed);
}

#[tokio::test]
async fn test_sync_status_tracking() {
    let db = LocalDb::new_in_memory().await.unwrap();

    // New todos start as pending
    let todo = Todo::new("Sync test".to_string(), None);
    db.create_todo(&todo).await.unwrap();

    let pending = db.list_pending_sync().await.unwrap();
    assert_eq!(pending.len(), 1);

    // Mark as synced
    db.mark_synced(todo.id).await.unwrap();

    let pending = db.list_pending_sync().await.unwrap();
    assert_eq!(pending.len(), 0);
}

#[test]
fn test_config_defaults() {
    let config = Config::default();

    assert_eq!(config.ai.provider, "openrouter");
    assert!(config.ai.model.is_none()); // User must set this
    assert!(config.notifications.enabled);
}
```

**Step 2: Run tests**

Run: `cargo test -p todoee-core --test integration`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add crates/todoee-core/tests/
git commit -m "test: add integration tests for core workflow"
```

---

### Task 16: Final Build Verification

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests PASS

**Step 2: Build release binaries**

Run: `cargo build --release`
Expected: Successful compilation

**Step 3: Test the CLI end-to-end**

```bash
# Initialize config
./target/release/todoee config --init

# Add a task (without AI since no key)
./target/release/todoee add "test task for MVP" --no-ai --category "Testing" --priority 3

# List tasks
./target/release/todoee list

# Mark done
./target/release/todoee done <first-8-chars-of-id>

# Verify completion
./target/release/todoee list --all
```

**Step 4: Commit any final fixes**

```bash
git add -A
git commit -m "chore: MVP complete - offline-first todo CLI with AI parsing"
```

---

## Summary

This plan implements the Todoee MVP with:

1. **Offline-first architecture** - SQLite local database with sync status tracking
2. **AI-powered parsing** - OpenRouter integration with user-configurable model
3. **Core CLI commands** - add, list, done, delete, edit, sync, config
4. **Helpful error messages** - Clear guidance when AI or config fails
5. **Test coverage** - Unit tests and integration tests

**Not included in MVP (future phases):**
- Cloud sync to Neon Postgres
- Background daemon for notifications
- Interactive TUI mode
- Calendar/events
- Shell shortcuts

**To use AI features:**
1. Get an OpenRouter API key
2. Run `todoee config --init`
3. Edit `~/.config/todoee/config.toml` and set `model = "your-model-string"`
4. Set `export OPENROUTER_API_KEY="your-key"`
5. Run `todoee add "your natural language task"`
