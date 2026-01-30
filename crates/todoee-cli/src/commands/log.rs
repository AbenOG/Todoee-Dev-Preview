//! View operation history command.

use std::fs;

use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use todoee_core::{Config, LocalDb};

pub async fn run(limit: Option<usize>, oneline: bool) -> Result<()> {
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

    let operations = db.list_operations(limit.unwrap_or(10)).await?;

    if operations.is_empty() {
        println!("No operations recorded yet.");
        return Ok(());
    }

    for op in operations {
        let time = Local.from_utc_datetime(&op.created_at.naive_utc());
        let short_id = &op.id.to_string()[..7];
        let entity_short = &op.entity_id.to_string()[..8];

        let title = op
            .new_state
            .as_ref()
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
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
