//! Fuzzy search command for finding todos by text.

use std::fs;

use anyhow::{Context, Result};
use todoee_core::{Config, LocalDb, Priority, Todo};

/// Run fuzzy search on todos.
pub async fn run(query: &str) -> Result<()> {
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

    // Get all todos for fuzzy matching (include completed)
    let all_todos = db.list_todos(false).await?;

    // Fuzzy match
    let query_lower = query.to_lowercase();
    let mut matches: Vec<(&Todo, i32)> = all_todos
        .iter()
        .filter_map(|todo| {
            let score = fuzzy_score(&todo.title.to_lowercase(), &query_lower);
            if score > 0 {
                Some((todo, score))
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending
    matches.sort_by(|a, b| b.1.cmp(&a.1));

    if matches.is_empty() {
        println!("No matches for \"{}\"", query);
        return Ok(());
    }

    println!("Found {} matches for \"{}\":\n", matches.len().min(20), query);

    for (todo, _score) in matches.iter().take(20) {
        let check = if todo.is_completed {
            "\x1b[32m[x]\x1b[0m"
        } else {
            "[ ]"
        };

        let pri = match todo.priority {
            Priority::High => "\x1b[31m!!!\x1b[0m",
            Priority::Medium => "\x1b[33m!! \x1b[0m",
            Priority::Low => "\x1b[90m!  \x1b[0m",
        };

        let id = &todo.id.to_string()[..8];

        // Highlight matching parts
        let highlighted = highlight_match(&todo.title, query);
        println!("{} {} \x1b[90m{}\x1b[0m {}", check, pri, id, highlighted);
    }

    Ok(())
}

/// Simple fuzzy scoring - higher is better match.
fn fuzzy_score(haystack: &str, needle: &str) -> i32 {
    // Exact substring match is best
    if haystack.contains(needle) {
        return 100;
    }

    // Check if all chars appear in order
    let mut score = 0;
    let mut haystack_chars = haystack.chars().peekable();
    let mut prev_matched = false;

    for needle_char in needle.chars() {
        let mut found = false;
        while let Some(&hay_char) = haystack_chars.peek() {
            haystack_chars.next();
            if hay_char == needle_char {
                score += 10;
                // Bonus for consecutive matches
                if prev_matched {
                    score += 5;
                }
                prev_matched = true;
                found = true;
                break;
            }
            prev_matched = false;
        }
        if !found {
            return 0; // Character not found, no match
        }
    }

    // Bonus for matching at word start
    if let Some(first_char) = needle.chars().next() {
        for word in haystack.split_whitespace() {
            if word.starts_with(first_char) {
                score += 15;
                break;
            }
        }
    }

    score
}

/// Highlight the matching portion of text in yellow bold.
fn highlight_match(text: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    if let Some(pos) = text_lower.find(&query_lower) {
        let before = &text[..pos];
        let matched = &text[pos..pos + query.len()];
        let after = &text[pos + query.len()..];
        format!("{}\x1b[1;33m{}\x1b[0m{}", before, matched, after)
    } else {
        text.to_string()
    }
}
