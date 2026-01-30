use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler as InputHandler;

use super::app::{App, Mode};

/// Handle key events and update app state
pub async fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    // Clear status message on any key press
    app.status_message = None;

    match app.mode {
        Mode::Normal => handle_normal_mode(app, key).await?,
        Mode::Adding => handle_adding_mode(app, key).await?,
        Mode::Editing => handle_editing_mode(app, key).await?,
        Mode::Searching => handle_searching_mode(app, key).await?,
        Mode::Help => handle_help_mode(app, key),
    }

    Ok(())
}

async fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => app.quit(),

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
            if let Some(title) = app.selected_todo().map(|t| t.title.clone()) {
                app.mode = Mode::Editing;
                app.input = tui_input::Input::new(title);
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

        // Help
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
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
