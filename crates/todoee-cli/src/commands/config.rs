use anyhow::Result;
use tracing::debug;

pub async fn run(init: bool) -> Result<()> {
    debug!("config command: init={}", init);
    println!("[DEBUG] config: init={}", init);
    Ok(())
}
