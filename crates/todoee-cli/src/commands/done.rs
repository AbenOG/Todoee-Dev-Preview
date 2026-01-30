use anyhow::{Context, Result};
use std::fs;
use todoee_core::{Config, EntityType, LocalDb, Operation, OperationType, Todo};

pub async fn run(id: String) -> Result<()> {
    // Load config and open local database
    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;

    // Ensure config directory exists
    if let Some(parent) = db_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    // Find todos matching the partial ID
    let matches = find_todos_by_partial_id(&db, &id).await?;

    match matches.len() {
        0 => {
            // No match found
            eprintln!("No todo found matching '{}'", id);
            eprintln!("Hint: Use 'todoee list --all' to see all todos including completed ones.");
            anyhow::bail!("Todo not found");
        }
        1 => {
            // Single match found
            let mut todo = matches.into_iter().next().unwrap();

            if todo.is_completed {
                println!("Todo '{}' is already marked as complete.", todo.title);
                return Ok(());
            }

            // Save previous state for undo support
            let prev_state = serde_json::to_value(&todo)?;

            // Mark as complete
            todo.mark_complete();
            db.update_todo(&todo).await?;

            // Record operation for undo support
            let new_state = serde_json::to_value(&todo)?;
            let op = Operation::new(
                if todo.is_completed {
                    OperationType::Complete
                } else {
                    OperationType::Uncomplete
                },
                EntityType::Todo,
                todo.id,
                Some(prev_state),
                Some(new_state),
            );
            db.record_operation(&op).await?;

            println!("\u{2713} Completed: {}", todo.title);
            println!("  ID: {}", &todo.id.to_string()[..8]);
        }
        _ => {
            // Multiple matches - ask for more specific ID
            eprintln!("Multiple todos match '{}'. Please be more specific:", id);
            eprintln!();
            for todo in &matches {
                let status = if todo.is_completed { "[x]" } else { "[ ]" };
                let short_id = &todo.id.to_string()[..8];
                eprintln!("  {} {} [{}]", status, todo.title, short_id);
            }
            anyhow::bail!("Ambiguous ID - provide more characters");
        }
    }

    Ok(())
}

/// Find todos whose ID starts with the given prefix
async fn find_todos_by_partial_id(db: &LocalDb, prefix: &str) -> Result<Vec<Todo>> {
    let prefix_lower = prefix.to_lowercase();
    let all_todos = db.list_todos(false).await?;

    let matches: Vec<Todo> = all_todos
        .into_iter()
        .filter(|todo| {
            todo.id
                .to_string()
                .to_lowercase()
                .starts_with(&prefix_lower)
        })
        .collect();

    Ok(matches)
}
