use anyhow::Result;
use std::env;

pub async fn run() -> Result<()> {
    // Check if NEON_DATABASE_URL is set
    let neon_url = env::var("NEON_DATABASE_URL");

    match neon_url {
        Ok(_) => {
            // Database URL is configured
            println!("Cloud sync not yet implemented.");
            println!();
            println!("Your todos are being stored locally and will sync");
            println!("automatically when this feature is available.");
            println!();
            println!("For now, your data is safe in the local database.");
        }
        Err(_) => {
            // Database URL is not set - show setup instructions
            println!("Cloud sync is not configured.");
            println!();
            println!("To enable cloud sync with Neon PostgreSQL:");
            println!();
            println!("1. Create a free Neon database at https://neon.tech");
            println!();
            println!("2. Copy your connection string from the Neon dashboard");
            println!();
            println!("3. Set the environment variable:");
            println!(
                "   export NEON_DATABASE_URL=\"postgres://user:pass@host/db?sslmode=require\""
            );
            println!();
            println!("4. Add it to your shell profile (~/.bashrc or ~/.zshrc) for persistence");
            println!();
            println!("Until then, your todos are stored locally and work offline.");
        }
    }

    Ok(())
}
