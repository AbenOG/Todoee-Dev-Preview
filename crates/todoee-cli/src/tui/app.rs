use anyhow::Result;
use todoee_core::{Config, LocalDb, Todo, Category, Priority};
use tui_input::Input;

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal navigation mode
    Normal,
    /// Adding a new task
    Adding,
    /// Editing a task
    Editing,
    /// Searching/filtering
    Searching,
    /// Showing help
    Help,
}

/// Filter state for the task list
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub today_only: bool,
    pub category: Option<String>,
    pub show_completed: bool,
    pub search_query: String,
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
    db: LocalDb,
    /// Configuration
    config: Config,
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
        };

        app.refresh_todos().await?;
        app.refresh_categories().await?;

        Ok(app)
    }

    /// Refresh the todo list from database
    pub async fn refresh_todos(&mut self) -> Result<()> {
        self.todos = if self.filter.today_only {
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
            self.todos.retain(|t| t.title.to_lowercase().contains(&query));
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
        if let Some(todo) = self.todos.get_mut(self.selected) {
            if !todo.is_completed {
                todo.mark_complete();
                self.db.update_todo(todo).await?;
                self.status_message = Some(format!("✓ Completed: {}", todo.title));
                self.refresh_todos().await?;
            } else {
                self.status_message = Some("Already completed".to_string());
            }
        }
        Ok(())
    }

    /// Delete selected todo
    pub async fn delete_selected(&mut self) -> Result<()> {
        if let Some(todo) = self.todos.get(self.selected) {
            let title = todo.title.clone();
            self.db.delete_todo(todo.id).await?;
            self.status_message = Some(format!("✗ Deleted: {}", title));
            self.refresh_todos().await?;
        }
        Ok(())
    }

    /// Add a new todo from input
    pub async fn add_todo_from_input(&mut self) -> Result<()> {
        let description = self.input.value().trim().to_string();
        if description.is_empty() {
            self.status_message = Some("Cannot add empty task".to_string());
            return Ok(());
        }

        let todo = Todo::new(description.clone(), None);
        self.db.create_todo(&todo).await?;
        self.status_message = Some(format!("✓ Added: {}", description));
        self.input.reset();
        self.refresh_todos().await?;

        Ok(())
    }

    /// Toggle today filter
    pub fn toggle_today_filter(&mut self) {
        self.filter.today_only = !self.filter.today_only;
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
}
