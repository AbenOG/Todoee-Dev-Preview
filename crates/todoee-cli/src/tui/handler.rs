use anyhow::Result;
use chrono::TimeZone;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use todoee_core::{EntityType, Operation, OperationType, Priority};
use tui_input::backend::crossterm::EventHandler as InputHandler;

use super::app::{
    AddField, AddState, App, EditField, EditState, Mode, SettingsSection, SortBy, SortOrder, View,
};
#[allow(unused_imports)]
use super::app::InsightsData;
use todoee_core::Config;

/// Handle key events and update app state
pub async fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    // Clear status message on any key press
    app.status_message = None;

    match app.mode {
        Mode::Normal => handle_normal_mode(app, key).await?,
        Mode::Adding => handle_adding_mode(app, key).await?,
        Mode::Editing => handle_editing_mode(app, key).await?,
        Mode::EditingFull => handle_editing_full_mode(app, key).await?,
        Mode::Searching => handle_searching_mode(app, key).await?,
        Mode::Help => handle_help_mode(app, key),
        Mode::ViewingDetail => handle_viewing_detail_mode(app, key),
        Mode::AddingCategory => handle_adding_category_mode(app, key).await?,
        Mode::AddingFull => handle_adding_full_mode(app, key).await?,
        Mode::Insights => {
            app.mode = Mode::Normal;
            app.insights_data = None;
        }
        Mode::Focus => {
            match key.code {
                KeyCode::Char(' ') => {
                    if let Some(ref mut state) = app.focus_state {
                        state.toggle_pause();
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    app.cancel_focus();
                }
                KeyCode::Enter => {
                    app.complete_focus();
                }
                _ => {}
            }
        }
    }

    Ok(())
}

async fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    // View switching always available
    match key.code {
        KeyCode::Char('1') => {
            app.current_view = View::Todos;
            return Ok(());
        }
        KeyCode::Char('2') => {
            app.current_view = View::Categories;
            return Ok(());
        }
        KeyCode::Char('3') => {
            app.current_view = View::Settings;
            return Ok(());
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.quit();
            return Ok(());
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
            return Ok(());
        }
        _ => {}
    }

    // View-specific handling
    match app.current_view {
        View::Todos => handle_todos_view(app, key).await?,
        View::Categories => handle_categories_view(app, key).await?,
        View::Settings => handle_settings_view(app, key)?,
    }

    Ok(())
}

async fn handle_todos_view(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_previous(),
        KeyCode::Char('g') => app.selected = 0,
        KeyCode::Char('G') => {
            if !app.todos.is_empty() {
                app.selected = app.todos.len() - 1;
            }
        }

        // Actions
        KeyCode::Char('a') => {
            app.add_state = Some(AddState::new());
            app.mode = Mode::AddingFull;
        }
        KeyCode::Char('A') => {
            // Quick add with AI parsing
            app.mode = Mode::Adding;
            app.input.reset();
            app.pending_priority = None;
        }
        KeyCode::Char('d') | KeyCode::Enter => {
            app.mark_selected_done().await?;
        }
        KeyCode::Char('x') => {
            app.delete_selected().await?;
        }
        KeyCode::Char('e') => {
            if let Some(todo) = app.selected_todo() {
                if todo.is_completed {
                    app.status_message = Some("Cannot edit completed todo (uncomplete first)".to_string());
                } else {
                    app.edit_state = Some(EditState::from_todo(todo, &app.categories));
                    app.mode = Mode::EditingFull;
                }
            }
        }
        KeyCode::Char('v') | KeyCode::Char(' ') => {
            if app.selected_todo().is_some() {
                app.mode = Mode::ViewingDetail;
            }
        }

        // Filtering
        KeyCode::Char('/') => {
            app.mode = Mode::Searching;
            app.input.reset();
        }
        KeyCode::Char('t') => {
            app.toggle_today_filter();
            app.refresh_todos().await?;
            if app.filter.today_only {
                app.status_message = Some("Showing today's tasks".to_string());
            } else {
                app.status_message = Some("Showing all tasks".to_string());
            }
        }
        KeyCode::Char('o') => {
            app.toggle_overdue_filter();
            app.refresh_todos().await?;
            if app.filter.overdue_only {
                app.status_message = Some("Showing overdue tasks".to_string());
            } else {
                app.status_message = Some("Showing all tasks".to_string());
            }
        }
        KeyCode::Tab => {
            app.toggle_show_completed();
            app.refresh_todos().await?;
        }
        KeyCode::Char('c') => {
            // Cycle through categories
            if app.categories.is_empty() {
                app.filter.category = None;
            } else if let Some(ref current) = app.filter.category {
                let idx = app.categories.iter().position(|c| &c.name == current);
                app.filter.category = match idx {
                    Some(i) if i + 1 < app.categories.len() => {
                        Some(app.categories[i + 1].name.clone())
                    }
                    _ => None,
                };
            } else {
                app.filter.category = Some(app.categories[0].name.clone());
            }
            app.filter.today_only = false;
            app.refresh_todos().await?;
        }
        KeyCode::Char('p') => {
            // Cycle priority filter: None -> High -> Medium -> Low -> None
            app.filter.priority = match app.filter.priority {
                None => Some(Priority::High),
                Some(Priority::High) => Some(Priority::Medium),
                Some(Priority::Medium) => Some(Priority::Low),
                Some(Priority::Low) => None,
            };
            app.refresh_todos().await?;
        }
        KeyCode::Char('s') => {
            // Cycle sort: Created -> DueDate -> Priority -> Title -> Created
            app.filter.sort_by = match app.filter.sort_by {
                SortBy::CreatedAt => SortBy::DueDate,
                SortBy::DueDate => SortBy::Priority,
                SortBy::Priority => SortBy::Title,
                SortBy::Title => SortBy::CreatedAt,
            };
            app.refresh_todos().await?;
            let sort_name = match app.filter.sort_by {
                SortBy::CreatedAt => "Created",
                SortBy::DueDate => "Due Date",
                SortBy::Priority => "Priority",
                SortBy::Title => "Title",
            };
            app.status_message = Some(format!("Sorted by: {}", sort_name));
        }
        KeyCode::Char('S') => {
            // Toggle sort order
            app.filter.sort_order = match app.filter.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
            app.refresh_todos().await?;
            let order = match app.filter.sort_order {
                SortOrder::Ascending => "Ascending",
                SortOrder::Descending => "Descending",
            };
            app.status_message = Some(format!("Sort order: {}", order));
        }

        // Undo
        KeyCode::Char('u') => {
            app.undo().await?;
        }

        // Redo (Ctrl+r)
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.redo().await?;
        }

        // Stash
        KeyCode::Char('z') => {
            app.stash_selected().await?;
        }
        KeyCode::Char('Z') => {
            app.stash_pop().await?;
        }

        // Insights
        KeyCode::Char('i') => {
            app.set_loading("Computing insights...");
            let data = app.compute_insights().await?;
            app.clear_loading();
            app.insights_data = Some(data);
            app.mode = Mode::Insights;
        }

        // Now recommendation
        KeyCode::Char('n') => {
            if let Some(idx) = app.get_now_recommendation() {
                app.selected = idx;
                if let Some(todo) = app.selected_todo() {
                    app.status_message = Some(format!("Recommended: {}", todo.title));
                }
            } else {
                app.status_message = Some("No tasks to recommend".to_string());
            }
        }

        // Focus mode
        KeyCode::Char('f') => {
            if app.selected_todo().is_some() {
                app.start_focus(25); // 25 minute pomodoro
            } else {
                app.status_message = Some("No task selected".to_string());
            }
        }
        KeyCode::Char('F') => {
            if app.selected_todo().is_some() {
                app.start_focus(5); // 5 minute quick focus
            } else {
                app.status_message = Some("No task selected".to_string());
            }
        }

        _ => {}
    }
    Ok(())
}

async fn handle_categories_view(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.category_selected < app.categories.len().saturating_sub(1) {
                app.category_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.category_selected > 0 {
                app.category_selected -= 1;
            }
        }
        KeyCode::Char('a') => {
            app.mode = Mode::AddingCategory;
            app.input.reset();
        }
        KeyCode::Char('x') => {
            app.delete_selected_category().await?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_settings_view(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.settings_section = match app.settings_section {
                SettingsSection::Ai => SettingsSection::Display,
                SettingsSection::Display => SettingsSection::Notifications,
                SettingsSection::Notifications => SettingsSection::Database,
                SettingsSection::Database => SettingsSection::Database,
            };
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.settings_section = match app.settings_section {
                SettingsSection::Ai => SettingsSection::Ai,
                SettingsSection::Display => SettingsSection::Ai,
                SettingsSection::Notifications => SettingsSection::Display,
                SettingsSection::Database => SettingsSection::Notifications,
            };
        }
        KeyCode::Char('r') => {
            // Reload config
            if let Ok(config) = Config::load() {
                app.config = config;
                app.status_message = Some("✓ Configuration reloaded".to_string());
            } else {
                app.status_message = Some("✗ Failed to reload configuration".to_string());
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_adding_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
            app.pending_priority = None;
        }
        KeyCode::Enter => {
            // Use AI only if Shift held AND AI is configured
            let use_ai = app.has_ai() && key.modifiers.contains(KeyModifiers::SHIFT);
            app.add_todo_with_ai(use_ai).await?;
            app.mode = Mode::Normal;
            app.pending_priority = None;
        }
        // Priority shortcuts: Ctrl+1/2/3 or Alt+1/2/3
        KeyCode::Char('1')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT) =>
        {
            app.pending_priority = Some(Priority::Low);
        }
        KeyCode::Char('2')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT) =>
        {
            app.pending_priority = Some(Priority::Medium);
        }
        KeyCode::Char('3')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT) =>
        {
            app.pending_priority = Some(Priority::High);
        }
        // Tab cycles priority
        KeyCode::Tab => {
            app.pending_priority = match app.pending_priority {
                None => Some(Priority::Low),
                Some(Priority::Low) => Some(Priority::Medium),
                Some(Priority::Medium) => Some(Priority::High),
                Some(Priority::High) => None,
            };
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }

    Ok(())
}

async fn handle_editing_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
        }
        KeyCode::Enter => {
            let new_title = app.input.value().trim().to_string();
            if !new_title.is_empty()
                && let Some(todo) = app.todos.get_mut(app.selected)
            {
                todo.title = new_title.clone();
                todo.updated_at = chrono::Utc::now();
                todo.sync_status = todoee_core::SyncStatus::Pending;
                app.db.update_todo(todo).await?;
                app.status_message = Some(format!("✓ Updated: {}", new_title));
            }
            app.mode = Mode::Normal;
            app.input.reset();
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }

    Ok(())
}

async fn handle_searching_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
            app.clear_search();
            app.refresh_todos().await?;
        }
        KeyCode::Enter => {
            app.apply_search();
            app.mode = Mode::Normal;
            app.refresh_todos().await?;
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input.reset();
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }

    Ok(())
}

fn handle_help_mode(app: &mut App, _key: KeyEvent) {
    app.mode = Mode::Normal;
}

fn handle_viewing_detail_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc
        | KeyCode::Char('q')
        | KeyCode::Char('v')
        | KeyCode::Char(' ')
        | KeyCode::Enter => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

async fn handle_editing_full_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    let Some(ref mut state) = app.edit_state else {
        app.mode = Mode::Normal;
        return Ok(());
    };

    match key.code {
        KeyCode::Esc => {
            app.edit_state = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            state.active_field = match state.active_field {
                EditField::Title => EditField::Description,
                EditField::Description => EditField::Priority,
                EditField::Priority => EditField::DueDate,
                EditField::DueDate => EditField::Category,
                EditField::Category => EditField::Title,
            };
        }
        KeyCode::BackTab => {
            state.active_field = match state.active_field {
                EditField::Title => EditField::Category,
                EditField::Description => EditField::Title,
                EditField::Priority => EditField::Description,
                EditField::DueDate => EditField::Priority,
                EditField::Category => EditField::DueDate,
            };
        }
        KeyCode::Enter => {
            // Save changes
            let todo_id = state.todo_id;
            let category_name = state.category_name.clone();

            // Find todo with defensive handling
            let Some(todo) = app.todos.iter_mut().find(|t| t.id == todo_id) else {
                app.edit_state = None;
                app.mode = Mode::Normal;
                app.status_message = Some("Todo no longer exists".to_string());
                return Ok(());
            };

            // Capture previous state
            let previous_state = serde_json::to_value(&*todo).ok();

            // Apply all field updates
            todo.title = state.title.clone();
            todo.description = if state.description.is_empty() {
                None
            } else {
                Some(state.description.clone())
            };
            todo.priority = state.priority;
            todo.due_date = state.due_date.as_ref().and_then(|s| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(12, 0, 0))
                    .map(|dt| chrono::Utc.from_utc_datetime(&dt))
            });
            // Set category_id from name
            todo.category_id = category_name.as_ref().and_then(|name| {
                app.categories
                    .iter()
                    .find(|c| &c.name == name)
                    .map(|c| c.id)
            });
            todo.updated_at = chrono::Utc::now();
            todo.sync_status = todoee_core::SyncStatus::Pending;

            // Capture new state
            let new_state = serde_json::to_value(&*todo).ok();
            let title = todo.title.clone();

            app.db.update_todo(todo).await?;

            // Record operation for undo/redo support
            let op = Operation::new(
                OperationType::Update,
                EntityType::Todo,
                todo_id,
                previous_state,
                new_state,
            );
            app.db.record_operation(&op).await?;

            app.status_message = Some(format!("✓ Updated: {}", title));
            app.edit_state = None;
            app.mode = Mode::Normal;
            app.refresh_todos().await?;
        }
        KeyCode::Char(c) => {
            match state.active_field {
                EditField::Title => state.title.push(c),
                EditField::Description => state.description.push(c),
                EditField::Priority => {
                    state.priority = match c {
                        '1' => Priority::Low,
                        '2' => Priority::Medium,
                        '3' => Priority::High,
                        _ => state.priority,
                    };
                }
                EditField::DueDate => {
                    let due = state.due_date.get_or_insert_with(String::new);
                    if c.is_ascii_digit() || c == '-' {
                        due.push(c);
                    }
                }
                EditField::Category => {
                    // Cycle through categories with any key
                    let cat_names: Vec<_> = app.categories.iter().map(|c| c.name.clone()).collect();
                    if !cat_names.is_empty() {
                        state.category_name = match &state.category_name {
                            None => Some(cat_names[0].clone()),
                            Some(current) => {
                                let idx = cat_names.iter().position(|n| n == current).unwrap_or(0);
                                if idx + 1 < cat_names.len() {
                                    Some(cat_names[idx + 1].clone())
                                } else {
                                    None
                                }
                            }
                        };
                    }
                }
            }
        }
        KeyCode::Backspace => {
            match state.active_field {
                EditField::Title => {
                    state.title.pop();
                }
                EditField::Description => {
                    state.description.pop();
                }
                EditField::Priority => {} // Can't backspace priority
                EditField::DueDate => {
                    if let Some(ref mut due) = state.due_date {
                        due.pop();
                        if due.is_empty() {
                            state.due_date = None;
                        }
                    }
                }
                EditField::Category => {
                    // Backspace clears category
                    state.category_name = None;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

async fn handle_adding_category_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
        }
        KeyCode::Enter => {
            let name = app.input.value().trim().to_string();
            app.add_category(name, None).await?;
            app.input.reset();
            app.mode = Mode::Normal;
        }
        _ => {
            app.input.handle_event(&crossterm::event::Event::Key(key));
        }
    }
    Ok(())
}

async fn handle_adding_full_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    let Some(ref mut state) = app.add_state else {
        app.mode = Mode::Normal;
        return Ok(());
    };

    match key.code {
        KeyCode::Esc => {
            app.add_state = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Tab => {
            state.active_field = match state.active_field {
                AddField::Title => AddField::Description,
                AddField::Description => AddField::Priority,
                AddField::Priority => AddField::DueDate,
                AddField::DueDate => AddField::Reminder,
                AddField::Reminder => AddField::Category,
                AddField::Category => AddField::Title,
            };
        }
        KeyCode::BackTab => {
            state.active_field = match state.active_field {
                AddField::Title => AddField::Category,
                AddField::Description => AddField::Title,
                AddField::Priority => AddField::Description,
                AddField::DueDate => AddField::Priority,
                AddField::Reminder => AddField::DueDate,
                AddField::Category => AddField::Reminder,
            };
        }
        KeyCode::Enter => {
            if state.is_valid() {
                app.create_todo_from_add_state().await?;
                app.add_state = None;
                app.mode = Mode::Normal;
            } else {
                app.status_message = Some("Title is required".to_string());
            }
        }
        KeyCode::Char(c) => {
            match state.active_field {
                AddField::Title => state.title.push(c),
                AddField::Description => state.description.push(c),
                AddField::Priority => {
                    state.priority = match c {
                        '1' => Priority::Low,
                        '2' => Priority::Medium,
                        '3' => Priority::High,
                        _ => state.priority,
                    };
                }
                AddField::DueDate => {
                    let due = state.due_date.get_or_insert_with(String::new);
                    if c.is_ascii_alphanumeric() || c == '-' || c == '+' {
                        due.push(c);
                    }
                }
                AddField::Reminder => {
                    let rem = state.reminder.get_or_insert_with(String::new);
                    if c.is_ascii_digit() || c == '-' || c == ':' || c == ' ' {
                        rem.push(c);
                    }
                }
                AddField::Category => {
                    // Cycle through categories with any key
                    let cat_names: Vec<_> = app.categories.iter().map(|c| c.name.clone()).collect();
                    if !cat_names.is_empty() {
                        state.category_name = match &state.category_name {
                            None => Some(cat_names[0].clone()),
                            Some(current) => {
                                let idx = cat_names.iter().position(|n| n == current).unwrap_or(0);
                                if idx + 1 < cat_names.len() {
                                    Some(cat_names[idx + 1].clone())
                                } else {
                                    None
                                }
                            }
                        };
                    }
                }
            }
        }
        KeyCode::Backspace => {
            match state.active_field {
                AddField::Title => {
                    state.title.pop();
                }
                AddField::Description => {
                    state.description.pop();
                }
                AddField::Priority => {} // Can't backspace priority
                AddField::DueDate => {
                    if let Some(ref mut due) = state.due_date {
                        due.pop();
                        if due.is_empty() {
                            state.due_date = None;
                        }
                    }
                }
                AddField::Reminder => {
                    if let Some(ref mut rem) = state.reminder {
                        rem.pop();
                        if rem.is_empty() {
                            state.reminder = None;
                        }
                    }
                }
                AddField::Category => {
                    state.category_name = None;
                }
            }
        }
        _ => {}
    }

    Ok(())
}
