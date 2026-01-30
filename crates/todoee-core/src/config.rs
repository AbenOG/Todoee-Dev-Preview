//! Application configuration module
//!
//! Provides configuration management with TOML file support,
//! environment variable integration, and sensible defaults.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Main application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub display: DisplayConfig,
}

/// AI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_ai_api_key_env")]
    pub api_key_env: String,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_url_env")]
    pub url_env: String,
    #[serde(default = "default_local_db_name")]
    pub local_db_name: String,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub sound: bool,
    #[serde(default = "default_advance_minutes")]
    pub advance_minutes: u32,
}

/// Display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_date_format")]
    pub date_format: String,
}

// Default value functions for serde
fn default_ai_provider() -> String {
    "openrouter".to_string()
}

fn default_ai_api_key_env() -> String {
    "OPENROUTER_API_KEY".to_string()
}

fn default_database_url_env() -> String {
    "NEON_DATABASE_URL".to_string()
}

fn default_local_db_name() -> String {
    "cache.db".to_string()
}

fn default_true() -> bool {
    true
}

fn default_advance_minutes() -> u32 {
    15
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_date_format() -> String {
    "%Y-%m-%d".to_string()
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: default_ai_provider(),
            model: None,
            api_key_env: default_ai_api_key_env(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url_env: default_database_url_env(),
            local_db_name: default_local_db_name(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            sound: default_true(),
            advance_minutes: default_advance_minutes(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            date_format: default_date_format(),
        }
    }
}

impl Config {
    /// Returns the configuration directory path (~/.config/todoee/)
    pub fn config_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("todoee");
        Ok(config_dir)
    }

    /// Returns the configuration file path (~/.config/todoee/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Returns the local database path (~/.config/todoee/cache.db)
    pub fn local_db_path(&self) -> Result<PathBuf> {
        Ok(Self::config_dir()?.join(&self.database.local_db_name))
    }

    /// Returns the authentication file path (~/.config/todoee/auth.json)
    pub fn auth_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("auth.json"))
    }

    /// Load configuration from file, or return default if file doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save configuration to file, creating the directory if needed
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = Self::config_path()?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .with_context(|| format!("Failed to create config directory: {}", config_dir.display()))?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;

        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        Ok(())
    }

    /// Get the AI API key from the environment variable
    pub fn get_ai_api_key(&self) -> Result<String> {
        env::var(&self.ai.api_key_env)
            .with_context(|| format!("Environment variable {} not set", self.ai.api_key_env))
    }

    /// Get the database URL from the environment variable
    pub fn get_database_url(&self) -> Result<String> {
        env::var(&self.database.url_env)
            .with_context(|| format!("Environment variable {} not set", self.database.url_env))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_values() {
        let config = Config::default();

        // Test AiConfig defaults
        assert_eq!(config.ai.provider, "openrouter");
        assert!(config.ai.model.is_none());
        assert_eq!(config.ai.api_key_env, "OPENROUTER_API_KEY");

        // Test DatabaseConfig defaults
        assert_eq!(config.database.url_env, "NEON_DATABASE_URL");
        assert_eq!(config.database.local_db_name, "cache.db");

        // Test NotificationConfig defaults
        assert!(config.notifications.enabled);
        assert!(config.notifications.sound);
        assert_eq!(config.notifications.advance_minutes, 15);

        // Test DisplayConfig defaults
        assert_eq!(config.display.theme, "dark");
        assert_eq!(config.display.date_format, "%Y-%m-%d");
    }

    #[test]
    fn test_config_load_from_toml() {
        let toml_content = r#"
[ai]
provider = "anthropic"
model = "claude-3-opus"
api_key_env = "ANTHROPIC_API_KEY"

[database]
url_env = "CUSTOM_DB_URL"
local_db_name = "my_cache.db"

[notifications]
enabled = false
sound = false
advance_minutes = 30

[display]
theme = "light"
date_format = "%d/%m/%Y"
"#;

        let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

        // Test AiConfig
        assert_eq!(config.ai.provider, "anthropic");
        assert_eq!(config.ai.model, Some("claude-3-opus".to_string()));
        assert_eq!(config.ai.api_key_env, "ANTHROPIC_API_KEY");

        // Test DatabaseConfig
        assert_eq!(config.database.url_env, "CUSTOM_DB_URL");
        assert_eq!(config.database.local_db_name, "my_cache.db");

        // Test NotificationConfig
        assert!(!config.notifications.enabled);
        assert!(!config.notifications.sound);
        assert_eq!(config.notifications.advance_minutes, 30);

        // Test DisplayConfig
        assert_eq!(config.display.theme, "light");
        assert_eq!(config.display.date_format, "%d/%m/%Y");
    }

    #[test]
    fn test_config_partial_toml() {
        // Test that partial TOML uses defaults for missing fields
        let toml_content = r#"
[ai]
provider = "custom_provider"

[notifications]
advance_minutes = 60
"#;

        let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

        // Specified values should be set
        assert_eq!(config.ai.provider, "custom_provider");
        assert_eq!(config.notifications.advance_minutes, 60);

        // Missing values should use defaults
        assert!(config.ai.model.is_none());
        assert_eq!(config.ai.api_key_env, "OPENROUTER_API_KEY");
        assert_eq!(config.database.url_env, "NEON_DATABASE_URL");
        assert_eq!(config.database.local_db_name, "cache.db");
        assert!(config.notifications.enabled);
        assert!(config.notifications.sound);
        assert_eq!(config.display.theme, "dark");
        assert_eq!(config.display.date_format, "%Y-%m-%d");
    }

    #[test]
    fn test_config_dir_path() {
        let config_dir = Config::config_dir().expect("Failed to get config dir");
        assert!(config_dir.ends_with("todoee"));
    }

    #[test]
    fn test_config_path() {
        let config_path = Config::config_path().expect("Failed to get config path");
        assert!(config_path.ends_with("config.toml"));
    }

    #[test]
    fn test_auth_path() {
        let auth_path = Config::auth_path().expect("Failed to get auth path");
        assert!(auth_path.ends_with("auth.json"));
    }

    #[test]
    fn test_local_db_path() {
        let config = Config::default();
        let db_path = config.local_db_path().expect("Failed to get local db path");
        assert!(db_path.ends_with("cache.db"));
    }

    #[test]
    fn test_local_db_path_custom() {
        let mut config = Config::default();
        config.database.local_db_name = "custom.db".to_string();
        let db_path = config.local_db_path().expect("Failed to get local db path");
        assert!(db_path.ends_with("custom.db"));
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");

        // Verify it can be deserialized back
        let parsed: Config = toml::from_str(&toml_str).expect("Failed to parse");
        assert_eq!(parsed.ai.provider, config.ai.provider);
        assert_eq!(parsed.display.theme, config.display.theme);
    }

    #[test]
    fn test_get_ai_api_key_missing() {
        let config = Config::default();
        // Clear the env var to ensure the test is reliable
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("OPENROUTER_API_KEY") };
        let result = config.get_ai_api_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_database_url_missing() {
        let config = Config::default();
        // Clear the env var to ensure the test is reliable
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("NEON_DATABASE_URL") };
        let result = config.get_database_url();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_ai_api_key_set() {
        let config = Config::default();
        // SAFETY: This is a single-threaded test
        unsafe { env::set_var("OPENROUTER_API_KEY", "test_key_123") };
        let result = config.get_ai_api_key();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_key_123");
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("OPENROUTER_API_KEY") };
    }

    #[test]
    fn test_get_database_url_set() {
        let config = Config::default();
        // SAFETY: This is a single-threaded test
        unsafe { env::set_var("NEON_DATABASE_URL", "postgres://localhost/test") };
        let result = config.get_database_url();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "postgres://localhost/test");
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("NEON_DATABASE_URL") };
    }
}
