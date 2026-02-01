use thiserror::Error;

#[derive(Error, Debug)]
pub enum TodoeeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error(
        "AI service error: {message}\n\nTo fix this:\n1. Check that your AI model string is correct in ~/.config/todoee/config.toml\n2. Verify your OPENROUTER_API_KEY environment variable is set\n3. Ensure you have API credits available\n\nExample config:\n[ai]\nmodel = \"anthropic/claude-3-haiku\""
    )]
    AiService { message: String },

    #[error(
        "AI parsing failed: {message}\n\nThe AI could not parse your input. Please try:\n1. Being more specific (e.g., \"buy milk tomorrow at 9am\" instead of \"milk\")\n2. Adding the task manually with: todoee add --title \"your task\" --category \"Work\""
    )]
    AiParsing { message: String },

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error(
        "Network error: {0}\n\nYou appear to be offline. Your changes are saved locally and will sync when you reconnect."
    )]
    Network(String),

    #[error("Sync conflict: {0}")]
    SyncConflict(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, TodoeeError>;
