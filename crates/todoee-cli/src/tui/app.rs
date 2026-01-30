use anyhow::Result;
use chrono::{Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use todoee_core::{Category, Config, EntityType, LocalDb, Operation, OperationType, Priority, Todo};
use tui_input::Input;

/// Main view/tab of the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Todos,
    Categories,
    Settings,
}

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Mode {
    /// Normal navigation mode
    Normal,
    /// Adding a new task
    Adding,
    /// Editing a task (title-only quick edit)
    Editing,
    /// Full multi-field edit
    EditingFull,
    /// Searching/filtering
    Searching,
    /// Showing help
    Help,
    /// Viewing todo details
    ViewingDetail,
    /// Adding a new category
    AddingCategory,
    /// Adding a new task with full fields
    AddingFull,
}

/// Field being edited in full edit mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    Title,
    Description,
    Priority,
    DueDate,
    Category,
}

/// Field being edited in add mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum AddField {
    #[default]
    Title,
    Description,
    Priority,
    DueDate,
    Reminder,
    Category,
}

/// Settings panel section
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    Ai,
    Display,
    Notifications,
    Database,
}

/// Sort field for the task list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    #[default]
    CreatedAt,
    DueDate,
    Priority,
    Title,
}

/// Sort order for the task list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

/// State for editing a todo with multiple fields
#[derive(Debug, Clone)]
pub struct EditState {
    pub todo_id: uuid::Uuid,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub due_date: Option<String>, // Store as string for editing
    pub category_name: Option<String>,
    pub active_field: EditField,
}

impl EditState {
    pub fn from_todo(todo: &Todo, categories: &[Category]) -> Self {
        let category_name = todo.category_id.and_then(|id| {
            categories
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.name.clone())
        });
        Self {
            todo_id: todo.id,
            title: todo.title.clone(),
            description: todo.description.clone().unwrap_or_default(),
            priority: todo.priority,
            due_date: todo.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
            category_name,
            active_field: EditField::Title,
        }
    }
}

/// State for adding a new todo with multiple fields
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct AddState {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub due_date: Option<String>, // YYYY-MM-DD format
    pub reminder: Option<String>, // YYYY-MM-DD HH:MM format
    pub category_name: Option<String>,
    pub active_field: AddField,
}

#[allow(dead_code)]
impl AddState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_valid(&self) -> bool {
        !self.title.trim().is_empty()
    }
}

/// Parse a date string that can be:
/// - Absolute: "2026-01-30"
/// - Relative: "today", "tomorrow", "+3d", "+1w"
fn parse_due_date(input: &str) -> Option<chrono::DateTime<Utc>> {
    let input = input.trim().to_lowercase();
    let today = Local::now().date_naive();

    let date = match input.as_str() {
        "today" => today,
        "tomorrow" => today + Duration::days(1),
        s if s.starts_with('+') && s.ends_with('d') => {
            let days: i64 = s[1..s.len() - 1].parse().ok()?;
            today + Duration::days(days)
        }
        s if s.starts_with('+') && s.ends_with('w') => {
            let weeks: i64 = s[1..s.len() - 1].parse().ok()?;
            today + Duration::weeks(weeks)
        }
        _ => NaiveDate::parse_from_str(&input, "%Y-%m-%d").ok()?,
    };

    date.and_hms_opt(12, 0, 0)
        .map(|dt| Utc.from_utc_datetime(&dt))
}

/// Parse a reminder datetime string: "2026-01-30 14:00"
fn parse_reminder(input: &str) -> Option<chrono::DateTime<Utc>> {
    let input = input.trim();
    NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M")
        .ok()
        .map(|dt| Utc.from_utc_datetime(&dt))
}

/// Filter state for the task list
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub today_only: bool,
    pub overdue_only: bool,
    pub category: Option<String>,
    pub show_completed: bool,
    pub search_query: String,
    pub priority: Option<Priority>,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
}

/// Application state
pub struct App {
    /// Is the app running?
    pub running: bool,
    /// Current mode
    pub mode: Mode,
    /// List of todos
    pub todos: Vec<Todo>,
    /// List of categories
    pub categories: Vec<Category>,
    /// Currently selected index
    pub selected: usize,
    /// Input field for adding/editing/searching
    pub input: Input,
    /// Current filter
    pub filter: Filter,
    /// Status message to display
    pub status_message: Option<String>,
    /// Database connection
    pub db: LocalDb,
    /// Configuration
    pub config: Config,
    /// Edit state for full todo editing
    pub edit_state: Option<EditState>,
    /// Add state for creating new todos
    pub add_state: Option<AddState>,
    /// Current view/tab
    pub current_view: View,
    /// Selected category index
    pub category_selected: usize,
    /// Current settings section
    pub settings_section: SettingsSection,
    /// Whether an async operation is in progress
    pub is_loading: bool,
    /// Loading message to display
    pub loading_message: Option<String>,
    /// Priority to apply when adding a task
    pub pending_priority: Option<Priority>,
}

impl App {
    /// Create a new application instance
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let db_path = config.local_db_path()?;
        let db = LocalDb::new(&db_path).await?;

        let mut app = Self {
            running: true,
            mode: Mode::Normal,
            todos: Vec::new(),
            categories: Vec::new(),
            selected: 0,
            input: Input::default(),
            filter: Filter::default(),
            status_message: None,
            db,
            config,
            edit_state: None,
            add_state: None,
            current_view: View::default(),
            category_selected: 0,
            settings_section: SettingsSection::default(),
            is_loading: false,
            loading_message: None,
            pending_priority: None,
        };

        app.refresh_todos().await?;
        app.refresh_categories().await?;

        Ok(app)
    }

    /// Refresh the todo list from database
    pub async fn refresh_todos(&mut self) -> Result<()> {
        self.todos = if self.filter.overdue_only {
            self.db.list_todos_overdue().await?
        } else if self.filter.today_only {
            self.db.list_todos_due_today().await?
        } else if let Some(ref cat_name) = self.filter.category {
            if let Some(cat) = self.db.get_category_by_name(cat_name).await? {
                self.db.list_todos_by_category(cat.id).await?
            } else {
                Vec::new()
            }
        } else {
            self.db.list_todos(!self.filter.show_completed).await?
        };

        // Apply search filter
        if !self.filter.search_query.is_empty() {
            let query = self.filter.search_query.to_lowercase();
            self.todos
                .retain(|t| t.title.to_lowercase().contains(&query));
        }

        // Apply priority filter
        if let Some(priority) = self.filter.priority {
            self.todos.retain(|t| t.priority == priority);
        }

        // Sort todos
        match (self.filter.sort_by, self.filter.sort_order) {
            (SortBy::CreatedAt, SortOrder::Ascending) => {
                self.todos.sort_by(|a, b| a.created_at.cmp(&b.created_at))
            }
            (SortBy::CreatedAt, SortOrder::Descending) => {
                self.todos.sort_by(|a, b| b.created_at.cmp(&a.created_at))
            }
            (SortBy::DueDate, SortOrder::Ascending) => {
                self.todos.sort_by(|a, b| a.due_date.cmp(&b.due_date))
            }
            (SortBy::DueDate, SortOrder::Descending) => {
                self.todos.sort_by(|a, b| b.due_date.cmp(&a.due_date))
            }
            (SortBy::Priority, SortOrder::Ascending) => {
                self.todos.sort_by(|a, b| a.priority.cmp(&b.priority))
            }
            (SortBy::Priority, SortOrder::Descending) => {
                self.todos.sort_by(|a, b| b.priority.cmp(&a.priority))
            }
            (SortBy::Title, SortOrder::Ascending) => self
                .todos
                .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
            (SortBy::Title, SortOrder::Descending) => self
                .todos
                .sort_by(|a, b| b.title.to_lowercase().cmp(&a.title.to_lowercase())),
        }

        // Ensure selected index is valid
        if self.selected >= self.todos.len() && !self.todos.is_empty() {
            self.selected = self.todos.len() - 1;
        }

        Ok(())
    }

    /// Refresh categories from database
    pub async fn refresh_categories(&mut self) -> Result<()> {
        self.categories = self.db.list_categories().await?;
        Ok(())
    }

    /// Get the currently selected todo
    pub fn selected_todo(&self) -> Option<&Todo> {
        self.todos.get(self.selected)
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected < self.todos.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Mark selected todo as done
    pub async fn mark_selected_done(&mut self) -> Result<()> {
        // Check if selected todo exists and is not completed
        let should_complete = self
            .todos
            .get(self.selected)
            .is_some_and(|t| !t.is_completed);

        if should_complete {
            self.set_loading("Completing task...");
            let todo = self.todos.get_mut(self.selected).unwrap();
            todo.mark_complete();
            let title = todo.title.clone();
            self.db.update_todo(todo).await?;
            self.clear_loading();
            self.status_message = Some(format!("✓ Completed: {}", title));
            self.refresh_todos().await?;
        } else if self.todos.get(self.selected).is_some() {
            self.status_message = Some("Already completed".to_string());
        }
        Ok(())
    }

    /// Delete selected todo
    pub async fn delete_selected(&mut self) -> Result<()> {
        // Extract necessary data before borrowing self mutably
        let todo_info = self
            .todos
            .get(self.selected)
            .map(|t| (t.id, t.title.clone()));

        if let Some((id, title)) = todo_info {
            self.set_loading("Deleting task...");
            self.db.delete_todo(id).await?;
            self.clear_loading();
            self.status_message = Some(format!("✗ Deleted: {}", title));
            self.refresh_todos().await?;
        }
        Ok(())
    }

    /// Toggle today filter
    pub fn toggle_today_filter(&mut self) {
        self.filter.today_only = !self.filter.today_only;
        self.filter.overdue_only = false;
        self.filter.category = None;
    }

    /// Toggle overdue filter
    pub fn toggle_overdue_filter(&mut self) {
        self.filter.overdue_only = !self.filter.overdue_only;
        self.filter.today_only = false;
        self.filter.category = None;
    }

    /// Toggle show completed
    pub fn toggle_show_completed(&mut self) {
        self.filter.show_completed = !self.filter.show_completed;
    }

    /// Set search query from input
    pub fn apply_search(&mut self) {
        self.filter.search_query = self.input.value().to_string();
        self.input.reset();
    }

    /// Clear search
    pub fn clear_search(&mut self) {
        self.filter.search_query.clear();
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Add a new todo with optional AI parsing
    pub async fn add_todo_with_ai(&mut self, use_ai: bool) -> Result<()> {
        let description = self.input.value().trim().to_string();
        if description.is_empty() {
            self.status_message = Some("Cannot add empty task".to_string());
            return Ok(());
        }

        // Show loading indicator for AI parsing
        if use_ai && self.config.ai.model.is_some() {
            self.set_loading("Parsing with AI...");
        }

        let todo = if use_ai && self.config.ai.model.is_some() {
            match self.parse_with_ai(&description).await {
                Ok(t) => {
                    self.clear_loading();
                    t
                }
                Err(e) => {
                    self.clear_loading();
                    self.status_message = Some(format!("AI failed: {}, using plain text", e));
                    Todo::new(description.clone(), None)
                }
            }
        } else {
            Todo::new(description.clone(), None)
        };

        // Apply pending priority if set
        let mut todo = todo;
        if let Some(priority) = self.pending_priority.take() {
            todo.priority = priority;
        }

        let title = todo.title.clone();
        self.db.create_todo(&todo).await?;
        self.status_message = Some(format!("✓ Added: {}", title));
        self.input.reset();
        self.refresh_todos().await?;

        Ok(())
    }

    async fn parse_with_ai(&self, description: &str) -> Result<Todo> {
        use todoee_core::AiClient;

        let client = AiClient::new(&self.config)?;
        let parsed = client.parse_task(description).await?;

        let mut todo = Todo::new(parsed.title, None);
        todo.description = parsed.description;
        todo.due_date = parsed.due_date;
        todo.reminder_at = parsed.reminder_at;

        if let Some(p) = parsed.priority {
            todo.priority = match p {
                1 => Priority::Low,
                3 => Priority::High,
                _ => Priority::Medium,
            };
        }

        todo.ai_metadata = Some(serde_json::json!({
            "original_input": description,
            "parsed_category": parsed.category,
        }));

        Ok(todo)
    }

    /// Check if AI is configured
    pub fn has_ai(&self) -> bool {
        self.config.ai.model.is_some()
    }

    /// Set loading state with a message
    pub fn set_loading(&mut self, message: &str) {
        self.is_loading = true;
        self.loading_message = Some(message.to_string());
    }

    /// Clear loading state
    pub fn clear_loading(&mut self) {
        self.is_loading = false;
        self.loading_message = None;
    }

    /// Add a new category
    pub async fn add_category(&mut self, name: String, color: Option<String>) -> Result<()> {
        if name.is_empty() {
            self.status_message = Some("Category name cannot be empty".to_string());
            return Ok(());
        }

        self.set_loading("Creating category...");

        // Check if category exists
        if self.db.get_category_by_name(&name).await?.is_some() {
            self.clear_loading();
            self.status_message = Some(format!("Category '{}' already exists", name));
            return Ok(());
        }

        let mut category = Category::new(uuid::Uuid::nil(), name.clone());
        category.color = color;
        self.db.create_category(&category).await?;
        self.clear_loading();
        self.status_message = Some(format!("Created category: {}", name));
        self.refresh_categories().await?;
        Ok(())
    }

    /// Delete selected category
    pub async fn delete_selected_category(&mut self) -> Result<()> {
        if let Some(cat) = self.categories.get(self.category_selected) {
            let name = cat.name.clone();
            let id = cat.id;

            self.set_loading("Deleting category...");
            self.db.delete_category(id).await?;
            self.clear_loading();

            self.status_message = Some(format!("Deleted category: {}", name));
            self.refresh_categories().await?;
            if self.category_selected >= self.categories.len() && !self.categories.is_empty() {
                self.category_selected = self.categories.len() - 1;
            }
        }
        Ok(())
    }

    /// Undo the last operation
    pub async fn undo(&mut self) -> Result<()> {
        let Some(op) = self.db.get_last_undoable_operation().await? else {
            self.status_message = Some("Nothing to undo".to_string());
            return Ok(());
        };

        // Only handle Todo operations for now
        if op.entity_type != EntityType::Todo {
            self.status_message = Some("Cannot undo category operations yet".to_string());
            return Ok(());
        }

        self.apply_undo(&op).await?;
        self.db.mark_operation_undone(op.id).await?;

        let op_name = match op.operation_type {
            OperationType::Create => "create",
            OperationType::Update => "update",
            OperationType::Delete => "delete",
            OperationType::Complete => "complete",
            OperationType::Uncomplete => "uncomplete",
            OperationType::Stash => "stash",
            OperationType::Unstash => "unstash",
        };
        self.status_message = Some(format!("↶ Undone: {}", op_name));
        self.refresh_todos().await?;

        Ok(())
    }

    /// Apply the reverse of an operation
    async fn apply_undo(&mut self, op: &Operation) -> Result<()> {
        match op.operation_type {
            OperationType::Create => {
                // Undo create by deleting the entity
                self.db.delete_todo(op.entity_id).await?;
            }
            OperationType::Delete => {
                // Undo delete by restoring from previous_state
                if let Some(ref state) = op.previous_state {
                    let todo: Todo = serde_json::from_value(state.clone())?;
                    self.db.create_todo(&todo).await?;
                }
            }
            OperationType::Update => {
                // Undo update by restoring previous_state
                if let Some(ref state) = op.previous_state {
                    let todo: Todo = serde_json::from_value(state.clone())?;
                    self.db.update_todo(&todo).await?;
                }
            }
            OperationType::Complete => {
                // Undo complete by marking as incomplete
                if let Some(mut todo) = self.db.get_todo(op.entity_id).await? {
                    todo.is_completed = false;
                    todo.completed_at = None;
                    self.db.update_todo(&todo).await?;
                }
            }
            OperationType::Uncomplete => {
                // Undo uncomplete by marking as complete
                if let Some(mut todo) = self.db.get_todo(op.entity_id).await? {
                    todo.mark_complete();
                    self.db.update_todo(&todo).await?;
                }
            }
            OperationType::Stash | OperationType::Unstash => {
                // Stash operations not yet implemented in Todo model
            }
        }
        Ok(())
    }

    /// Redo the last undone operation
    pub async fn redo(&mut self) -> Result<()> {
        let Some(op) = self.db.get_last_redoable_operation().await? else {
            self.status_message = Some("Nothing to redo".to_string());
            return Ok(());
        };

        // Only handle Todo operations for now
        if op.entity_type != EntityType::Todo {
            self.status_message = Some("Cannot redo category operations yet".to_string());
            return Ok(());
        }

        self.apply_redo(&op).await?;
        self.db.mark_operation_redone(op.id).await?;

        let op_name = match op.operation_type {
            OperationType::Create => "create",
            OperationType::Update => "update",
            OperationType::Delete => "delete",
            OperationType::Complete => "complete",
            OperationType::Uncomplete => "uncomplete",
            OperationType::Stash => "stash",
            OperationType::Unstash => "unstash",
        };
        self.status_message = Some(format!("↷ Redone: {}", op_name));
        self.refresh_todos().await?;

        Ok(())
    }

    /// Re-apply an operation
    async fn apply_redo(&mut self, op: &Operation) -> Result<()> {
        match op.operation_type {
            OperationType::Create => {
                // Redo create by creating from new_state
                if let Some(ref state) = op.new_state {
                    let todo: Todo = serde_json::from_value(state.clone())?;
                    self.db.create_todo(&todo).await?;
                }
            }
            OperationType::Delete => {
                // Redo delete by deleting the entity
                self.db.delete_todo(op.entity_id).await?;
            }
            OperationType::Update => {
                // Redo update by applying new_state
                if let Some(ref state) = op.new_state {
                    let todo: Todo = serde_json::from_value(state.clone())?;
                    self.db.update_todo(&todo).await?;
                }
            }
            OperationType::Complete => {
                // Redo complete by marking as complete
                if let Some(mut todo) = self.db.get_todo(op.entity_id).await? {
                    todo.mark_complete();
                    self.db.update_todo(&todo).await?;
                }
            }
            OperationType::Uncomplete => {
                // Redo uncomplete by marking as incomplete
                if let Some(mut todo) = self.db.get_todo(op.entity_id).await? {
                    todo.is_completed = false;
                    todo.completed_at = None;
                    self.db.update_todo(&todo).await?;
                }
            }
            OperationType::Stash | OperationType::Unstash => {
                // Stash operations not yet implemented in Todo model
            }
        }
        Ok(())
    }

    /// Create a todo from the current add state
    pub async fn create_todo_from_add_state(&mut self) -> Result<()> {
        let Some(ref state) = self.add_state else {
            return Ok(());
        };

        // Extract all needed data from state before mutating self
        let title = state.title.trim().to_string();
        let description = if state.description.is_empty() {
            None
        } else {
            Some(state.description.clone())
        };
        let priority = state.priority;
        let due_date = state.due_date.as_ref().and_then(|s| parse_due_date(s));
        let reminder_at = state.reminder.as_ref().and_then(|s| parse_reminder(s));
        let category_name = state.category_name.clone();

        self.set_loading("Creating task...");

        let mut todo = Todo::new(title.clone(), None);
        todo.description = description;
        todo.priority = priority;
        todo.due_date = due_date;
        todo.reminder_at = reminder_at;

        // Category
        if let Some(ref cat_name) = category_name
            && let Some(cat) = self.categories.iter().find(|c| &c.name == cat_name)
        {
            todo.category_id = Some(cat.id);
        }

        self.db.create_todo(&todo).await?;
        self.clear_loading();
        self.status_message = Some(format!("✓ Added: {}", title));
        self.refresh_todos().await?;

        Ok(())
    }

    /// Stash the selected todo
    pub async fn stash_selected(&mut self) -> Result<()> {
        if let Some(todo) = self.selected_todo().cloned() {
            let title = todo.title.clone();
            self.db.stash_todo(todo.id, None).await?;
            self.status_message = Some(format!("Stashed: {}", title));
            self.refresh_todos().await?;
        }
        Ok(())
    }

    /// Pop the most recent stashed todo
    pub async fn stash_pop(&mut self) -> Result<()> {
        if let Some(todo) = self.db.stash_pop().await? {
            self.status_message = Some(format!("Restored: {}", todo.title));
            self.refresh_todos().await?;
        } else {
            self.status_message = Some("Stash is empty".to_string());
        }
        Ok(())
    }
}
