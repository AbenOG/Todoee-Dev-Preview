use std::fs;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::ExecutableCommand;
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{self, ClearType};
use todoee_core::{Config, EntityType, LocalDb, Operation, OperationType, Priority, Todo};

/// Result of a focus session.
enum FocusResult {
    /// User marked the task as done.
    Done,
    /// User skipped to a different task.
    Skip,
    /// User quit the focus session.
    Quit,
    /// Timer completed naturally.
    Completed,
}

pub async fn run(id: Option<String>, duration_mins: u32) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let db_path = config.local_db_path()?;

    if let Some(parent) = db_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    let todo = select_todo(&db, id.as_deref()).await?;
    let duration = Duration::from_secs(u64::from(duration_mins) * 60);
    let start = Instant::now();

    // Set up terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(terminal::Clear(ClearType::All))?;
    stdout.execute(cursor::Hide)?;

    // Main loop
    let result = run_timer(&mut stdout, &todo.title, duration, start);

    // Restore terminal
    stdout.execute(cursor::Show)?;
    terminal::disable_raw_mode()?;

    handle_result(result?, &db, &todo, start).await
}

async fn select_todo(db: &LocalDb, id: Option<&str>) -> Result<Todo> {
    let todos = db.list_todos(false).await?;

    if let Some(id) = id {
        let id_lower = id.to_lowercase();
        let matches: Vec<_> = todos
            .into_iter()
            .filter(|t| t.id.to_string().to_lowercase().starts_with(&id_lower))
            .collect();

        match matches.len() {
            0 => anyhow::bail!("Todo not found"),
            1 => Ok(matches.into_iter().next().unwrap()),
            _ => {
                eprintln!("Multiple todos match '{}'. Please be more specific:", id);
                for todo in &matches {
                    let short_id = &todo.id.to_string()[..8];
                    eprintln!("  {} [{}]", todo.title, short_id);
                }
                anyhow::bail!("Ambiguous ID - provide more characters")
            }
        }
    } else {
        // Pick the highest priority todo, or first if tied
        todos
            .into_iter()
            .max_by_key(|t| priority_value(t.priority))
            .ok_or_else(|| anyhow::anyhow!("No todos to focus on"))
    }
}

fn priority_value(priority: Priority) -> u8 {
    match priority {
        Priority::High => 3,
        Priority::Medium => 2,
        Priority::Low => 1,
    }
}

async fn handle_result(
    result: FocusResult,
    db: &LocalDb,
    todo: &Todo,
    start: Instant,
) -> Result<()> {
    match result {
        FocusResult::Done => {
            let mut updated = todo.clone();
            let prev = serde_json::to_value(&updated)?;
            updated.mark_complete();
            db.update_todo(&updated).await?;

            let op = Operation::new(
                OperationType::Complete,
                EntityType::Todo,
                todo.id,
                Some(prev),
                None,
            );
            db.record_operation(&op).await?;

            println!("\n\x1b[32m\u{2713} Marked as done: {}\x1b[0m", todo.title);
        }
        FocusResult::Skip | FocusResult::Quit => {
            println!("\nFocus session ended.");
        }
        FocusResult::Completed => {
            println!(
                "\n\x1b[32mTimer completed!\x1b[0m Focus session on: {}",
                todo.title
            );
        }
    }

    let elapsed = start.elapsed();
    let elapsed_mins = elapsed.as_secs() / 60;
    let elapsed_secs = elapsed.as_secs() % 60;
    println!("Focused for {}:{:02}", elapsed_mins, elapsed_secs);

    Ok(())
}

fn run_timer(
    stdout: &mut io::Stdout,
    title: &str,
    duration: Duration,
    start: Instant,
) -> Result<FocusResult> {
    loop {
        let elapsed = start.elapsed();
        if elapsed >= duration {
            return Ok(FocusResult::Completed);
        }

        draw_ui(stdout, title, duration, elapsed)?;

        // Check for input (poll for 100ms)
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(KeyEvent { code, .. }) = event::read()?
        {
            match code {
                KeyCode::Char('d') => return Ok(FocusResult::Done),
                KeyCode::Char('s') => return Ok(FocusResult::Skip),
                KeyCode::Char('q') | KeyCode::Esc => return Ok(FocusResult::Quit),
                _ => {}
            }
        }
    }
}

fn draw_ui(
    stdout: &mut io::Stdout,
    title: &str,
    duration: Duration,
    elapsed: Duration,
) -> Result<()> {
    let remaining = duration - elapsed;
    let mins = remaining.as_secs() / 60;
    let secs = remaining.as_secs() % 60;

    let progress = elapsed.as_secs_f64() / duration.as_secs_f64();
    let bar_width = 30;
    let filled = (progress * bar_width as f64) as usize;
    let bar = format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(bar_width - filled)
    );

    stdout.execute(cursor::MoveTo(0, 0))?;

    println!(
        "\x1b[1;36m\u{256d}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256e}\x1b[0m"
    );
    println!(
        "\x1b[1;36m\u{2502}\x1b[0m               \x1b[1mFOCUS MODE\x1b[0m                        \x1b[1;36m\u{2502}\x1b[0m"
    );
    println!(
        "\x1b[1;36m\u{251c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2524}\x1b[0m"
    );
    println!(
        "\x1b[1;36m\u{2502}\x1b[0m                                                  \x1b[1;36m\u{2502}\x1b[0m"
    );
    println!(
        "\x1b[1;36m\u{2502}\x1b[0m  \x1b[1m{:<46}\x1b[0m  \x1b[1;36m\u{2502}\x1b[0m",
        truncate(title, 46)
    );
    println!(
        "\x1b[1;36m\u{2502}\x1b[0m                                                  \x1b[1;36m\u{2502}\x1b[0m"
    );
    println!(
        "\x1b[1;36m\u{2502}\x1b[0m  {} {:02}:{:02}                        \x1b[1;36m\u{2502}\x1b[0m",
        bar, mins, secs
    );
    println!(
        "\x1b[1;36m\u{2502}\x1b[0m                                                  \x1b[1;36m\u{2502}\x1b[0m"
    );
    println!(
        "\x1b[1;36m\u{2502}\x1b[0m  \x1b[90m[d] done  [s] skip  [q] quit\x1b[0m                  \x1b[1;36m\u{2502}\x1b[0m"
    );
    println!(
        "\x1b[1;36m\u{256e}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f}\x1b[0m"
    );

    stdout.flush()?;
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
