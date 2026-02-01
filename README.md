<div align="center">

# Todoee

**A blazing-fast, offline-first todo manager for developers**

Combines the power of a CLI with a beautiful TUI, featuring git-like commands, optional AI parsing, and productivity tools.

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)
[![Status](https://img.shields.io/badge/status-dev%20preview-yellow.svg)](#development-status)

[Features](#features) · [Installation](#installation) · [Quick Start](#quick-start) · [Usage](#usage) · [Configuration](#configuration)

> **Note:** This is a development preview. Some features are still being implemented. See [Development Status](#development-status) for details.

</div>

---

## Features

| Feature | Description |
|---------|-------------|
| **Offline-First** | Works without internet, no external dependencies |
| **Interactive TUI** | Beautiful terminal interface with vim-style navigation |
| **Smooth Animations** | Polished UI with loading spinners, progress bars, and transitions |
| **Optional AI** | Natural language task parsing (opt-in, requires API key) |
| **Git-Like Commands** | `undo`, `redo`, `stash`, `log`, `diff` |
| **Focus Mode** | Built-in Pomodoro timer with motivational messages |
| **Smart Recommendations** | `now` command suggests what to work on |
| **Productivity Insights** | Track completion rates and patterns |
| **Fuzzy Search** | Find tasks instantly |

## Demo

```
┌─────────────────────────────────────────────────────────────────┐
│  1: Todos    2: Categories    3: Settings                       │
├─────────────────────────────────────────────────────────────────┤
│ > Press 'a' to add task, '/' to search                          │
├─────────────────────────────────────────────────────────────────┤
│  Tasks (4)                                                      │
│ ▸ [ ] !!! Review PR #123                  abc12345  [TODAY]     │
│   [ ] !!  Write documentation             def67890  [2d]        │
│   [ ] !   Clean up old branches           ghi11111              │
│   [x] !!  Fix login bug                   jkl22222              │
├─────────────────────────────────────────────────────────────────┤
│ ✓ Completed: Fix login bug                                      │
├─────────────────────────────────────────────────────────────────┤
│ j/k:nav  a:add  d:done  x:del  u:undo  z:stash  ?:help  q:quit  │
└─────────────────────────────────────────────────────────────────┘
```

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/AbenOG/Todoee-Dev-Preview.git
cd Todoee-Dev-Preview

# Build and install
cargo build --release
cargo install --path crates/todoee-cli

# Or copy manually
cp target/release/todoee ~/.local/bin/
```

### Requirements

- Rust 1.75+ (2024 edition)
- SQLite (bundled)

## Quick Start

```bash
# Launch interactive TUI (recommended)
todoee

# Or use CLI commands
todoee add "Review PR #123 by tomorrow"
todoee list
todoee done abc1
```

## Usage

### Interactive TUI

Launch with `todoee` (no arguments). The TUI provides a complete task management experience with keyboard-driven navigation.

#### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `g` / `G` | Jump to top / bottom |
| `1` `2` `3` | Switch tabs (Todos, Categories, Settings) |

#### Core Actions

| Key | Action |
|-----|--------|
| `a` | Add task (full editor) |
| `A` | Quick add (Shift+Enter for AI) |
| `d` / `Enter` | Mark as done |
| `x` | Delete task |
| `e` | Edit task |
| `v` / `Space` | View details |

#### Git-Like Commands

| Key | Action |
|-----|--------|
| `u` | Undo last action |
| `Ctrl+r` | Redo |
| `z` | Stash selected task |
| `Z` | Pop from stash |

#### Filters & Sorting

| Key | Action |
|-----|--------|
| `/` | Search (fuzzy) |
| `t` | Toggle today filter |
| `o` | Toggle overdue filter |
| `p` | Cycle priority filter |
| `c` | Cycle category filter |
| `s` | Cycle sort field |
| `Tab` | Show/hide completed |

#### Productivity

| Key | Action |
|-----|--------|
| `n` | Jump to recommended task |
| `f` | Start 25-min focus session |
| `F` | Start 5-min quick focus |
| `i` | View productivity insights |

### Command Line Interface

#### Adding Tasks

```bash
# Simple task (offline by default)
todoee add "Buy groceries"

# With priority (1=low, 2=medium, 3=high)
todoee add "Fix critical bug" --priority 3

# With category
todoee add "Team meeting" --category work

# With AI parsing (requires configuration)
todoee add "Review PR by Friday high priority" --ai
```

#### Viewing Tasks

```bash
todoee list              # Pending tasks
todoee list --today      # Due today
todoee list --all        # Include completed
todoee overdue           # Past due date
todoee search "meeting"  # Fuzzy search
todoee show abc1         # Detailed view
```

#### Managing Tasks

```bash
todoee done abc1         # Mark complete
todoee delete abc1       # Delete
todoee edit abc1 --title "New title"
todoee edit abc1 --priority 3
```

#### Git-Like Operations

```bash
todoee undo              # Undo last action
todoee redo              # Redo
todoee log               # View history
todoee diff              # Recent changes
todoee stash push abc1   # Stash a task
todoee stash pop         # Restore stashed
```

#### Batch Operations

```bash
todoee batch done abc1 def2 ghi3
todoee batch delete abc1 def2
todoee batch priority 3 abc1 def2
```

#### Productivity

```bash
todoee now               # What should I work on?
todoee focus             # 25-min Pomodoro
todoee focus abc1 -d 45  # Custom duration
todoee insights          # Weekly stats
```

#### Import/Export

```bash
todoee export                    # Export to JSON (default)
todoee export -f csv             # Export to CSV
todoee export -o backup.json     # Specify output file
todoee import backup.json        # Import from file
todoee import backup.json -m replace  # Overwrite existing
```

#### Cloud Sync

```bash
# Setup
export NEON_DATABASE_URL="postgres://user:pass@host/db"

# Sync
todoee sync                      # Sync with cloud
```

Sync features:
- **Bi-directional**: Upload local changes, download remote changes
- **Categories first**: Categories sync before todos (foreign key safety)
- **Delete propagation**: Local deletes sync to cloud and won't re-download
- **Conflict resolution**: Last-write-wins based on timestamps

#### Daemon & Reminders

```bash
todoee daemon start              # Start background daemon
todoee daemon stop               # Stop daemon
todoee daemon status             # Check daemon status

# Add task with reminder
todoee add "Meeting" -r "in 30 minutes"
todoee add "Call mom" -r "tomorrow"
```

## Focus Mode

Built-in Pomodoro timer with progress tracking and motivational messages:

```
┌──────────────────────────────────────────┐
│              FOCUS MODE                  │
│                                          │
│          Review PR #123                  │
│                                          │
│               12:34                      │
│                                          │
│      [████████████████░░░░░░░░░░░░]      │
│                                          │
│          Making progress!                │
│                                          │
│  Space: pause   q/Esc: cancel   Enter: done  │
└──────────────────────────────────────────┘
```

- Dynamic progress bar with color changes (green → yellow → red)
- Blinking colon separator animation
- Progress-based motivational messages
- Pause/resume support

## Priority Levels

| Level | Display | CLI Flag | Color |
|-------|---------|----------|-------|
| High | `!!!` | `-p 3` | Red |
| Medium | `!!` | `-p 2` | Yellow |
| Low | `!` | `-p 1` | Green |

## Configuration

Configuration is stored at `~/.config/todoee/config.toml`.

```bash
# Interactive setup
todoee config --init
```

### AI Configuration (Optional)

AI parsing is **opt-in** and not required. To enable:

```toml
[ai]
model = "gpt-4"  # or any OpenRouter-compatible model
api_key = "your-api-key"
```

The AI parses natural language for:
- Due dates: "by Friday", "tomorrow", "next week"
- Priorities: "urgent", "high priority", "low"
- Categories: "for work", "personal"

## Data Storage

| Type | Location |
|------|----------|
| Config | `~/.config/todoee/config.toml` |
| Database | `~/.local/share/todoee/todoee.db` |

## Security

Todoee implements security best practices to protect your data:

### Memory Safety

- **API Key Zeroing** - API keys are securely zeroed from memory when the AI client is dropped, preventing exposure via memory dumps or core dumps
- **No Unsafe Code** - Tests use thread-safe environment variable handling via `temp-env` crate

### Input Validation

- **Path Traversal Protection** - Database names are validated to prevent directory traversal attacks (e.g., `../../../etc/passwd`)
- **Input Length Limits** - Task descriptions are limited to 10,000 characters to prevent DoS attacks

### File System Security

- **Restrictive Permissions** - Config directory is created with mode `0700` and config file with mode `0600` (Unix), preventing other users from reading your configuration

### Network Security

- **Request Timeouts** - AI API requests have a 30-second timeout to prevent indefinite hangs
- **Sanitized Error Messages** - Database errors are logged via structured tracing, not exposed to stdout/stderr

### Best Practices

- Store API keys in environment variables, not in config files
- Use a dedicated API key with minimal permissions for AI features
- Keep your system updated with latest security patches

## UI Animations

Todoee features polished UI animations for a smooth experience:

- **Loading Spinners** - 8 different ASCII animation styles
- **Progress Bars** - Visual feedback for long operations
- **Pulsing Cursors** - Subtle selection indicator animation
- **Status Icons** - Animated success/error feedback
- **Tab Transitions** - Smooth view switching
- **Focus Timer** - Animated countdown with blinking separator
- **Insights Reveal** - Stats count up when opening

All animations run at 250ms intervals for smooth, non-distracting motion.

## Tips

### Keyboard-Only Workflow

1. `todoee` - Open TUI
2. `A` - Quick add task
3. `n` - Jump to recommended
4. `f` - Start focus session
5. `d` - Mark done when complete
6. Repeat!

### Morning Routine

```bash
todoee overdue      # Check what's late
todoee now          # Get recommendation
todoee focus        # Start working
```

### Weekly Cleanup

```bash
todoee gc --dry-run  # Preview cleanup
todoee gc            # Clean old items
todoee insights      # Review productivity
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "AI parsing failed" | Check API key or use offline mode (remove `--ai`) |
| "Todo not found" | Use `todoee list --all` or try longer ID prefix |
| "Nothing to undo" | Undo history is limited; some operations can't be undone |

## Project Structure

```
todoee/
└── crates/
    ├── todoee-cli/      # CLI + TUI application
    ├── todoee-core/     # Business logic & database
    └── todoee-daemon/   # Background service for reminders
```

## Development Status

This is a **development preview**. The core functionality is working, but some features are still in progress.

### Fully Implemented

- Interactive TUI with vim-style navigation
- CLI commands (add, list, done, delete, edit, search)
- Git-like operations (undo, redo, stash, log, diff)
- Focus mode with Pomodoro timer
- Productivity insights
- Fuzzy search
- Categories and priorities
- UI animations and loading indicators
- Local SQLite database
- Import/Export (JSON and CSV)
- Cloud Sync (Neon Postgres) with delete propagation
- Daemon Service (background reminders)
- Notifications (desktop alerts)

### In Development

| Feature | Status | Notes |
|---------|--------|-------|
| AI Parsing | Beta | Works but requires external API key |
| Config Wizard | Partial | `--init` flag not fully implemented |

### Known Limitations

- **AI Parsing** - Requires external API key and network connection
- **Config Wizard** - `--init` flag not fully implemented

### Roadmap

1. **v0.2** - Cloud sync with Neon Postgres (completed)
2. **v0.3** - Desktop notifications and reminders (completed)
3. **v0.4** - Background daemon service (completed)
4. **v1.0** - Stable release with full feature set

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

This project is in active development - check the [Issues](https://github.com/AbenOG/Todoee-Dev-Preview/issues) for ways to help.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<div align="center">

**Built with Rust + Ratatui**

</div>
