use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod tui;

/// todoee - AI-powered todo manager
#[derive(Parser)]
#[command(name = "todoee")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run in interactive TUI mode (default when no command given)
    #[arg(short, long, global = true)]
    interactive: bool,
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

    /// Undo the last operation
    Undo,

    /// Redo the last undone operation
    Redo,

    /// Show operation history
    Log {
        /// Number of operations to show (default: 10)
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// Show one operation per line
        #[arg(long)]
        oneline: bool,
    },

    /// Show recent changes
    Diff {
        /// Show changes in the last N hours (default: 24)
        #[arg(long)]
        hours: Option<i64>,
    },

    /// Show last N todos (most recent)
    Head {
        /// Number of todos to show (default: 5)
        #[arg(default_value = "5")]
        count: usize,

        /// Include completed todos
        #[arg(short, long)]
        all: bool,
    },

    /// Show oldest N todos
    Tail {
        /// Number of todos to show (default: 5)
        #[arg(default_value = "5")]
        count: usize,

        /// Include completed todos
        #[arg(short, long)]
        all: bool,
    },

    /// Show next N upcoming todos by due date
    Upcoming {
        /// Number of todos to show (default: 5)
        #[arg(default_value = "5")]
        count: usize,
    },

    /// Show all overdue todos
    Overdue,

    /// Search todos (fuzzy matching)
    Search {
        /// Search query
        query: String,
    },

    /// Show detailed view of a todo
    Show {
        /// Todo ID (or prefix)
        id: String,
    },

    /// Stash todos temporarily
    Stash {
        #[command(subcommand)]
        command: commands::stash::StashCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // If no command provided or -i flag, run interactive mode
    if cli.command.is_none() || cli.interactive {
        return run_interactive().await;
    }

    // Handle subcommands
    match cli.command.unwrap() {
        Commands::Add {
            description,
            no_ai,
            category,
            priority,
        } => {
            commands::add(description, no_ai, category, priority).await?;
        }
        Commands::List {
            today,
            category,
            all,
        } => {
            commands::list(today, category, all).await?;
        }
        Commands::Done { id } => {
            commands::done(id).await?;
        }
        Commands::Delete { id } => {
            commands::delete(id).await?;
        }
        Commands::Edit {
            id,
            title,
            category,
            priority,
        } => {
            commands::edit(id, title, category, priority).await?;
        }
        Commands::Sync => {
            commands::sync().await?;
        }
        Commands::Config { init } => {
            commands::config(init).await?;
        }
        Commands::Undo => {
            commands::undo().await?;
        }
        Commands::Redo => {
            commands::redo().await?;
        }
        Commands::Log { limit, oneline } => {
            commands::log::run(limit, oneline).await?;
        }
        Commands::Diff { hours } => {
            commands::diff::run(hours).await?;
        }
        Commands::Head { count, all } => {
            commands::head::head(count, all).await?;
        }
        Commands::Tail { count, all } => {
            commands::head::tail(count, all).await?;
        }
        Commands::Upcoming { count } => {
            commands::upcoming::upcoming(count).await?;
        }
        Commands::Overdue => {
            commands::upcoming::overdue().await?;
        }
        Commands::Search { query } => {
            commands::search::run(&query).await?;
        }
        Commands::Show { id } => {
            commands::show::run(&id).await?;
        }
        Commands::Stash { command } => {
            commands::stash::run(command).await?;
        }
    }

    Ok(())
}

/// Run the interactive TUI
async fn run_interactive() -> Result<()> {
    // Initialize application state
    let mut app = tui::App::new().await?;

    // Initialize terminal
    let mut terminal = tui::Tui::new()?;

    // Create event handler
    let events = tui::EventHandler::new(250);

    // Main loop
    while app.running {
        // Render UI
        terminal.draw(|frame| tui::ui::render(&app, frame))?;

        // Handle events
        match events.next()? {
            tui::Event::Tick => {
                // Could refresh data periodically here
            }
            tui::Event::Key(key) => {
                tui::handle_key_event(&mut app, key).await?;
            }
            tui::Event::Mouse(_) => {
                // Mouse support could be added here
            }
            tui::Event::Resize(_, _) => {
                // Terminal handles resize automatically
            }
        }
    }

    Ok(())
}
