//! Head and tail commands for listing todos by creation date.

use std::fs;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use todoee_core::{Config, LocalDb, Priority, Todo};

/// Show the N most recently created todos.
pub async fn head(count: usize, all: bool) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;

    if let Some(parent) = db_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    let todos = db.list_todos_head(count, all).await?;

    if todos.is_empty() {
        println!("No todos.");
        return Ok(());
    }

    println!("Last {} todos:\n", todos.len());
    print_todos(&todos);
    Ok(())
}

/// Show the N oldest todos.
pub async fn tail(count: usize, all: bool) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;

    if let Some(parent) = db_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    let todos = db.list_todos_tail(count, all).await?;

    if todos.is_empty() {
        println!("No todos.");
        return Ok(());
    }

    println!("Oldest {} todos:\n", todos.len());
    print_todos(&todos);
    Ok(())
}

fn print_todos(todos: &[Todo]) {
    for todo in todos {
        let check = if todo.is_completed {
            "\x1b[32m[x]\x1b[0m"
        } else {
            "[ ]"
        };

        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };

        let id = &todo.id.to_string()[..8];
        let age = format_age(todo.created_at);

        println!(
            "{} {} \x1b[90m{}\x1b[0m {} \x1b[90m({})\x1b[0m",
            check, pri, id, todo.title, age
        );
    }
}

fn format_age(dt: DateTime<Utc>) -> String {
    let diff = Utc::now().signed_duration_since(dt);
    let days = diff.num_days();

    if days > 0 {
        format!("{}d ago", days)
    } else {
        let hours = diff.num_hours();
        if hours > 0 {
            format!("{}h ago", hours)
        } else {
            "just now".to_string()
        }
    }
}
