//! Show recent changes command.

use std::fs;

use anyhow::{Context, Result};
use chrono::{Local, TimeZone, Utc};
use todoee_core::{Config, LocalDb, OperationType};

pub async fn run(hours: Option<i64>) -> Result<()> {
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

    let hours = hours.unwrap_or(24);
    let since = Utc::now() - chrono::Duration::hours(hours);
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

        let title = op
            .new_state
            .as_ref()
            .or(op.previous_state.as_ref())
            .and_then(|s| s.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or("?");

        match op.operation_type {
            OperationType::Create => {
                println!(
                    "\x1b[32m+ {}\x1b[0m {} {}",
                    time.format("%H:%M"),
                    short_id,
                    title
                );
                creates += 1;
            }
            OperationType::Delete => {
                println!(
                    "\x1b[31m- {}\x1b[0m {} {}",
                    time.format("%H:%M"),
                    short_id,
                    title
                );
                deletes += 1;
            }
            OperationType::Update => {
                println!(
                    "\x1b[33m~ {}\x1b[0m {} {}",
                    time.format("%H:%M"),
                    short_id,
                    title
                );
                updates += 1;
            }
            OperationType::Complete => {
                println!(
                    "\x1b[32m\u{2713} {}\x1b[0m {} {}",
                    time.format("%H:%M"),
                    short_id,
                    title
                );
                completes += 1;
            }
            OperationType::Uncomplete => {
                println!(
                    "\x1b[33m\u{25cb} {}\x1b[0m {} {}",
                    time.format("%H:%M"),
                    short_id,
                    title
                );
            }
            OperationType::Stash | OperationType::Unstash => {}
        }
    }

    println!("\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!(
        "\x1b[32m+{} created\x1b[0m  \x1b[33m~{} updated\x1b[0m  \x1b[31m-{} deleted\x1b[0m  \x1b[32m\u{2713}{} completed\x1b[0m",
        creates, updates, deletes, completes
    );

    Ok(())
}
