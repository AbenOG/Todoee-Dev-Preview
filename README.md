# Todoee

A blazing-fast, offline-first todo manager for developers. Combines the power of a CLI with a beautiful TUI (terminal UI), featuring git-like commands, optional AI parsing, and productivity tools.

## Features

- **Offline-First** - Works without internet, no external dependencies required
- **Interactive TUI** - Beautiful terminal interface with vim-style navigation
- **Optional AI Parsing** - Natural language task creation with `--ai` flag
- **Git-Like Commands** - Undo, redo, stash, log, diff
- **Focus Mode** - Built-in Pomodoro timer
- **Smart Recommendations** - "Now" command suggests what to work on
- **Productivity Insights** - Track completion rates and patterns
- **Fuzzy Search** - Find tasks quickly
- **Categories & Priorities** - Organize your work

## Quick Start

```bash
# Install (from source)
cargo install --path crates/todoee-cli

# Launch interactive TUI (recommended)
todoee

# Or use CLI commands
todoee add "Review PR #123 by tomorrow"
todoee list
todoee done abc1
```

## Installation

### From Source

```bash
git clone https://github.com/youruser/todoee
cd todoee
cargo build --release
cp target/release/todoee ~/.local/bin/
```

### Configuration

On first run, todoee creates a config file at `~/.config/todoee/config.toml`.

```bash
# Interactive setup wizard
todoee config --init
```

## Usage Guide

### Interactive TUI

Launch the TUI by running `todoee` without arguments:

```
┌─────────────────────────────────────────────────────────────┐
│ 1: Todos   2: Categories   3: Settings                      │
├─────────────────────────────────────────────────────────────┤
│ > Press 'a' to add task, '/' to search                      │
├─────────────────────────────────────────────────────────────┤
│ ▸ [ ] !!! Review PR #123                    [abc12345] [TODAY]│
│   [ ] !!  Write documentation               [def67890] [2d]   │
│   [ ] !   Clean up old branches             [ghi11111]        │
│   [x] !!  Fix login bug                     [jkl22222]        │
├─────────────────────────────────────────────────────────────┤
│ ✓ Completed: Fix login bug                                   │
├─────────────────────────────────────────────────────────────┤
│ j/k:nav a:add d:done x:del u:undo z:stash ?:help q:quit     │
└─────────────────────────────────────────────────────────────┘
```

### Essential Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `a` | Add task (full editor) |
| `A` | Quick add (offline, Shift+Enter for AI) |
| `d` / `Enter` | Mark as done |
| `x` | Delete task |
| `e` | Edit task |
| `v` / `Space` | View details |
| `u` | Undo |
| `Ctrl+r` | Redo |
| `z` / `Z` | Stash / Pop |
| `/` | Search |
| `?` | Help |
| `q` | Quit |

### Command Line Interface

#### Adding Tasks

```bash
# Simple task (default, offline)
todoee add "Buy groceries"

# With priority and category
todoee add "Fix bug" --priority 3 --category work

# Enable AI parsing (requires API key)
todoee add "Review PR by Friday high priority" --ai
```

#### Viewing Tasks

```bash
# List pending tasks
todoee list

# List with filters
todoee list --today
todoee list --category work
todoee list --all  # Include completed

# Different views
todoee head 10      # 10 most recent
todoee tail 10      # 10 oldest
todoee upcoming 5   # Next 5 by due date
todoee overdue      # Past due date

# Search
todoee search "meeting"

# Detailed view
todoee show abc1
```

#### Completing & Editing

```bash
# Mark done (use short ID prefix)
todoee done abc1

# Delete
todoee delete abc1

# Edit
todoee edit abc1 --title "New title"
todoee edit abc1 --priority 3 --category urgent
```

#### Git-Like Commands

```bash
# Undo/Redo
todoee undo
todoee redo

# View history
todoee log
todoee log -n 20 --oneline

# See recent changes
todoee diff
todoee diff --hours 48

# Stash (hide temporarily)
todoee stash push abc1
todoee stash push abc1 -m "WIP feature"
todoee stash pop
todoee stash list
todoee stash clear
```

#### Batch Operations

```bash
# Complete multiple
todoee batch done abc1 def2 ghi3

# Delete multiple
todoee batch delete abc1 def2

# Set priority for multiple
todoee batch priority 3 abc1 def2 ghi3
```

#### Productivity

```bash
# What should I work on?
todoee now

# Focus session (Pomodoro)
todoee focus              # 25 min, auto-picks task
todoee focus abc1         # Focus on specific task
todoee focus -d 45        # Custom duration

# Analytics
todoee insights
todoee insights --days 7
```

#### Maintenance

```bash
# Clean up old items
todoee gc                 # Delete > 30 days old
todoee gc --days 7        # Delete > 7 days old
todoee gc --dry-run       # Preview only

# Sync with server
todoee sync
```

## Filters & Sorting (TUI)

| Key | Filter/Sort |
|-----|-------------|
| `t` | Toggle today filter |
| `o` | Toggle overdue filter |
| `p` | Cycle priority filter |
| `c` | Cycle category filter |
| `s` | Cycle sort field |
| `S` | Toggle sort order |
| `Tab` | Show/hide completed |

## Priority Levels

| Level | Symbol | CLI Flag |
|-------|--------|----------|
| High | `!!!` (red) | `-p 3` |
| Medium | `!!` (yellow) | `-p 2` |
| Low | `!` (green) | `-p 1` |

## Focus Mode

The built-in Pomodoro timer helps you concentrate:

```
┌──────────────────────────────────────┐
│              FOCUS MODE              │
│                                      │
│        Review PR #123                │
│                                      │
│              23:45                   │
│                                      │
│   [###############--------------]    │
│                                      │
│ Space:pause  q/Esc:cancel  Enter:done│
└──────────────────────────────────────┘
```

Controls:
- `Space` - Pause/Resume
- `Enter` - Complete early
- `q` / `Esc` - Cancel

## AI Configuration (Optional)

AI parsing is **opt-in** and not required for normal operation. To enable AI-powered task parsing, add to `~/.config/todoee/config.toml`:

```toml
[ai]
model = "gpt-4"  # or "claude-3-opus", etc.
api_key = "your-api-key"  # or use environment variable
```

The AI can parse:
- Due dates: "by Friday", "tomorrow", "next week"
- Priorities: "urgent", "high priority", "low priority"
- Categories: "for work", "personal task"

## Data Storage

- **Config**: `~/.config/todoee/config.toml`
- **Database**: `~/.local/share/todoee/todoee.db` (SQLite)

## Tips & Tricks

### 1. Use Short IDs
Don't type full UUIDs. Use the first few characters:
```bash
todoee done abc1    # Instead of full UUID
```

### 2. Chain Commands
```bash
todoee add "Quick task" && todoee list
```

### 3. Morning Routine
```bash
todoee overdue      # Check what's late
todoee now          # Get recommendation
todoee focus        # Start working
```

### 4. Weekly Cleanup
```bash
todoee gc --dry-run  # Preview cleanup
todoee gc            # Actually clean up
todoee insights      # Review productivity
```

### 5. Keyboard-Only Workflow
The TUI is designed for keyboard efficiency:
1. `todoee` to open
2. `A` to quick-add (Shift+Enter for AI)
3. `n` to jump to recommended task
4. `f` to start focus
5. `d` when done
6. Repeat!

## Troubleshooting

### "AI parsing failed"
- Check your API key in config
- Remove `--ai` flag to use offline mode

### "Todo not found"
- Use `todoee list --all` to see completed todos
- Try a longer ID prefix if ambiguous

### "Nothing to undo"
- Undo history is limited
- Some operations (like gc) can't be undone

## Contributing

Contributions welcome! Please read our contributing guidelines.

## License

MIT License - see LICENSE file for details.
