//! Productivity insights command.

use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result};
use chrono::{Datelike, Local, TimeZone, Utc, Weekday};
use todoee_core::{Config, LocalDb, OperationType};

pub async fn run(days: Option<i64>) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;
    let db_path = config.local_db_path()?;

    if let Some(parent) = db_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    let days = days.unwrap_or(30);
    let since = Utc::now() - chrono::Duration::days(days);
    let operations = db.list_operations_since(since).await?;
    let todos = db.list_todos(false).await?;

    // Calculate metrics
    let total_completed = operations
        .iter()
        .filter(|op| op.operation_type == OperationType::Complete)
        .count();

    let total_created = operations
        .iter()
        .filter(|op| op.operation_type == OperationType::Create)
        .count();

    // Completion by day of week
    let mut by_weekday: HashMap<Weekday, usize> = HashMap::new();
    for op in operations
        .iter()
        .filter(|op| op.operation_type == OperationType::Complete)
    {
        let local = Local.from_utc_datetime(&op.created_at.naive_utc());
        *by_weekday.entry(local.weekday()).or_insert(0) += 1;
    }

    // Find most productive day
    let best_day = by_weekday
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(day, _)| *day);

    // Completion heatmap (last 4 weeks)
    let mut heatmap: Vec<Vec<usize>> = vec![vec![0; 7]; 4];
    for op in operations
        .iter()
        .filter(|op| op.operation_type == OperationType::Complete)
    {
        let days_ago = Utc::now().signed_duration_since(op.created_at).num_days() as usize;
        if days_ago < 28 {
            let week = days_ago / 7;
            let local = Local.from_utc_datetime(&op.created_at.naive_utc());
            let day = local.weekday().num_days_from_monday() as usize;
            if week < 4 && day < 7 {
                heatmap[week][day] += 1;
            }
        }
    }

    // Print report
    println!("\x1b[1m┌─────────────────────────────────────────────────────────┐\x1b[0m");
    println!(
        "\x1b[1m│            PRODUCTIVITY INSIGHTS ({:>2} days)              │\x1b[0m",
        days
    );
    println!("\x1b[1m└─────────────────────────────────────────────────────────┘\x1b[0m\n");

    println!("  Tasks Created:    {}", total_created);
    println!("  Tasks Completed:  {}", total_completed);

    let completion_rate = if total_created > 0 {
        (total_completed as f64 / total_created as f64 * 100.0) as u32
    } else {
        0
    };
    println!("  Completion Rate:  {}%", completion_rate);

    if let Some(day) = best_day {
        println!("  Most Productive:  {:?}", day);
    }

    println!("\n  \x1b[1mCompletion Heatmap (last 4 weeks):\x1b[0m");
    println!("         Mon Tue Wed Thu Fri Sat Sun");
    for (week_idx, week) in heatmap.iter().enumerate() {
        let week_label = match week_idx {
            0 => "This  ",
            1 => "Last  ",
            2 => "2 ago ",
            3 => "3 ago ",
            _ => "      ",
        };
        print!("  {}", week_label);
        for count in week {
            let block = match count {
                0 => "\x1b[90m░\x1b[0m",
                1..=2 => "\x1b[32m▒\x1b[0m",
                3..=5 => "\x1b[32m▓\x1b[0m",
                _ => "\x1b[1;32m█\x1b[0m",
            };
            print!("{}   ", block);
        }
        println!();
    }

    // Suggestions
    println!("\n  \x1b[1mSuggestions:\x1b[0m");

    let pending = todos.iter().filter(|t| !t.is_completed).count();
    let overdue = todos
        .iter()
        .filter(|t| !t.is_completed && t.due_date.is_some_and(|d| d < Utc::now()))
        .count();

    if overdue > 0 {
        println!(
            "  \x1b[33m•\x1b[0m You have {} overdue todos - consider rescheduling",
            overdue
        );
    }
    if pending > 20 {
        println!(
            "  \x1b[33m•\x1b[0m {} pending todos - consider archiving or breaking down",
            pending
        );
    }
    if let Some(day) = best_day {
        println!("  \x1b[36m•\x1b[0m Schedule important tasks on {:?}s", day);
    }
    if total_completed == 0 && days >= 7 {
        println!(
            "  \x1b[33m•\x1b[0m No completions in {} days - try smaller tasks",
            days
        );
    }

    Ok(())
}
