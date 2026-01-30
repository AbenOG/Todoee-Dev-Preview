//! Batch commands for operating on multiple todos at once.

use anyhow::Result;
use clap::Subcommand;
use todoee_core::{Config, EntityType, LocalDb, Operation, OperationType, Priority};

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

pub async fn run(cmd: BatchCommand) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.local_db_path()?;
    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    match cmd {
        BatchCommand::Done { ids } => {
            // For done, we only need incomplete todos (exclude_completed = true)
            let todos = db.list_todos(true).await?;
            let mut count = 0;
            for id in &ids {
                let id_lower = id.to_lowercase();
                if let Some(todo) = todos
                    .iter()
                    .find(|t| t.id.to_string().to_lowercase().starts_with(&id_lower))
                {
                    let mut updated = todo.clone();
                    let prev = serde_json::to_value(&updated)?;
                    updated.mark_complete();
                    db.update_todo(&updated).await?;

                    let op = Operation::new(
                        OperationType::Complete,
                        EntityType::Todo,
                        todo.id,
                        Some(prev),
                        None,
                    );
                    db.record_operation(&op).await?;
                    count += 1;
                    println!("\u{2713} {}", todo.title);
                } else {
                    println!("Not found: {}", id);
                }
            }
            println!("\nMarked {} todo(s) as done", count);
        }
        BatchCommand::Delete { ids } => {
            // Include all todos for delete (exclude_completed = false)
            let todos = db.list_todos(false).await?;
            let mut count = 0;
            for id in &ids {
                let id_lower = id.to_lowercase();
                if let Some(todo) = todos
                    .iter()
                    .find(|t| t.id.to_string().to_lowercase().starts_with(&id_lower))
                {
                    let op = Operation::new(
                        OperationType::Delete,
                        EntityType::Todo,
                        todo.id,
                        Some(serde_json::to_value(todo)?),
                        None,
                    );
                    db.record_operation(&op).await?;
                    db.delete_todo(todo.id).await?;
                    count += 1;
                    println!("\u{2717} {}", todo.title);
                } else {
                    println!("Not found: {}", id);
                }
            }
            println!("\nDeleted {} todo(s)", count);
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

            // Include all todos for priority changes (exclude_completed = false)
            let todos = db.list_todos(false).await?;
            let mut count = 0;
            for id in &ids {
                let id_lower = id.to_lowercase();
                if let Some(todo) = todos
                    .iter()
                    .find(|t| t.id.to_string().to_lowercase().starts_with(&id_lower))
                {
                    let mut updated = todo.clone();
                    let prev = serde_json::to_value(&updated)?;
                    updated.priority = priority;
                    db.update_todo(&updated).await?;

                    let op = Operation::new(
                        OperationType::Update,
                        EntityType::Todo,
                        todo.id,
                        Some(prev),
                        Some(serde_json::to_value(&updated)?),
                    );
                    db.record_operation(&op).await?;
                    count += 1;
                    println!("~ {} (now {:?})", todo.title, priority);
                } else {
                    println!("Not found: {}", id);
                }
            }
            println!("\nUpdated priority for {} todo(s)", count);
        }
    }

    Ok(())
}
