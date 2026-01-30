use anyhow::Result;
use tracing::debug;

pub async fn run(id: String) -> Result<()> {
    debug!("delete command: id={}", id);
    println!("[DEBUG] delete: id={}", id);
    Ok(())
}
