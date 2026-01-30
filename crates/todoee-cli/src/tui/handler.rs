use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler as InputHandler;
use chrono::TimeZone;
use todoee_core::Priority;

use super::app::{App, EditField, EditState, Mode, View};

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
    }

    Ok(())
}

async fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    // View switching always available
    match key.code {
        KeyCode::Char('1') => { app.current_view = View::Todos; return Ok(()); }
        KeyCode::Char('2') => { app.current_view = View::Categories; return Ok(()); }
        KeyCode::Char('3') => { app.current_view = View::Settings; return Ok(()); }
        KeyCode::Char('q') | KeyCode::Esc => { app.quit(); return Ok(()); }
        KeyCode::Char('?') => { app.mode = Mode::Help; return Ok(()); }
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
            app.mode = Mode::Adding;
            app.input.reset();
        }
        KeyCode::Char('d') | KeyCode::Enter => {
            app.mark_selected_done().await?;
        }
        KeyCode::Char('x') => {
            app.delete_selected().await?;
        }
        KeyCode::Char('e') => {
            if let Some(todo) = app.selected_todo() {
                app.edit_state = Some(EditState::from_todo(todo));
                app.mode = Mode::EditingFull;
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

fn handle_settings_view(_app: &mut App, _key: KeyEvent) -> Result<()> {
    // Settings navigation - to be implemented in later tasks
    Ok(())
}

async fn handle_adding_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input.reset();
        }
        KeyCode::Enter => {
            // Use AI if available and Shift not held
            let use_ai = app.has_ai() && !key.modifiers.contains(KeyModifiers::SHIFT);
            app.add_todo_with_ai(use_ai).await?;
            app.mode = Mode::Normal;
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
            if !new_title.is_empty() {
                if let Some(todo) = app.todos.get_mut(app.selected) {
                    todo.title = new_title.clone();
                    todo.updated_at = chrono::Utc::now();
                    todo.sync_status = todoee_core::SyncStatus::Pending;
                    app.db.update_todo(todo).await?;
                    app.status_message = Some(format!("✓ Updated: {}", new_title));
                }
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
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('v') | KeyCode::Char(' ') | KeyCode::Enter => {
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
                EditField::DueDate => EditField::Title,
            };
        }
        KeyCode::BackTab => {
            state.active_field = match state.active_field {
                EditField::Title => EditField::DueDate,
                EditField::Description => EditField::Title,
                EditField::Priority => EditField::Description,
                EditField::DueDate => EditField::Priority,
            };
        }
        KeyCode::Enter => {
            // Save changes
            let todo_id = state.todo_id;
            if let Some(todo) = app.todos.iter_mut().find(|t| t.id == todo_id) {
                todo.title = state.title.clone();
                todo.description = if state.description.is_empty() { None } else { Some(state.description.clone()) };
                todo.priority = state.priority;
                todo.due_date = state.due_date.as_ref().and_then(|s| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .and_then(|d| d.and_hms_opt(12, 0, 0))
                        .map(|dt| chrono::Utc.from_utc_datetime(&dt))
                });
                todo.updated_at = chrono::Utc::now();
                todo.sync_status = todoee_core::SyncStatus::Pending;
                app.db.update_todo(todo).await?;
                app.status_message = Some(format!("✓ Updated: {}", todo.title));
            }
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
            }
        }
        KeyCode::Backspace => {
            match state.active_field {
                EditField::Title => { state.title.pop(); }
                EditField::Description => { state.description.pop(); }
                EditField::Priority => {} // Can't backspace priority
                EditField::DueDate => {
                    if let Some(ref mut due) = state.due_date {
                        due.pop();
                        if due.is_empty() {
                            state.due_date = None;
                        }
                    }
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
