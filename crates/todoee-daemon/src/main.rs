use std::collections::HashSet;
use std::time::Duration;

use anyhow::Result;
use notify_rust::Notification;
use todoee_core::{config::Config, db::LocalDb};
use tokio::time::interval;
use uuid::Uuid;

const CHECK_INTERVAL_SECS: u64 = 60; // Check every minute

#[tokio::main]
async fn main() -> Result<()> {
    println!("todoee-daemon starting...");

    let config = Config::load()?;

    if !config.notifications.enabled {
        println!("Notifications are disabled in config. Exiting.");
        return Ok(());
    }

    let db_path = config.local_db_path()?;
    let db = LocalDb::new(&db_path).await?;
    db.run_migrations().await?;

    println!(
        "Daemon running. Checking for reminders every {} seconds.",
        CHECK_INTERVAL_SECS
    );

    let mut ticker = interval(Duration::from_secs(CHECK_INTERVAL_SECS));
    let mut sent_reminders: HashSet<Uuid> = HashSet::new();

    loop {
        ticker.tick().await;

        if let Err(e) = check_and_notify(&db, &config, &mut sent_reminders).await {
            eprintln!("Error checking reminders: {}", e);
        }
    }
}

async fn check_and_notify(
    db: &LocalDb,
    config: &Config,
    sent_reminders: &mut HashSet<Uuid>,
) -> Result<()> {
    let window = chrono::Duration::minutes(config.notifications.advance_minutes as i64);

    // Use the optimized query instead of filtering in Rust
    let todos = db.list_todos_with_reminders_due(window).await?;

    for todo in &todos {
        if sent_reminders.contains(&todo.id) {
            continue;
        }

        send_notification(&todo.title, config)?;
        sent_reminders.insert(todo.id);
    }

    // Cleanup sent_reminders for todos no longer in the result
    sent_reminders.retain(|id| todos.iter().any(|t| &t.id == id));

    Ok(())
}

fn send_notification(title: &str, config: &Config) -> Result<()> {
    let mut notification = Notification::new();

    notification
        .summary("Todoee Reminder")
        .body(title)
        .appname("todoee")
        .timeout(notify_rust::Timeout::Milliseconds(10000));

    if config.notifications.sound {
        notification.sound_name("message-new-instant");
    }

    notification.show()?;

    println!("Sent reminder: {}", title);
    Ok(())
}
