//! Sync service for bi-directional synchronization between local and remote databases.
//!
//! This module provides `SyncService` which orchestrates sync between the local SQLite
//! database and the remote PostgreSQL database (Neon).

use crate::{
    config::Config,
    db::{LocalDb, RemoteDb},
    models::SyncStatus,
    Result as TodoeeResult, TodoeeError,
};
use chrono::{DateTime, Utc};

/// Result of a sync operation, containing counts of items processed.
#[derive(Debug, Default)]
pub struct SyncResult {
    /// Number of todos uploaded to remote.
    pub uploaded: usize,
    /// Number of todos downloaded from remote.
    pub downloaded: usize,
    /// Number of conflicts encountered (local wins when local is newer).
    pub conflicts: usize,
}

/// Service for bi-directional sync between local and remote databases.
pub struct SyncService {
    local: LocalDb,
    remote: Option<RemoteDb>,
}

impl SyncService {
    /// Create a new SyncService from the given configuration.
    ///
    /// This initializes the local database and optionally connects to the remote
    /// database if `NEON_DATABASE_URL` is configured.
    pub async fn new(config: &Config) -> TodoeeResult<Self> {
        let local = LocalDb::new(&config.local_db_path().map_err(|e| {
            TodoeeError::Config(format!("Failed to get local db path: {}", e))
        })?).await.map_err(|e| {
            TodoeeError::Config(format!("Failed to open local database: {}", e))
        })?;
        local.run_migrations().await.map_err(|e| {
            TodoeeError::Config(format!("Failed to run migrations: {}", e))
        })?;

        let remote = if let Some(url) = config.get_database_url() {
            Some(RemoteDb::new(&url).await?)
        } else {
            None
        };

        Ok(Self { local, remote })
    }

    /// Create a SyncService with a pre-existing LocalDb (useful for testing).
    pub fn with_local(local: LocalDb) -> Self {
        Self {
            local,
            remote: None,
        }
    }

    /// Check if cloud sync is configured.
    pub fn is_configured(&self) -> bool {
        self.remote.is_some()
    }

    /// Get a reference to the local database.
    pub fn local(&self) -> &LocalDb {
        &self.local
    }

    /// Perform a bi-directional sync with the remote database.
    ///
    /// This:
    /// 1. Uploads all local changes (pending sync items) to remote
    /// 2. Downloads all remote changes since last sync
    /// 3. Resolves conflicts using last-write-wins strategy
    ///
    /// # Errors
    ///
    /// Returns an error if cloud sync is not configured or if any database
    /// operations fail.
    pub async fn sync(&self) -> TodoeeResult<SyncResult> {
        let remote = self.remote.as_ref().ok_or_else(|| {
            TodoeeError::Config(
                "Cloud sync not configured. Set NEON_DATABASE_URL environment variable.".to_string(),
            )
        })?;

        let mut result = SyncResult::default();

        // 1. Upload local changes (pending sync items)
        let pending = self.local.list_pending_sync().await.map_err(|e| {
            TodoeeError::Database(sqlx::Error::Protocol(format!(
                "Failed to list pending sync: {}",
                e
            )))
        })?;

        for todo in pending {
            remote.upsert_todo(&todo).await?;
            self.local.mark_synced(todo.id).await.map_err(|e| {
                TodoeeError::Database(sqlx::Error::Protocol(format!(
                    "Failed to mark synced: {}",
                    e
                )))
            })?;
            result.uploaded += 1;
        }

        // 2. Download remote changes
        let last_sync = self.get_last_sync_time().await;
        let remote_changes = remote.get_todos_since(last_sync).await?;

        for remote_todo in remote_changes {
            match self.local.get_todo(remote_todo.id).await {
                Ok(Some(local_todo)) => {
                    // Conflict resolution: last-write-wins
                    if remote_todo.updated_at > local_todo.updated_at {
                        let mut updated = remote_todo.clone();
                        updated.sync_status = SyncStatus::Synced;
                        self.local.update_todo(&updated).await.map_err(|e| {
                            TodoeeError::Database(sqlx::Error::Protocol(format!(
                                "Failed to update todo: {}",
                                e
                            )))
                        })?;
                        result.downloaded += 1;
                    } else if local_todo.sync_status == SyncStatus::Pending {
                        // Local is newer and has pending changes - count as conflict
                        result.conflicts += 1;
                    }
                }
                Ok(None) => {
                    // New remote todo - download it
                    let mut new_todo = remote_todo.clone();
                    new_todo.sync_status = SyncStatus::Synced;
                    self.local.create_todo(&new_todo).await.map_err(|e| {
                        TodoeeError::Database(sqlx::Error::Protocol(format!(
                            "Failed to create todo: {}",
                            e
                        )))
                    })?;
                    result.downloaded += 1;
                }
                Err(e) => {
                    // Log error but continue syncing other todos
                    eprintln!("Warning: Failed to get local todo {}: {}", remote_todo.id, e);
                }
            }
        }

        Ok(result)
    }

    /// Get the timestamp of the last sync.
    ///
    /// For now, returns epoch (0) to sync all changes.
    /// In production, this would be stored in a metadata table.
    async fn get_last_sync_time(&self) -> DateTime<Utc> {
        // Return epoch to sync all changes. In production, store in metadata table.
        DateTime::from_timestamp(0, 0).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Todo;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sync_uploads_pending_todos() {
        // Test the local side only (no remote connection needed)
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let local_db = LocalDb::new(&db_path).await.unwrap();
        local_db.run_migrations().await.unwrap();

        // Create a pending todo
        let todo = Todo::new("Sync me".to_string(), None);
        local_db.create_todo(&todo).await.unwrap();

        // Verify it's pending
        let pending = local_db.list_pending_sync().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Sync me");
    }

    #[tokio::test]
    async fn test_sync_service_without_remote() {
        // Test that SyncService can be created without a remote
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let local_db = LocalDb::new(&db_path).await.unwrap();
        local_db.run_migrations().await.unwrap();

        let service = SyncService::with_local(local_db);
        assert!(!service.is_configured());

        // Sync should fail with a clear error
        let result = service.sync().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TodoeeError::Config(_)));
    }

    #[tokio::test]
    async fn test_mark_synced_changes_status() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let local_db = LocalDb::new(&db_path).await.unwrap();
        local_db.run_migrations().await.unwrap();

        // Create a pending todo
        let todo = Todo::new("Will be synced".to_string(), None);
        local_db.create_todo(&todo).await.unwrap();

        // Verify it's pending
        let pending = local_db.list_pending_sync().await.unwrap();
        assert_eq!(pending.len(), 1);

        // Mark as synced
        local_db.mark_synced(todo.id).await.unwrap();

        // Verify it's no longer pending
        let pending = local_db.list_pending_sync().await.unwrap();
        assert_eq!(pending.len(), 0);

        // Verify the todo still exists and is marked as synced
        let retrieved = local_db.get_todo(todo.id).await.unwrap().unwrap();
        assert_eq!(retrieved.sync_status, SyncStatus::Synced);
    }
}
