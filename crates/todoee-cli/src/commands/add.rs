use anyhow::Result;
use tracing::debug;

pub async fn run(
    description: Vec<String>,
    no_ai: bool,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
    debug!(
        "add command: description={:?}, no_ai={}, category={:?}, priority={:?}",
        description, no_ai, category, priority
    );
    println!(
        "[DEBUG] add: description={:?}, no_ai={}, category={:?}, priority={:?}",
        description, no_ai, category, priority
    );
    Ok(())
}
