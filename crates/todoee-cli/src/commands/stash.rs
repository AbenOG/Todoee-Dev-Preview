//! Stash commands for temporarily hiding todos.

use anyhow::Result;
use clap::Subcommand;
use todoee_core::{Config, EntityType, LocalDb, Operation, OperationType};

#[derive(Subcommand, Clone)]
pub enum StashCommand {
    /// Stash a todo by ID
    Push {
        /// Todo ID (or prefix)
        id: String,
        /// Optional message describing why this todo is stashed
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Restore the most recently stashed todo
    Pop,
    /// List all stashed todos
    List,
    /// Clear all stashed todos
    Clear,
}

pub async fn run(cmd: StashCommand) -> Result<()> {
    let config = Config::load()?;
    let db_path = config.local_db_path()?;
    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    match cmd {
        StashCommand::Push { id, message } => push(&db, &id, message.as_deref()).await,
        StashCommand::Pop => pop(&db).await,
        StashCommand::List => list(&db).await,
        StashCommand::Clear => clear(&db).await,
    }
}

async fn push(db: &LocalDb, id: &str, message: Option<&str>) -> Result<()> {
    let todos = db.list_todos(false).await?;
    let matching: Vec<_> = todos
        .iter()
        .filter(|t| t.id.to_string().starts_with(id))
        .collect();

    match matching.len() {
        0 => {
            println!("No todo found with ID '{}'", id);
        }
        1 => {
            let todo = db.stash_todo(matching[0].id, message).await?;

            let op = Operation::new(
                OperationType::Stash,
                EntityType::Todo,
                todo.id,
                Some(serde_json::to_value(&todo)?),
                None,
            );
            db.record_operation(&op).await?;

            let msg_display = message.map(|m| format!(": {}", m)).unwrap_or_default();
            println!("Stashed{}: {}", msg_display, todo.title);
        }
        _ => {
            println!("Multiple matches. Be more specific:");
            for t in matching {
                println!("  {} - {}", &t.id.to_string()[..8], t.title);
            }
        }
    }

    Ok(())
}

async fn pop(db: &LocalDb) -> Result<()> {
    match db.stash_pop().await? {
        Some(todo) => {
            let op = Operation::new(
                OperationType::Unstash,
                EntityType::Todo,
                todo.id,
                None,
                Some(serde_json::to_value(&todo)?),
            );
            db.record_operation(&op).await?;
            println!("Restored: {}", todo.title);
        }
        None => {
            println!("Stash is empty");
        }
    }

    Ok(())
}

async fn list(db: &LocalDb) -> Result<()> {
    let stashed = db.stash_list().await?;

    if stashed.is_empty() {
        println!("Stash is empty");
    } else {
        println!("Stashed todos:\n");
        for (i, (todo, _at, msg)) in stashed.iter().enumerate() {
            let msg_str = msg.as_ref().map(|m| format!(": {}", m)).unwrap_or_default();
            println!("stash@{{{}}}{}", i, msg_str);
            println!("  {}", todo.title);
        }
    }

    Ok(())
}

async fn clear(db: &LocalDb) -> Result<()> {
    let count = db.stash_clear().await?;
    println!("Cleared {} stashed todo(s)", count);
    Ok(())
}
