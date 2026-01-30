use anyhow::Result;
use tracing::debug;

pub async fn run() -> Result<()> {
    debug!("sync command");
    println!("[DEBUG] sync");
    Ok(())
}
