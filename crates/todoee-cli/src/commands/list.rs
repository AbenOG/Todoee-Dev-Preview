use anyhow::Result;
use tracing::debug;

pub async fn run(today: bool, category: Option<String>, all: bool) -> Result<()> {
    debug!(
        "list command: today={}, category={:?}, all={}",
        today, category, all
    );
    println!(
        "[DEBUG] list: today={}, category={:?}, all={}",
        today, category, all
    );
    Ok(())
}
