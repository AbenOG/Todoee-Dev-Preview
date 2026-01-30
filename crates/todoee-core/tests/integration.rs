//! Integration tests for the todoee-core crate.
//!
//! These tests verify the full todo workflow including creation,
//! listing, updating, and sync status tracking.

use todoee_core::{Category, Config, LocalDb, Priority, Todo};
use uuid::Uuid;

/// Helper function to set up an in-memory database with migrations.
async fn setup_db() -> LocalDb {
    let db = LocalDb::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    db
}

#[tokio::test]
async fn test_full_todo_workflow() {
    // Setup
    let db = setup_db().await;
    let user_id = Uuid::new_v4();

    // Create a category
    let mut category = Category::new(user_id, "Work".to_string());
    category.is_ai_generated = true;
    db.create_category(&category).await.unwrap();

    // Create a todo in that category
    let mut todo = Todo::new("Review pull request".to_string(), Some(user_id));
    todo.category_id = Some(category.id);
    todo.priority = Priority::High;
    db.create_todo(&todo).await.unwrap();

    // List todos (exclude_completed = true means only incomplete)
    let todos = db.list_todos(true).await.unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].title, "Review pull request");

    // List by category
    let work_todos = db.list_todos_by_category(category.id).await.unwrap();
    assert_eq!(work_todos.len(), 1);

    // Mark complete
    let mut todo = todos[0].clone();
    todo.mark_complete();
    db.update_todo(&todo).await.unwrap();

    // Verify not in incomplete list (exclude_completed = true)
    let incomplete = db.list_todos(true).await.unwrap();
    assert_eq!(incomplete.len(), 0);

    // But in all list (exclude_completed = false)
    let all = db.list_todos(false).await.unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].is_completed);
}

#[tokio::test]
async fn test_sync_status_tracking() {
    let db = setup_db().await;

    // New todos start as pending
    let todo = Todo::new("Sync test".to_string(), None);
    db.create_todo(&todo).await.unwrap();

    let pending = db.list_pending_sync().await.unwrap();
    assert_eq!(pending.len(), 1);

    // Mark as synced
    db.mark_synced(todo.id).await.unwrap();

    let pending = db.list_pending_sync().await.unwrap();
    assert_eq!(pending.len(), 0);
}

#[test]
fn test_config_defaults() {
    let config = Config::default();

    assert_eq!(config.ai.provider, "openrouter");
    assert!(config.ai.model.is_none()); // User must set this
    assert!(config.notifications.enabled);
}

#[tokio::test]
async fn test_category_workflow() {
    let db = setup_db().await;
    let user_id = Uuid::new_v4();

    // Create multiple categories
    let work_cat = Category::new(user_id, "Work".to_string());
    let personal_cat = Category::new(user_id, "Personal".to_string());

    db.create_category(&work_cat).await.unwrap();
    db.create_category(&personal_cat).await.unwrap();

    // List categories
    let categories = db.list_categories().await.unwrap();
    assert_eq!(categories.len(), 2);

    // Get category by name
    let found = db.get_category_by_name("Work").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, work_cat.id);

    // Create todos in different categories
    let mut todo1 = Todo::new("Work task".to_string(), Some(user_id));
    todo1.category_id = Some(work_cat.id);

    let mut todo2 = Todo::new("Personal task".to_string(), Some(user_id));
    todo2.category_id = Some(personal_cat.id);

    db.create_todo(&todo1).await.unwrap();
    db.create_todo(&todo2).await.unwrap();

    // List by category
    let work_todos = db.list_todos_by_category(work_cat.id).await.unwrap();
    assert_eq!(work_todos.len(), 1);
    assert_eq!(work_todos[0].title, "Work task");

    let personal_todos = db.list_todos_by_category(personal_cat.id).await.unwrap();
    assert_eq!(personal_todos.len(), 1);
    assert_eq!(personal_todos[0].title, "Personal task");
}

#[tokio::test]
async fn test_todo_priority_and_description() {
    let db = setup_db().await;

    // Create todo with all fields set
    let mut todo = Todo::new("Important task".to_string(), None);
    todo.priority = Priority::High;
    todo.description = Some("This is a detailed description".to_string());

    db.create_todo(&todo).await.unwrap();

    // Retrieve and verify
    let retrieved = db.get_todo(todo.id).await.unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::High);
    assert_eq!(
        retrieved.description,
        Some("This is a detailed description".to_string())
    );

    // Update priority
    let mut updated = retrieved;
    updated.priority = Priority::Low;
    db.update_todo(&updated).await.unwrap();

    let final_todo = db.get_todo(todo.id).await.unwrap().unwrap();
    assert_eq!(final_todo.priority, Priority::Low);
}

#[tokio::test]
async fn test_delete_todo() {
    let db = setup_db().await;

    let todo = Todo::new("To be deleted".to_string(), None);
    db.create_todo(&todo).await.unwrap();

    // Verify exists
    let retrieved = db.get_todo(todo.id).await.unwrap();
    assert!(retrieved.is_some());

    // Delete
    db.delete_todo(todo.id).await.unwrap();

    // Verify deleted
    let deleted = db.get_todo(todo.id).await.unwrap();
    assert!(deleted.is_none());
}

#[tokio::test]
async fn test_multiple_todos_sync_status() {
    let db = setup_db().await;

    // Create multiple todos
    let todo1 = Todo::new("Task 1".to_string(), None);
    let todo2 = Todo::new("Task 2".to_string(), None);
    let todo3 = Todo::new("Task 3".to_string(), None);

    db.create_todo(&todo1).await.unwrap();
    db.create_todo(&todo2).await.unwrap();
    db.create_todo(&todo3).await.unwrap();

    // All should be pending initially
    let pending = db.list_pending_sync().await.unwrap();
    assert_eq!(pending.len(), 3);

    // Mark some as synced
    db.mark_synced(todo1.id).await.unwrap();
    db.mark_synced(todo3.id).await.unwrap();

    // Only todo2 should be pending
    let pending = db.list_pending_sync().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, todo2.id);
}
