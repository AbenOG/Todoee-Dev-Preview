use anyhow::Result;
use tracing::debug;

pub async fn run(id: String) -> Result<()> {
    debug!("done command: id={}", id);
    println!("[DEBUG] done: id={}", id);
    Ok(())
}
