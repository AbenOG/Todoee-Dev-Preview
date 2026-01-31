use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use todoee_core::{
    Category, Config, EntityType, LocalDb, Operation, OperationType, Priority, SyncStatus, Todo,
};
use uuid::Uuid;

pub async fn run(
    id: String,
    title: Option<String>,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
    // Validate that at least one field is being edited
    if title.is_none() && category.is_none() && priority.is_none() {
        anyhow::bail!("At least one of --title, --category, or --priority must be provided");
    }

    if let Some(ref t) = title
        && t.trim().is_empty()
    {
        anyhow::bail!("Title cannot be empty");
    }

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
            let mut changes: Vec<String> = Vec::new();

            // Save previous state for undo support
            let prev_state = serde_json::to_value(&todo)?;

            // Update title if provided
            if let Some(new_title) = title {
                let old_title = todo.title.clone();
                todo.title = new_title.clone();
                changes.push(format!("Title: '{}' -> '{}'", old_title, new_title));
            }

            // Update category if provided (create if doesn't exist)
            if let Some(cat_name) = category {
                let cat_id = get_or_create_category(&db, &cat_name, None).await?;
                let old_cat = match todo.category_id {
                    Some(old_id) => find_category_name(&db, old_id)
                        .await?
                        .unwrap_or_else(|| "Unknown".to_string()),
                    None => "None".to_string(),
                };
                todo.category_id = Some(cat_id);
                changes.push(format!("Category: '{}' -> '{}'", old_cat, cat_name));
            }

            // Update priority if provided
            if let Some(p) = priority {
                let old_priority = match todo.priority {
                    Priority::Low => "Low",
                    Priority::Medium => "Medium",
                    Priority::High => "High",
                };
                todo.priority = match p {
                    1 => Priority::Low,
                    3 => Priority::High,
                    _ => Priority::Medium,
                };
                let new_priority = match todo.priority {
                    Priority::Low => "Low",
                    Priority::Medium => "Medium",
                    Priority::High => "High",
                };
                changes.push(format!("Priority: {} -> {}", old_priority, new_priority));
            }

            // Update timestamps and sync status
            todo.updated_at = Utc::now();
            todo.sync_status = SyncStatus::Pending;

            // Save to database
            db.update_todo(&todo).await?;

            // Record operation for undo support
            let op = Operation::new(
                OperationType::Update,
                EntityType::Todo,
                todo.id,
                Some(prev_state),
                Some(serde_json::to_value(&todo)?),
            );
            db.record_operation(&op).await?;

            // Print confirmation
            println!("\u{270E} Updated: {}", todo.title);
            println!("  ID: {}", &todo.id.to_string()[..8]);
            println!();
            println!("Changes:");
            for change in &changes {
                println!("  - {}", change);
            }
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

/// Look up or create a category by name
async fn get_or_create_category(db: &LocalDb, name: &str, user_id: Option<Uuid>) -> Result<Uuid> {
    // Check if category already exists
    if let Some(existing) = db.get_category_by_name(name).await? {
        return Ok(existing.id);
    }

    // Create new category with provided user_id or generate one
    let category = Category::new(user_id.unwrap_or_else(Uuid::new_v4), name.to_string());
    db.create_category(&category).await?;

    Ok(category.id)
}

/// Look up category name by ID
async fn find_category_name(db: &LocalDb, id: Uuid) -> Result<Option<String>> {
    let categories = db.list_categories().await?;
    Ok(categories.into_iter().find(|c| c.id == id).map(|c| c.name))
}
