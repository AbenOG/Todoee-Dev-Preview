use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low = 1,
    #[default]
    Medium = 2,
    High = 3,
}

impl Priority {
    /// Convert an i32 value to a Priority.
    /// Returns Medium for any value that doesn't match Low (1) or High (3).
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => Priority::Low,
            3 => Priority::High,
            _ => Priority::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    #[default]
    Pending,
    Synced,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub is_ai_generated: bool,
    pub sync_status: SyncStatus,
}

impl Category {
    pub fn new(user_id: Uuid, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            name,
            color: None,
            is_ai_generated: false,
            sync_status: SyncStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub reminder_at: Option<DateTime<Utc>>,
    pub priority: Priority,
    pub is_completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub ai_metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sync_status: SyncStatus,
}

impl Todo {
    pub fn new(title: String, user_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            category_id: None,
            title,
            description: None,
            due_date: None,
            reminder_at: None,
            priority: Priority::default(),
            is_completed: false,
            completed_at: None,
            ai_metadata: None,
            created_at: now,
            updated_at: now,
            sync_status: SyncStatus::Pending,
        }
    }

    pub fn mark_complete(&mut self) {
        self.is_completed = true;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.sync_status = SyncStatus::Pending;
    }

    pub fn mark_incomplete(&mut self) {
        self.is_completed = false;
        self.completed_at = None;
        self.updated_at = Utc::now();
        self.sync_status = SyncStatus::Pending;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub reminder_at: Option<DateTime<Utc>>,
    pub recurrence_rule: Option<String>,
    pub created_at: DateTime<Utc>,
    pub sync_status: SyncStatus,
}

/// Type of operation performed on an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    Create,
    Update,
    Delete,
    Complete,
    Uncomplete,
    Stash,
    Unstash,
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OperationType::Create => "create",
            OperationType::Update => "update",
            OperationType::Delete => "delete",
            OperationType::Complete => "complete",
            OperationType::Uncomplete => "uncomplete",
            OperationType::Stash => "stash",
            OperationType::Unstash => "unstash",
        };
        write!(f, "{}", s)
    }
}

/// Type of entity that an operation was performed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Todo,
    Category,
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EntityType::Todo => "todo",
            EntityType::Category => "category",
        };
        write!(f, "{}", s)
    }
}

/// Represents a tracked operation for undo/redo and analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: Uuid,
    pub operation_type: OperationType,
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub previous_state: Option<serde_json::Value>,
    pub new_state: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub undone: bool,
}

impl Operation {
    pub fn new(
        operation_type: OperationType,
        entity_type: EntityType,
        entity_id: Uuid,
        previous_state: Option<serde_json::Value>,
        new_state: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            operation_type,
            entity_type,
            entity_id,
            previous_state,
            new_state,
            created_at: Utc::now(),
            undone: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_new_creates_valid_todo() {
        let todo = Todo::new("Buy milk".to_string(), None);

        assert_eq!(todo.title, "Buy milk");
        assert!(!todo.is_completed);
        assert!(todo.category_id.is_none());
        assert_eq!(todo.priority, Priority::Medium);
    }

    #[test]
    fn test_todo_mark_complete_sets_completed_at() {
        let mut todo = Todo::new("Test task".to_string(), None);
        assert!(!todo.is_completed);
        assert!(todo.completed_at.is_none());

        todo.mark_complete();

        assert!(todo.is_completed);
        assert!(todo.completed_at.is_some());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::High > Priority::Medium);
        assert!(Priority::Medium > Priority::Low);
    }

    #[test]
    fn test_category_new() {
        let user_id = Uuid::new_v4();
        let cat = Category::new(user_id, "Work".to_string());

        assert_eq!(cat.name, "Work");
        assert_eq!(cat.user_id, user_id);
        assert!(!cat.is_ai_generated);
    }
}
