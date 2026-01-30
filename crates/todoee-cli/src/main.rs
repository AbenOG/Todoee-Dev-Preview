use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

mod commands;

/// todoee - AI-powered todo manager
#[derive(Parser)]
#[command(name = "todoee")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new todo item
    Add {
        /// Description of the todo (can be natural language)
        #[arg(required = true)]
        description: Vec<String>,

        /// Skip AI parsing and use description as-is
        #[arg(long)]
        no_ai: bool,

        /// Category for the todo
        #[arg(short, long)]
        category: Option<String>,

        /// Priority (1=low, 2=medium, 3=high)
        #[arg(short, long)]
        priority: Option<i32>,
    },

    /// List todos
    List {
        /// Show only today's todos
        #[arg(long)]
        today: bool,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Show all todos including completed
        #[arg(short, long)]
        all: bool,
    },

    /// Mark a todo as done
    Done {
        /// Todo ID (UUID or short ID)
        id: String,
    },

    /// Delete a todo
    Delete {
        /// Todo ID (UUID or short ID)
        id: String,
    },

    /// Edit a todo
    Edit {
        /// Todo ID (UUID or short ID)
        id: String,

        /// New title
        #[arg(short, long)]
        title: Option<String>,

        /// New category
        #[arg(short, long)]
        category: Option<String>,

        /// New priority (1=low, 2=medium, 3=high)
        #[arg(short, long)]
        priority: Option<i32>,
    },

    /// Sync todos with the server
    Sync,

    /// Configure todoee
    Config {
        /// Initialize configuration with interactive setup
        #[arg(long)]
        init: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("todoee starting");

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Add {
            description,
            no_ai,
            category,
            priority,
        }) => {
            commands::add(description, no_ai, category, priority).await?;
        }
        Some(Commands::List {
            today,
            category,
            all,
        }) => {
            commands::list(today, category, all).await?;
        }
        Some(Commands::Done { id }) => {
            commands::done(id).await?;
        }
        Some(Commands::Delete { id }) => {
            commands::delete(id).await?;
        }
        Some(Commands::Edit {
            id,
            title,
            category,
            priority,
        }) => {
            commands::edit(id, title, category, priority).await?;
        }
        Some(Commands::Sync) => {
            commands::sync().await?;
        }
        Some(Commands::Config { init }) => {
            commands::config(init).await?;
        }
        None => {
            // Default to list --today when no command provided
            commands::list(true, None, false).await?;
        }
    }

    Ok(())
}
