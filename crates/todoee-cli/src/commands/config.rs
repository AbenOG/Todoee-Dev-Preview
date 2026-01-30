use anyhow::{Context, Result};
use std::env;
use todoee_core::Config;

pub async fn run(init: bool) -> Result<()> {
    if init {
        run_init().await
    } else {
        show_config().await
    }
}

/// Initialize configuration with default settings
async fn run_init() -> Result<()> {
    let config_path = Config::config_path()?;

    // Check if config already exists
    if config_path.exists() {
        println!("Configuration file already exists at:");
        println!("  {}", config_path.display());
        println!();
        println!("To recreate, delete the file first and run 'todoee config --init' again.");
        return Ok(());
    }

    // Create default config and save
    let config = Config::default();
    config.save().context("Failed to save configuration")?;

    println!("\u{2713} Configuration file created at:");
    println!("  {}", config_path.display());
    println!();
    println!("Next steps:");
    println!();
    println!("1. Set up AI parsing (optional):");
    println!("   export OPENROUTER_API_KEY=your_api_key");
    println!("   Then edit {} and set ai.model", config_path.display());
    println!();
    println!("2. Set up cloud sync (optional):");
    println!("   export NEON_DATABASE_URL=your_connection_string");
    println!();
    println!("3. Start using todoee:");
    println!("   todoee add \"Buy groceries\"");
    println!("   todoee list");
    println!();
    println!("Run 'todoee config' to see current configuration status.");

    Ok(())
}

/// Show current configuration with status indicators
async fn show_config() -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;
    let config_path = Config::config_path()?;

    // Check if config file exists
    let config_exists = config_path.exists();

    println!("todoee Configuration");
    println!("====================");
    println!();

    // Configuration file status
    if config_exists {
        println!("\u{2713} Config file: {}", config_path.display());
    } else {
        println!("\u{2717} Config file: Not found (using defaults)");
        println!("  Run 'todoee config --init' to create one.");
    }
    println!();

    // AI Configuration
    println!("[AI]");
    println!("  Provider: {}", config.ai.provider);
    println!("  Model: {}", config.ai.model.as_deref().unwrap_or("(not set - AI parsing disabled)"));

    let ai_key_set = env::var(&config.ai.api_key_env).is_ok();
    if ai_key_set {
        println!("  {} {} is set", "\u{2713}", config.ai.api_key_env);
    } else {
        println!("  {} {} is not set", "\u{2717}", config.ai.api_key_env);
    }
    println!();

    // Database Configuration
    println!("[Database]");
    let db_url_set = env::var(&config.database.url_env).is_ok();
    if db_url_set {
        println!("  {} {} is set (cloud sync available)", "\u{2713}", config.database.url_env);
    } else {
        println!("  {} {} is not set (local-only mode)", "\u{2717}", config.database.url_env);
    }
    println!("  Local DB: {}", config.database.local_db_name);
    println!();

    // Notification Configuration
    println!("[Notifications]");
    println!("  Enabled: {}", if config.notifications.enabled { "yes" } else { "no" });
    println!("  Sound: {}", if config.notifications.sound { "yes" } else { "no" });
    println!("  Advance notice: {} minutes", config.notifications.advance_minutes);
    println!();

    // Display Configuration
    println!("[Display]");
    println!("  Theme: {}", config.display.theme);
    println!("  Date format: {}", config.display.date_format);

    Ok(())
}
