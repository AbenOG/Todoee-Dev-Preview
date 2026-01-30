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

    let Some(op) = db.get_last_redoable_operation().await? else {
        println!("Nothing to redo");
        return Ok(());
    };

    match (op.operation_type, op.entity_type) {
        (OperationType::Create, EntityType::Todo) => {
            if let Some(new) = &op.new_state {
                let todo: Todo = serde_json::from_value(new.clone())?;
                db.create_todo(&todo).await?;
                println!("\u{21aa} Redone create: \"{}\"", todo.title);
            }
        }
        (OperationType::Delete, EntityType::Todo) => {
            db.delete_todo(op.entity_id).await?;
            let title = op
                .previous_state
                .as_ref()
                .and_then(|s| s.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("todo");
            println!("\u{21aa} Redone delete: \"{}\"", title);
        }
        (OperationType::Update, EntityType::Todo) => {
            if let Some(new) = &op.new_state {
                let todo: Todo = serde_json::from_value(new.clone())?;
                db.update_todo(&todo).await?;
                println!("\u{21aa} Redone edit: \"{}\"", todo.title);
            }
        }
        (OperationType::Complete, EntityType::Todo) => {
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_complete();
                db.update_todo(&todo).await?;
                println!("\u{21aa} Redone complete: \"{}\" is done again", todo.title);
            }
        }
        (OperationType::Uncomplete, EntityType::Todo) => {
            if let Some(mut todo) = db.get_todo(op.entity_id).await? {
                todo.mark_incomplete();
                db.update_todo(&todo).await?;
                println!(
                    "\u{21aa} Redone uncomplete: \"{}\" is pending again",
                    todo.title
                );
            }
        }
        (OperationType::Stash, EntityType::Todo) => {
            db.delete_todo(op.entity_id).await?;
            let title = op
                .previous_state
                .as_ref()
                .and_then(|s| s.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("todo");
            println!("\u{21aa} Redone stash: \"{}\" stashed again", title);
        }
        _ => println!("Cannot redo this operation type"),
    }

    db.mark_operation_redone(op.id).await?;
    Ok(())
}
