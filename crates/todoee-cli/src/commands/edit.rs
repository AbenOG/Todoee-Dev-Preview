use anyhow::Result;
use tracing::debug;

pub async fn run(
    id: String,
    title: Option<String>,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
    debug!(
        "edit command: id={}, title={:?}, category={:?}, priority={:?}",
        id, title, category, priority
    );
    println!(
        "[DEBUG] edit: id={}, title={:?}, category={:?}, priority={:?}",
        id, title, category, priority
    );
    Ok(())
}
