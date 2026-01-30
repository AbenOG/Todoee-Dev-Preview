use anyhow::{Context, Result};
use std::fs;
use todoee_core::{Config, EntityType, LocalDb, OperationType, Todo};

pub async fn run() -> Result<()> {
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

    let Some(op) = db.get_last_undoable_operation().await? else {
        println!("Nothing to undo");
        return Ok(());
    };

    match (op.operation_type, op.entity_type) {
        (OperationType::Create, EntityType::Todo) => {
            db.delete_todo(op.entity_id).await?;
            let title = op
                .new_state
                .as_ref()
                .and_then(|s| s.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("todo");
            println!("\u{21a9} Undone create: deleted \"{}\"", title);
        }
        (OperationType::Delete, EntityType::Todo) => {
            if let Some(prev) = &op.previous_state {
                let todo: Todo = serde_json::from_value(prev.clone())?;
                db.create_todo(&todo).await?;
                println!("\u{21a9} Undone delete: restored \"{}\"", todo.title);
            }
        }
        (OperationType::Update, EntityType::Todo) => {
            if let Some(prev) = &op.previous_state {
                let todo: Todo = serde_json::from_value(prev.clone())?;
                db.update_todo(&todo).await?;
                println!("\u{21a9} Undone edit: reverted \"{}\"", todo.title);
            }
        }
        (OperationType::Complete, EntityType::Todo) => {
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_incomplete();
                db.update_todo(&todo).await?;
                println!(
                    "\u{21a9} Undone complete: \"{}\" is pending again",
                    todo.title
                );
            }
        }
        (OperationType::Uncomplete, EntityType::Todo) => {
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_complete();
                db.update_todo(&todo).await?;
                println!("\u{21a9} Undone uncomplete: \"{}\" is done again", todo.title);
            }
        }
        (OperationType::Stash, EntityType::Todo) => {
            if let Some(prev) = &op.previous_state {
                let todo: Todo = serde_json::from_value(prev.clone())?;
                db.create_todo(&todo).await?;
                println!("\u{21a9} Undone stash: \"{}\" restored", todo.title);
            }
        }
        _ => println!("Cannot undo this operation type"),
    }

    db.mark_operation_undone(op.id).await?;
    Ok(())
}
