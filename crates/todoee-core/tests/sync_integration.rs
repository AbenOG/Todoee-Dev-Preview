//! Integration test for sync and reminder workflow.
//!
//! This test verifies that the sync status tracking and reminder
//! query functionality work correctly together.

use tempfile::TempDir;
use todoee_core::{LocalDb, SyncStatus, Todo};
use uuid::Uuid;

#[tokio::test]
async fn test_full_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let db = LocalDb::new(&temp_dir.path().join("test.db"))
        .await
        .unwrap();
    db.run_migrations().await.unwrap();

    // 1. Create a todo with a reminder
    let mut todo = Todo::new("Integration test task".to_string(), None);
    todo.reminder_at = Some(chrono::Utc::now() + chrono::Duration::hours(1));
    db.create_todo(&todo).await.unwrap();

    // 2. Verify it's pending sync
    let pending = db.list_pending_sync().await.unwrap();
    assert_eq!(pending.len(), 1);

    // 3. Mark as synced
    db.mark_synced(todo.id).await.unwrap();

    // 4. Verify no longer pending
    let pending = db.list_pending_sync().await.unwrap();
    assert_eq!(pending.len(), 0);

    // 5. Check reminder query works
    let reminders = db
        .list_todos_with_reminders_due(chrono::Duration::hours(2))
        .await
        .unwrap();
    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0].title, "Integration test task");
}

#[tokio::test]
async fn test_delete_does_not_redownload() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let local_db = LocalDb::new(&db_path).await.unwrap();
    local_db.run_migrations().await.unwrap();

    // Create and "sync" a todo (simulate it came from cloud)
    let user_id = Uuid::new_v4();
    let mut todo = Todo::new("Cloud todo".to_string(), Some(user_id));
    todo.sync_status = SyncStatus::Synced;
    local_db.create_todo(&todo).await.unwrap();

    // Delete it locally
    local_db.delete_todo(todo.id).await.unwrap();

    // Verify it's tracked as deleted
    assert!(local_db.is_locally_deleted(todo.id).await.unwrap());

    // Verify it won't be in pending sync (it's deleted)
    let pending = local_db.list_pending_sync().await.unwrap();
    assert!(!pending.iter().any(|t| t.id == todo.id));
}
