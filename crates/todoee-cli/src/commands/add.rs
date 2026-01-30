use anyhow::{Context, Result};
use std::fs;
use todoee_core::{AiClient, Category, Config, LocalDb, Priority, Todo};
use uuid::Uuid;

pub async fn run(
    description: Vec<String>,
    no_ai: bool,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
    // Join description parts into a single string
    let description = description.join(" ");

    // Validate description is not empty
    if description.trim().is_empty() {
        anyhow::bail!("Task description cannot be empty");
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

    // Create todo based on whether AI is disabled or no model is configured
    let mut todo = if no_ai || config.ai.model.is_none() {
        // Manual mode: create task directly from input
        Todo::new(description.clone(), None)
    } else {
        // AI mode: parse natural language with graceful fallback
        match parse_with_ai(&config, &description).await {
            Ok(todo) => todo,
            Err(e) => {
                eprintln!("AI parsing failed: {}", e);
                eprintln!("Creating task with original text instead.");
                Todo::new(description.clone(), None)
            }
        }
    };

    // Override category if manually specified (create if doesn't exist)
    if let Some(cat_name) = category {
        let cat_id = get_or_create_category(&db, &cat_name, None).await?;
        todo.category_id = Some(cat_id);
    }

    // Override priority if manually specified (1=Low, 2=Medium, 3=High)
    if let Some(p) = priority {
        todo.priority = match p {
            1 => Priority::Low,
            3 => Priority::High,
            _ => Priority::Medium,
        };
    }

    // Save todo to database
    db.create_todo(&todo).await?;

    // Print confirmation with checkmark emoji
    println!("\u{2713} Created: {}", todo.title);

    // Print category if any
    if let Some(cat_id) = todo.category_id
        && let Some(cat_name) = find_category_name(&db, cat_id).await?
    {
        println!("  Category: {}", cat_name);
    }

    // Print due date if any
    if let Some(due) = todo.due_date {
        println!("  Due: {}", due.format("%Y-%m-%d %H:%M"));
    }

    // Print priority
    let priority_str = match todo.priority {
        Priority::Low => "Low",
        Priority::Medium => "Medium",
        Priority::High => "High",
    };
    println!("  Priority: {}", priority_str);

    // Print short ID (first 8 chars of UUID)
    println!("  ID: {}", &todo.id.to_string()[..8]);

    Ok(())
}

/// Parse natural language input using AI and convert to Todo
async fn parse_with_ai(config: &Config, description: &str) -> Result<Todo> {
    let client = AiClient::new(config)?;
    let parsed = client.parse_task(description).await?;

    let mut todo = Todo::new(parsed.title, None);
    todo.description = parsed.description;
    todo.due_date = parsed.due_date;
    todo.reminder_at = parsed.reminder_at;

    // Convert AI priority to internal Priority enum
    if let Some(p) = parsed.priority {
        todo.priority = match p {
            1 => Priority::High,  // AI uses 1=highest
            2 => Priority::High,
            3 => Priority::Medium,
            _ => Priority::Low,
        };
    }

    // Store AI metadata for debugging/learning
    todo.ai_metadata = Some(serde_json::json!({
        "original_input": description,
        "parsed_category": parsed.category,
    }));

    Ok(todo)
}

/// Look up or create a category by name
async fn get_or_create_category(
    db: &LocalDb,
    name: &str,
    user_id: Option<Uuid>,
) -> Result<Uuid> {
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
