//! Smart "now" command for task recommendations.

use std::cmp::Ordering;
use std::fs;

use anyhow::{Context, Result};
use chrono::{Local, Timelike, Utc};
use todoee_core::{Config, LocalDb, Priority, Todo};

pub async fn run() -> Result<()> {
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

    let todos = db.list_todos(false).await?; // Only pending todos

    if todos.is_empty() {
        println!("\x1b[32mNothing to do! Enjoy your free time.\x1b[0m");
        return Ok(());
    }

    // Score each todo
    let mut scored: Vec<(Todo, f64, Vec<&'static str>)> = todos
        .into_iter()
        .map(|t| {
            let (score, reasons) = calculate_score(&t);
            (t, score, reasons)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    println!("\x1b[1m\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}\x1b[0m");
    println!("\x1b[1m\u{2502}           RECOMMENDED RIGHT NOW                 \u{2502}\x1b[0m");
    println!("\x1b[1m\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}\x1b[0m\n");

    for (i, (todo, _score, reasons)) in scored.iter().take(3).enumerate() {
        let marker = if i == 0 {
            "\x1b[1;32m\u{2192}\x1b[0m"
        } else {
            " "
        };
        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };
        let id = &todo.id.to_string()[..8];

        println!("{} {} \x1b[90m{}\x1b[0m {}", marker, pri, id, todo.title);
        if !reasons.is_empty() {
            println!("    \x1b[90m{}\x1b[0m", reasons.join(" \u{2022} "));
        }
        println!();
    }

    if scored.len() > 3 {
        println!("\x1b[90m...and {} more todos\x1b[0m", scored.len() - 3);
    }

    Ok(())
}

fn calculate_score(todo: &Todo) -> (f64, Vec<&'static str>) {
    let mut score = 0.0;
    let mut reasons = Vec::new();

    // Priority weight
    match todo.priority {
        Priority::High => {
            score += 30.0;
            reasons.push("high priority");
        }
        Priority::Medium => {
            score += 15.0;
        }
        Priority::Low => {
            score += 5.0;
        }
    }

    // Due date urgency
    if let Some(due) = todo.due_date {
        let hours_until = due.signed_duration_since(Utc::now()).num_hours();
        if hours_until < 0 {
            score += 50.0;
            reasons.push("overdue!");
        } else if hours_until < 4 {
            score += 40.0;
            reasons.push("due very soon");
        } else if hours_until < 24 {
            score += 25.0;
            reasons.push("due today");
        } else if hours_until < 72 {
            score += 10.0;
            reasons.push("due soon");
        }
    }

    // Time of day heuristics
    let hour = Local::now().hour();
    if (9..12).contains(&hour) {
        // Morning: favor high-priority (peak focus time)
        if todo.priority == Priority::High {
            score += 10.0;
            reasons.push("morning = high focus time");
        }
    } else if (14..17).contains(&hour) {
        // Afternoon: favor medium tasks
        if todo.priority == Priority::Medium {
            score += 5.0;
        }
    } else if hour >= 20 {
        // Evening: favor low priority (wind-down)
        if todo.priority == Priority::Low {
            score += 5.0;
            reasons.push("evening = wind-down time");
        }
    }

    // Age penalty (very old uncompleted tasks might be stuck)
    let age_days = Utc::now().signed_duration_since(todo.created_at).num_days();
    if age_days > 14 {
        score -= 10.0;
        reasons.push("consider breaking down (old task)");
    } else if age_days > 7 {
        score -= 5.0;
    }

    (score, reasons)
}
