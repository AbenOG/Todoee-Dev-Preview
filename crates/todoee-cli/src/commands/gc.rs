//! Garbage collection command for cleaning up old data.

use std::fs;

use anyhow::{Context, Result};
use chrono::Utc;
use todoee_core::{Config, LocalDb};

pub async fn run(days: Option<i64>, dry_run: bool) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path()?;

    if let Some(parent) = db_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    let days = days.unwrap_or(30);

    // Get counts for stats
    let all_todos = db.list_todos(false).await?;
    let pending = all_todos.iter().filter(|t| !t.is_completed).count();
    let completed = all_todos.iter().filter(|t| t.is_completed).count();

    println!("Database stats:");
    println!("  Total todos:  {}", all_todos.len());
    println!("  Pending:      {}", pending);
    println!("  Completed:    {}", completed);
    println!();

    let cutoff = Utc::now() - chrono::Duration::days(days);

    if dry_run {
        println!("Dry run - would clean items older than {} days", days);

        // Count what would be deleted
        let old_completed = all_todos
            .iter()
            .filter(|t| t.is_completed && t.completed_at.map(|c| c < cutoff).unwrap_or(false))
            .count();

        println!("Would delete:");
        println!("  {} old completed todo(s)", old_completed);
        println!("  Old operation history");
        return Ok(());
    }

    // Delete old operations
    let deleted_ops = db.clear_old_operations(days).await?;

    // Delete old completed todos
    let mut deleted_todos = 0;
    for todo in all_todos.iter().filter(|t| t.is_completed) {
        if let Some(completed_at) = todo.completed_at
            && completed_at < cutoff
        {
            db.delete_todo(todo.id).await?;
            deleted_todos += 1;
        }
    }

    println!("Cleanup complete:");
    println!("  Deleted {} old operation(s)", deleted_ops);
    println!("  Deleted {} old completed todo(s)", deleted_todos);

    Ok(())
}
