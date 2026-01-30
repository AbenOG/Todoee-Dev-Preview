use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use std::collections::HashMap;
use std::fs;
use todoee_core::{Category, Config, LocalDb, Priority, Todo};
use uuid::Uuid;

pub async fn run(today: bool, category: Option<String>, all: bool) -> Result<()> {
    // Load config and open local database
    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;

    // Ensure config directory exists
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }
    }

    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    // Get todos based on filters
    let todos = if today {
        db.list_todos_due_today().await?
    } else if let Some(cat_name) = &category {
        // Find category by name
        let cat = db
            .get_category_by_name(cat_name)
            .await?
            .with_context(|| format!("Category '{}' not found", cat_name))?;
        db.list_todos_by_category(cat.id).await?
    } else {
        // List all todos, exclude completed unless --all is set
        db.list_todos(!all).await?
    };

    // Handle empty results
    if todos.is_empty() {
        if today {
            println!("No tasks due today. Enjoy your free time!");
        } else if let Some(cat_name) = &category {
            println!("No tasks in category '{}'.", cat_name);
        } else if all {
            println!("No tasks found. Use 'todoee add' to create one!");
        } else {
            println!("No pending tasks. Use 'todoee add' to create one!");
        }
        return Ok(());
    }

    // Get all categories for lookup
    let categories = db.list_categories().await?;
    let category_map: HashMap<Uuid, &Category> = categories.iter().map(|c| (c.id, c)).collect();

    // Group todos by category
    let mut grouped: HashMap<Option<Uuid>, Vec<&Todo>> = HashMap::new();
    for todo in &todos {
        grouped.entry(todo.category_id).or_default().push(todo);
    }

    // Sort categories alphabetically, with Uncategorized last
    let mut sorted_categories: Vec<Option<Uuid>> = grouped.keys().cloned().collect();
    sorted_categories.sort_by(|a, b| {
        match (a, b) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater, // Uncategorized last
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(a_id), Some(b_id)) => {
                let a_name = category_map.get(a_id).map(|c| c.name.as_str()).unwrap_or("");
                let b_name = category_map.get(b_id).map(|c| c.name.as_str()).unwrap_or("");
                a_name.to_lowercase().cmp(&b_name.to_lowercase())
            }
        }
    });

    // Print each category group
    for (idx, cat_id) in sorted_categories.iter().enumerate() {
        if idx > 0 {
            println!(); // Blank line between categories
        }

        // Print category header
        let header = match cat_id {
            Some(id) => category_map
                .get(id)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown"),
            None => "Uncategorized",
        };
        println!("== {} ==", header);

        // Print todos in this category
        if let Some(todos_in_cat) = grouped.get(cat_id) {
            for todo in todos_in_cat {
                print_todo(todo);
            }
        }
    }

    Ok(())
}

/// Print a single todo item with status, priority, title, ID, and due date
fn print_todo(todo: &Todo) {
    // Status checkbox
    let checkbox = if todo.is_completed { "[x]" } else { "[ ]" };

    // Priority markers
    let priority = match todo.priority {
        Priority::High => "!!! ",
        Priority::Medium => "!! ",
        Priority::Low => "! ",
    };

    // Short ID (first 8 characters of UUID)
    let short_id = &todo.id.to_string()[..8];

    // Due date formatting
    let due_info = format_due_date(todo.due_date);

    // Build the output line
    let mut line = format!("{} {}{} [{}]", checkbox, priority, todo.title, short_id);
    if !due_info.is_empty() {
        line.push_str(&format!(" {}", due_info));
    }

    println!("  {}", line);
}

/// Format due date info for display
fn format_due_date(due_date: Option<DateTime<Utc>>) -> String {
    let Some(due) = due_date else {
        return String::new();
    };

    let now = Utc::now();
    let today = now.date_naive();
    let due_date_naive = due.date_naive();

    let days_diff = (due_date_naive - today).num_days();

    if days_diff < 0 {
        // Overdue
        let days_overdue = -days_diff;
        if days_overdue == 1 {
            "[OVERDUE by 1 day]".to_string()
        } else {
            format!("[OVERDUE by {} days]", days_overdue)
        }
    } else if days_diff == 0 {
        "[TODAY]".to_string()
    } else if days_diff == 1 {
        "[Tomorrow]".to_string()
    } else if days_diff <= 7 {
        format!("[in {} days]", days_diff)
    } else {
        // Further out: show formatted date
        let local_due: DateTime<Local> = due.into();
        format!("[{}]", local_due.format("%Y-%m-%d"))
    }
}
