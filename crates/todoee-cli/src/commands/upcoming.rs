//! Upcoming and overdue commands for listing todos by due date.

use std::fs;

use anyhow::{Context, Result};
use chrono::{Local, TimeZone, Utc};
use todoee_core::{Config, LocalDb, Priority, Todo};

/// Show the next N upcoming todos by due date.
pub async fn upcoming(count: usize) -> Result<()> {
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

    let todos = db.list_todos_upcoming(count).await?;

    if todos.is_empty() {
        println!("No upcoming todos with due dates.");
        return Ok(());
    }

    println!("Next {} upcoming:\n", todos.len());
    print_upcoming_todos(&todos);
    Ok(())
}

/// Show all overdue todos.
pub async fn overdue() -> Result<()> {
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

    let todos = db.list_todos_overdue().await?;

    if todos.is_empty() {
        println!("\x1b[32mNo overdue todos!\x1b[0m");
        return Ok(());
    }

    println!("\x1b[31m{} overdue:\x1b[0m\n", todos.len());
    print_overdue_todos(&todos);
    Ok(())
}

fn print_upcoming_todos(todos: &[Todo]) {
    for todo in todos {
        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };

        let id = &todo.id.to_string()[..8];

        let due = todo
            .due_date
            .map(|d| {
                let local = Local.from_utc_datetime(&d.naive_utc());
                let diff = d.signed_duration_since(Utc::now());
                let days = diff.num_days();

                let label = if days == 0 {
                    "TODAY".to_string()
                } else if days == 1 {
                    "tomorrow".to_string()
                } else {
                    format!("in {}d", days)
                };

                format!("{} ({})", local.format("%m-%d %H:%M"), label)
            })
            .unwrap_or_default();

        println!(
            "{} \x1b[90m{}\x1b[0m {} \x1b[36m[{}]\x1b[0m",
            pri, id, todo.title, due
        );
    }
}

fn print_overdue_todos(todos: &[Todo]) {
    for todo in todos {
        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };

        let id = &todo.id.to_string()[..8];

        let overdue_by = todo
            .due_date
            .map(|d| {
                let diff = Utc::now().signed_duration_since(d);
                let days = diff.num_days();

                if days > 0 {
                    format!("{} days overdue", days)
                } else {
                    format!("{} hours overdue", diff.num_hours())
                }
            })
            .unwrap_or_default();

        println!(
            "{} \x1b[90m{}\x1b[0m {} \x1b[31m[{}]\x1b[0m",
            pri, id, todo.title, overdue_by
        );
    }
}
