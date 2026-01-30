//! Show detailed view of a single todo.

use std::fs;

use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use todoee_core::{Config, LocalDb, Priority, Todo};

/// Run the show command to display detailed info about a todo.
pub async fn run(id: &str) -> Result<()> {
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

    // Find todos matching the partial ID (include completed)
    let all_todos = db.list_todos(false).await?;
    let id_lower = id.to_lowercase();

    let matching: Vec<&Todo> = all_todos
        .iter()
        .filter(|t| t.id.to_string().to_lowercase().starts_with(&id_lower))
        .collect();

    match matching.len() {
        0 => {
            println!("No todo found with ID starting with '{}'", id);
            println!("Hint: Use 'todoee list --all' to see all todos including completed ones.");
        }
        1 => {
            let todo = matching[0];
            print_detailed_todo(&db, todo).await?;
        }
        _ => {
            println!("Multiple matches for '{}'. Be more specific:", id);
            println!();
            for t in matching {
                let status = if t.is_completed { "[x]" } else { "[ ]" };
                let short_id = &t.id.to_string()[..8];
                println!("  {} {} - {}", short_id, status, t.title);
            }
        }
    }

    Ok(())
}

/// Print detailed information about a todo in a formatted box.
async fn print_detailed_todo(db: &LocalDb, todo: &Todo) -> Result<()> {
    let categories = db.list_categories().await?;
    let cat_name = todo
        .category_id
        .and_then(|cid| categories.iter().find(|c| c.id == cid))
        .map(|c| c.name.as_str())
        .unwrap_or("None");

    let status_display = if todo.is_completed {
        "\x1b[32mCompleted\x1b[0m"
    } else {
        "\x1b[33mPending\x1b[0m"
    };

    let priority_display = match todo.priority {
        Priority::High => "\x1b[31mHigh (!!!)\x1b[0m",
        Priority::Medium => "\x1b[33mMedium (!!)\x1b[0m",
        Priority::Low => "Low (!)",
    };

    println!(
        "\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}"
    );
    println!("\u{2502} \x1b[1m{}\x1b[0m", truncate(&todo.title, 47));
    println!(
        "\u{251c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2524}"
    );
    println!("\u{2502} ID:         {}", todo.id);
    println!("\u{2502} Status:     {}", status_display);
    println!("\u{2502} Priority:   {}", priority_display);
    println!("\u{2502} Category:   {}", cat_name);

    if let Some(desc) = &todo.description {
        println!("\u{2502} Description:");
        for line in desc.lines() {
            println!("\u{2502}   {}", line);
        }
    }

    if let Some(due) = todo.due_date {
        let local = Local.from_utc_datetime(&due.naive_utc());
        println!("\u{2502} Due:        {}", local.format("%Y-%m-%d %H:%M"));
    }

    if let Some(reminder) = todo.reminder_at {
        let local = Local.from_utc_datetime(&reminder.naive_utc());
        println!("\u{2502} Reminder:   {}", local.format("%Y-%m-%d %H:%M"));
    }

    if let Some(completed) = todo.completed_at {
        let local = Local.from_utc_datetime(&completed.naive_utc());
        println!("\u{2502} Completed:  {}", local.format("%Y-%m-%d %H:%M"));
    }

    let created = Local.from_utc_datetime(&todo.created_at.naive_utc());
    let updated = Local.from_utc_datetime(&todo.updated_at.naive_utc());
    println!("\u{2502} Created:    {}", created.format("%Y-%m-%d %H:%M"));
    println!("\u{2502} Updated:    {}", updated.format("%Y-%m-%d %H:%M"));
    println!("\u{2502} Sync:       {:?}", todo.sync_status);
    println!(
        "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}"
    );

    Ok(())
}

/// Truncate a string to the given maximum length, adding "..." if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
