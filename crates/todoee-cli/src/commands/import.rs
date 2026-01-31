use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use todoee_core::{Category, Config, LocalDb, Todo};

#[derive(Debug, Clone, Copy)]
pub enum ImportMode {
    Merge,   // Skip existing IDs
    Replace, // Overwrite existing
}

#[derive(Deserialize)]
struct ImportData {
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    exported_at: String,
    todos: Vec<Todo>,
    categories: Vec<Category>,
}

/// Import todos from a file.
///
/// Imports todos and categories from a JSON file. This is the public API
/// for programmatic import, used by tests and intended for future sync functionality.
///
/// Returns a tuple of (imported_todos_count, imported_categories_count).
#[allow(dead_code)]
pub async fn import_todos(
    db: &LocalDb,
    input_path: &Path,
    mode: ImportMode,
) -> Result<(usize, usize)> {
    // Read and parse JSON
    let content = fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read import file: {}", input_path.display()))?;

    let data: ImportData = serde_json::from_str(&content)
        .context("Failed to parse import JSON")?;

    let mut imported_categories = 0;
    let mut imported_todos = 0;

    // Import categories first (todos may reference them)
    for category in data.categories {
        let existing = db.get_category_by_name(&category.name).await?;
        match (existing, mode) {
            (Some(_), ImportMode::Merge) => {
                // Skip existing category in merge mode
            }
            (Some(existing_cat), ImportMode::Replace) => {
                // Delete and recreate in replace mode
                db.delete_category(existing_cat.id).await?;
                db.create_category(&category).await?;
                imported_categories += 1;
            }
            (None, _) => {
                db.create_category(&category).await?;
                imported_categories += 1;
            }
        }
    }

    // Import todos with merge/replace logic
    for todo in data.todos {
        let existing = db.get_todo(todo.id).await?;
        match (existing, mode) {
            (Some(_), ImportMode::Merge) => {
                // Skip existing todo in merge mode
            }
            (Some(_), ImportMode::Replace) => {
                // Update existing in replace mode
                db.update_todo(&todo).await?;
                imported_todos += 1;
            }
            (None, _) => {
                db.create_todo(&todo).await?;
                imported_todos += 1;
            }
        }
    }

    Ok((imported_todos, imported_categories))
}

pub async fn run(input: String, mode: String) -> Result<()> {
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

    // Parse mode string to ImportMode
    let import_mode = match mode.to_lowercase().as_str() {
        "replace" => ImportMode::Replace,
        _ => ImportMode::Merge,
    };

    let input_path = Path::new(&input);
    let (todos_count, categories_count) = import_todos(&db, input_path, import_mode).await?;

    println!(
        "\u{2713} Imported {} todos and {} categories from {}",
        todos_count, categories_count, input
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_import_json_creates_todos() {
        let temp_dir = TempDir::new().unwrap();
        let import_path = temp_dir.path().join("import.json");

        // Create a JSON file to import matching the export format
        let json_data = r#"{
            "version": "1.0",
            "exported_at": "2026-01-31T12:00:00Z",
            "todos": [
                {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "title": "Imported task",
                    "priority": "medium",
                    "is_completed": false,
                    "created_at": "2026-01-31T12:00:00Z",
                    "updated_at": "2026-01-31T12:00:00Z",
                    "sync_status": "pending"
                }
            ],
            "categories": []
        }"#;
        std::fs::write(&import_path, json_data).unwrap();

        let db_path = temp_dir.path().join("test.db");
        let db = LocalDb::new(&db_path).await.unwrap();
        db.run_migrations().await.unwrap();

        let result = import_todos(&db, &import_path, ImportMode::Merge).await;
        assert!(result.is_ok());

        let todos = db.list_todos(false).await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].title, "Imported task");
    }

    #[tokio::test]
    async fn test_import_merge_skips_existing() {
        let temp_dir = TempDir::new().unwrap();
        let import_path = temp_dir.path().join("import.json");

        let db_path = temp_dir.path().join("test.db");
        let db = LocalDb::new(&db_path).await.unwrap();
        db.run_migrations().await.unwrap();

        // Create an existing todo with the same ID
        let existing_todo = Todo::new("Existing task".to_string(), None);
        let existing_id = existing_todo.id;
        db.create_todo(&existing_todo).await.unwrap();

        // Create a JSON file with a todo that has the same ID but different title
        let json_data = format!(
            r#"{{
            "version": "1.0",
            "exported_at": "2026-01-31T12:00:00Z",
            "todos": [
                {{
                    "id": "{}",
                    "title": "Should be skipped",
                    "priority": "medium",
                    "is_completed": false,
                    "created_at": "2026-01-31T12:00:00Z",
                    "updated_at": "2026-01-31T12:00:00Z",
                    "sync_status": "pending"
                }}
            ],
            "categories": []
        }}"#,
            existing_id
        );
        std::fs::write(&import_path, json_data).unwrap();

        let result = import_todos(&db, &import_path, ImportMode::Merge).await;
        assert!(result.is_ok());
        let (imported_todos, _) = result.unwrap();
        assert_eq!(imported_todos, 0); // Should skip existing

        let todos = db.list_todos(false).await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].title, "Existing task"); // Title unchanged
    }

    #[tokio::test]
    async fn test_import_replace_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let import_path = temp_dir.path().join("import.json");

        let db_path = temp_dir.path().join("test.db");
        let db = LocalDb::new(&db_path).await.unwrap();
        db.run_migrations().await.unwrap();

        // Create an existing todo with the same ID
        let existing_todo = Todo::new("Existing task".to_string(), None);
        let existing_id = existing_todo.id;
        db.create_todo(&existing_todo).await.unwrap();

        // Create a JSON file with a todo that has the same ID but different title
        let json_data = format!(
            r#"{{
            "version": "1.0",
            "exported_at": "2026-01-31T12:00:00Z",
            "todos": [
                {{
                    "id": "{}",
                    "title": "Updated task",
                    "priority": "high",
                    "is_completed": false,
                    "created_at": "2026-01-31T12:00:00Z",
                    "updated_at": "2026-01-31T12:00:00Z",
                    "sync_status": "pending"
                }}
            ],
            "categories": []
        }}"#,
            existing_id
        );
        std::fs::write(&import_path, json_data).unwrap();

        let result = import_todos(&db, &import_path, ImportMode::Replace).await;
        assert!(result.is_ok());
        let (imported_todos, _) = result.unwrap();
        assert_eq!(imported_todos, 1); // Should replace existing

        let todos = db.list_todos(false).await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].title, "Updated task"); // Title changed
    }
}
