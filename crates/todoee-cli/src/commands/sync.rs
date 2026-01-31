use anyhow::{Context, Result};
use todoee_core::{config::Config, sync::SyncService};

pub async fn run(force: bool) -> Result<()> {
    // Note: `force` parameter reserved for future use (e.g., skip "recently synced" check)
    let _ = force;

    let config = Config::load().context("Failed to load configuration")?;

    let service = SyncService::new(&config)
        .await
        .context("Failed to initialize sync service")?;

    if !service.is_configured() {
        println!("\u{2139}  Cloud sync is not configured.\n");
        println!("To enable sync with Neon Postgres:");
        println!("  1. Create a free database at https://neon.tech");
        println!("  2. Copy your connection string");
        println!("  3. Set the environment variable:");
        println!("     export NEON_DATABASE_URL=\"postgres://...\"");
        println!("\nThen run `todoee sync` again.");
        return Ok(());
    }

    println!("\u{1F504} Syncing with cloud...");

    let result = service.sync().await.context("Sync failed")?;

    println!("\u{2713} Sync complete!");
    println!("  Uploaded:   {} todos", result.uploaded);
    println!("  Downloaded: {} todos", result.downloaded);
    if result.conflicts > 0 {
        println!(
            "  Conflicts:  {} (resolved with last-write-wins)",
            result.conflicts
        );
    }

    Ok(())
}
