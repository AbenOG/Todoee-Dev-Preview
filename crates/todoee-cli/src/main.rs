use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod tui;

/// todoee - A blazing-fast, offline-first todo manager for developers
///
/// Todoee combines the power of a CLI with a beautiful TUI (terminal UI),
/// featuring git-like commands, optional AI parsing, and productivity tools
/// like focus timers and insights analytics.
///
/// GETTING STARTED:
///   todoee              Launch interactive TUI (recommended)
///   todoee add "task"   Add a task from command line
///   todoee list         List all pending tasks
///   todoee --help       Show all commands
///
/// EXAMPLES:
///   todoee add "Fix bug" -p 3                  High priority task
///   todoee add "Review PR" --ai                AI parses natural language
///   todoee done abc1                           Complete task by short ID
///   todoee undo                                Undo last action
///   todoee focus                               Start 25-min focus session
#[derive(Parser)]
#[command(name = "todoee")]
#[command(author, version)]
#[command(about = "A blazing-fast, offline-first todo manager for developers")]
#[command(long_about = None)]
#[command(after_help = "Run 'todoee' without arguments to launch the interactive TUI.\nRun 'todoee help' for comprehensive guide with examples.")]
#[command(disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run in interactive TUI mode (default when no command given)
    #[arg(short, long, global = true)]
    interactive: bool,
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Check daemon status
    Status,
}

#[derive(Subcommand)]
enum Commands {
    // ═══════════════════════════════════════════════════════════════════
    // CORE COMMANDS
    // ═══════════════════════════════════════════════════════════════════

    /// Add a new todo (offline by default, --ai for natural language parsing)
    ///
    /// Examples:
    ///   todoee add "Buy groceries"
    ///   todoee add "Fix bug" -p 3 -c work
    ///   todoee add "Review PR by Friday" --ai
    ///   todoee add "Meeting" -r "in 30 minutes"
    #[command(visible_alias = "a")]
    Add {
        /// Task description (AI parses dates, priorities from natural language)
        #[arg(required = true)]
        description: Vec<String>,

        /// Enable AI parsing for natural language (requires API key)
        #[arg(long)]
        ai: bool,

        /// Category for the todo
        #[arg(short, long)]
        category: Option<String>,

        /// Priority: 1=low, 2=medium, 3=high
        #[arg(short, long, value_parser = clap::value_parser!(i32).range(1..=3))]
        priority: Option<i32>,

        /// Set a reminder (e.g., "in 30 minutes", "in 1 hour", "tomorrow")
        #[arg(short = 'r', long)]
        reminder: Option<String>,
    },

    /// List todos with optional filters
    ///
    /// Examples:
    ///   todoee list              Show pending todos
    ///   todoee list --today      Show today's todos
    ///   todoee list -c work      Filter by category
    ///   todoee list --all        Include completed
    #[command(visible_alias = "ls")]
    List {
        /// Show only today's todos
        #[arg(long)]
        today: bool,

        /// Filter by category name
        #[arg(short, long)]
        category: Option<String>,

        /// Show all todos including completed
        #[arg(short, long)]
        all: bool,
    },

    /// Mark a todo as complete
    ///
    /// Use short ID prefix (e.g., "abc1") or full UUID
    #[command(visible_alias = "d")]
    Done {
        /// Todo ID (short prefix like "abc1" or full UUID)
        id: String,
    },

    /// Permanently delete a todo
    ///
    /// Use short ID prefix (e.g., "abc1") or full UUID
    #[command(visible_alias = "rm")]
    Delete {
        /// Todo ID (short prefix like "abc1" or full UUID)
        id: String,
    },

    /// Edit a todo's title, category, or priority
    ///
    /// Examples:
    ///   todoee edit abc1 --title "New title"
    ///   todoee edit abc1 -p 3 -c work
    Edit {
        /// Todo ID (short prefix or full UUID)
        id: String,

        /// New title
        #[arg(short, long)]
        title: Option<String>,

        /// New category
        #[arg(short, long)]
        category: Option<String>,

        /// New priority: 1=low, 2=medium, 3=high
        #[arg(short, long, value_parser = clap::value_parser!(i32).range(1..=3))]
        priority: Option<i32>,
    },

    // ═══════════════════════════════════════════════════════════════════
    // GIT-LIKE COMMANDS
    // ═══════════════════════════════════════════════════════════════════

    /// Undo the last operation (like git)
    ///
    /// Reverses add, delete, complete, edit, and stash operations
    Undo,

    /// Redo the last undone operation
    Redo,

    /// Show operation history (like git log)
    ///
    /// Examples:
    ///   todoee log              Show last 10 operations
    ///   todoee log -n 20        Show last 20 operations
    ///   todoee log --oneline    Compact format
    Log {
        /// Number of operations to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: Option<usize>,

        /// Show one operation per line (compact)
        #[arg(long)]
        oneline: bool,
    },

    /// Show recent changes (like git diff)
    ///
    /// Shows what was created, completed, or deleted recently
    Diff {
        /// Show changes in the last N hours
        #[arg(long, default_value = "24")]
        hours: Option<i64>,
    },

    /// Stash todos temporarily (like git stash)
    ///
    /// Subcommands: push, pop, list, clear
    ///
    /// Examples:
    ///   todoee stash push abc1           Stash a todo
    ///   todoee stash push abc1 -m "WIP"  Stash with message
    ///   todoee stash pop                 Restore last stashed
    ///   todoee stash list                Show stash contents
    Stash {
        #[command(subcommand)]
        command: commands::stash::StashCommand,
    },

    // ═══════════════════════════════════════════════════════════════════
    // VIEW COMMANDS
    // ═══════════════════════════════════════════════════════════════════

    /// Show N most recently created todos
    ///
    /// Example: todoee head 10
    Head {
        /// Number of todos to show
        #[arg(default_value = "5")]
        count: usize,

        /// Include completed todos
        #[arg(short, long)]
        all: bool,
    },

    /// Show N oldest todos
    ///
    /// Example: todoee tail 10
    Tail {
        /// Number of todos to show
        #[arg(default_value = "5")]
        count: usize,

        /// Include completed todos
        #[arg(short, long)]
        all: bool,
    },

    /// Show upcoming todos sorted by due date
    ///
    /// Example: todoee upcoming 10
    Upcoming {
        /// Number of todos to show
        #[arg(default_value = "5")]
        count: usize,
    },

    /// Show all overdue todos (past due date)
    Overdue,

    /// Search todos with fuzzy matching
    ///
    /// Searches title and description, ranks by relevance
    ///
    /// Example: todoee search "meeting"
    Search {
        /// Search query (fuzzy matched)
        query: String,
    },

    /// Show detailed view of a single todo
    ///
    /// Displays all fields including metadata
    Show {
        /// Todo ID (short prefix or full UUID)
        id: String,
    },

    // ═══════════════════════════════════════════════════════════════════
    // PRODUCTIVITY COMMANDS
    // ═══════════════════════════════════════════════════════════════════

    /// Start a focus session (Pomodoro timer)
    ///
    /// Interactive timer with keyboard controls:
    ///   Space: pause/resume, q: quit, Enter: complete early
    ///
    /// Examples:
    ///   todoee focus              Focus on highest priority (25 min)
    ///   todoee focus abc1         Focus on specific todo
    ///   todoee focus -d 45        Custom duration (45 min)
    Focus {
        /// Todo ID to focus on (auto-picks if not specified)
        id: Option<String>,

        /// Duration in minutes
        #[arg(short, long, default_value = "25")]
        duration: u32,
    },

    /// Suggest what to work on right now
    ///
    /// Recommends based on priority, due date, and time of day
    Now,

    /// Show productivity insights and analytics
    ///
    /// Displays completion rates, streaks, and patterns
    ///
    /// Example: todoee insights --days 7
    Insights {
        /// Number of days to analyze
        #[arg(short, long, default_value = "30")]
        days: Option<i64>,
    },

    // ═══════════════════════════════════════════════════════════════════
    // BATCH & MAINTENANCE
    // ═══════════════════════════════════════════════════════════════════

    /// Batch operations on multiple todos
    ///
    /// Subcommands: done, delete, priority
    ///
    /// Examples:
    ///   todoee batch done abc1 def2 ghi3
    ///   todoee batch delete abc1 def2
    ///   todoee batch priority 3 abc1 def2
    Batch {
        #[command(subcommand)]
        command: commands::batch::BatchCommand,
    },

    /// Clean up old completed todos and operations
    ///
    /// Examples:
    ///   todoee gc                 Delete items older than 30 days
    ///   todoee gc --days 7        Delete items older than 7 days
    ///   todoee gc --dry-run       Preview what would be deleted
    Gc {
        /// Delete items older than N days
        #[arg(short, long, default_value = "30")]
        days: Option<i64>,

        /// Preview only, don't actually delete
        #[arg(long)]
        dry_run: bool,
    },

    /// Export todos to JSON or CSV file
    ///
    /// Examples:
    ///   todoee export                          Export to JSON (default)
    ///   todoee export -o backup.json           Export to specific file
    ///   todoee export --format csv             Export as CSV
    ///   todoee export --include-completed      Include completed todos
    Export {
        /// Output file path (default: todoee_export_<timestamp>.<format>)
        #[arg(short, long)]
        output: Option<String>,

        /// Export format: json or csv
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Include completed todos in export
        #[arg(long)]
        include_completed: bool,
    },

    /// Import todos from a file
    ///
    /// Examples:
    ///   todoee import backup.json              Import from JSON file
    ///   todoee import backup.json --mode merge Skip existing todos
    ///   todoee import backup.json --mode replace Overwrite existing todos
    Import {
        /// Input file path
        input: String,

        /// Import mode: merge (skip existing) or replace (overwrite)
        #[arg(short, long, default_value = "merge")]
        mode: String,
    },

    /// Sync with cloud storage
    Sync {
        /// Force sync even if recently synced
        #[arg(short, long)]
        force: bool,
    },

    /// Configure todoee settings
    ///
    /// Use --init for interactive setup wizard
    Config {
        /// Run interactive configuration wizard
        #[arg(long)]
        init: bool,
    },

    /// Manage the background daemon
    ///
    /// The daemon runs in the background and handles reminders.
    ///
    /// Examples:
    ///   todoee daemon start    Start the daemon
    ///   todoee daemon stop     Stop the daemon
    ///   todoee daemon status   Check if daemon is running
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    // ═══════════════════════════════════════════════════════════════════
    // HELP
    // ═══════════════════════════════════════════════════════════════════

    /// Show comprehensive help with examples and workflows
    ///
    /// More detailed than --help, includes:
    ///   - Command examples
    ///   - Common workflows
    ///   - Tips and tricks
    #[command(name = "help", visible_alias = "h")]
    Help,
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
            ai,
            category,
            priority,
            reminder,
        } => {
            commands::add(description, ai, category, priority, reminder).await?;
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
        Commands::Sync { force } => {
            commands::sync(force).await?;
        }
        Commands::Config { init } => {
            commands::config(init).await?;
        }
        Commands::Daemon { action } => match action {
            DaemonAction::Start => commands::daemon::run_start().await?,
            DaemonAction::Stop => commands::daemon::run_stop().await?,
            DaemonAction::Status => commands::daemon::run_status().await?,
        },
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
        Commands::Batch { command } => {
            commands::batch::run(command).await?;
        }
        Commands::Gc { days, dry_run } => {
            commands::gc::run(days, dry_run).await?;
        }
        Commands::Export { output, format, include_completed } => {
            commands::export::run(output, format, include_completed).await?;
        }
        Commands::Import { input, mode } => {
            commands::import::run(input, mode).await?;
        }
        Commands::Focus { id, duration } => {
            commands::focus::run(id, duration).await?;
        }
        Commands::Now => {
            commands::now::run().await?;
        }
        Commands::Insights { days } => {
            commands::insights::run(days).await?;
        }
        Commands::Help => {
            commands::help()?;
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
                app.animation_frame = app.animation_frame.wrapping_add(1);
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
