use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use todoee_core::{Config, LocalDb};

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Csv,
}

#[derive(Serialize)]
struct ExportData {
    version: String,
    exported_at: String,
    todos: Vec<todoee_core::Todo>,
    categories: Vec<todoee_core::Category>,
}

/// Export todos to a file in the specified format.
///
/// By default, exports all todos (including completed). This is the public API
/// for programmatic export, used by tests and intended for future sync functionality.
#[allow(dead_code)]
pub async fn export_todos(
    db: &LocalDb,
    output_path: &Path,
    format: ExportFormat,
) -> Result<usize> {
    export_todos_impl(db, output_path, format, true).await
}

/// Internal implementation that supports filtering by completion status.
async fn export_todos_impl(
    db: &LocalDb,
    output_path: &Path,
    format: ExportFormat,
    include_completed: bool,
) -> Result<usize> {
    // list_todos takes exclude_completed, so we invert include_completed
    let todos = db.list_todos(!include_completed).await?;
    let categories = db.list_categories().await?;

    let data = ExportData {
        version: "1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        todos: todos.clone(),
        categories,
    };

    match format {
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(&data)
                .context("Failed to serialize export data to JSON")?;
            std::fs::write(output_path, json)
                .with_context(|| format!("Failed to write export file: {}", output_path.display()))?;
        }
        ExportFormat::Csv => {
            let mut wtr = csv::Writer::from_path(output_path)
                .with_context(|| format!("Failed to create CSV file: {}", output_path.display()))?;
            for todo in &todos {
                wtr.serialize(todo)
                    .context("Failed to serialize todo to CSV")?;
            }
            wtr.flush()
                .context("Failed to flush CSV writer")?;
        }
    }

    Ok(todos.len())
}

pub async fn run(output: Option<String>, format: String, include_completed: bool) -> Result<()> {
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

    let format = match format.to_lowercase().as_str() {
        "csv" => ExportFormat::Csv,
        _ => ExportFormat::Json,
    };

    let output_path = output.unwrap_or_else(|| {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        match format {
            ExportFormat::Json => format!("todoee_export_{}.json", timestamp),
            ExportFormat::Csv => format!("todoee_export_{}.csv", timestamp),
        }
    });

    let count = export_todos_impl(&db, Path::new(&output_path), format, include_completed).await?;

    println!("\u{2713} Exported {} todos to {}", count, output_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use todoee_core::Todo;

    #[tokio::test]
    async fn test_export_json_creates_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("export.json");

        let db_path = temp_dir.path().join("test.db");
        let db = LocalDb::new(&db_path).await.unwrap();
        db.run_migrations().await.unwrap();

        let todo = Todo::new("Test task".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        let result = export_todos(&db, &output_path, ExportFormat::Json).await;
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object());
        assert!(parsed["todos"].is_array());
        assert_eq!(parsed["todos"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_export_csv_creates_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("export.csv");

        let db_path = temp_dir.path().join("test.db");
        let db = LocalDb::new(&db_path).await.unwrap();
        db.run_migrations().await.unwrap();

        let todo = Todo::new("Test task".to_string(), None);
        db.create_todo(&todo).await.unwrap();

        let result = export_todos(&db, &output_path, ExportFormat::Csv).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // Verify file exists and has content
        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("Test task"));
    }
}
