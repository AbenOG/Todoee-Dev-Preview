# Offline-First AI Opt-In Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make offline mode the default for all operations, with AI as an explicit opt-in feature via `--ai` flag instead of `--no-ai`.

**Architecture:** Invert the current AI default behavior across CLI and TUI. The `--no-ai` flag becomes `--ai`, documentation is updated to reflect offline-first philosophy, and TUI behavior inverts so Shift+Enter enables AI (not disables it).

**Tech Stack:** Rust, Clap 4, Ratatui

---

## Summary of Changes

| Location | Current Behavior | New Behavior |
|----------|------------------|--------------|
| CLI `add` command | AI on by default, `--no-ai` disables | AI off by default, `--ai` enables |
| TUI Quick Add (A) | Enter = AI, Shift+Enter = no AI | Enter = no AI, Shift+Enter = AI |
| Help text (CLI) | Describes `--no-ai` | Describes `--ai` |
| Help text (TUI) | Mentions AI parsing as default | Mentions AI as opt-in |
| README.md | AI-powered messaging | Offline-first messaging |
| CLI description | "AI-powered" prominent | "Offline-first" prominent |

---

### Task 1: Update CLI Add Command Flag

**Files:**
- Modify: `crates/todoee-cli/src/main.rs:54-70`
- Modify: `crates/todoee-cli/src/commands/add.rs:8-51`

**Step 1: Write the test command to verify current behavior**

Run: `./target/release/todoee add --help 2>&1 | grep -E "(ai|AI)"`
Expected: Shows `--no-ai` flag

**Step 2: Update main.rs - change `--no-ai` to `--ai`**

In `crates/todoee-cli/src/main.rs`, find the Add command definition and change:

```rust
// OLD (around line 59-61)
        /// Skip AI parsing and use description as-is
        #[arg(long)]
        no_ai: bool,

// NEW
        /// Enable AI parsing for natural language (requires API key)
        #[arg(long)]
        ai: bool,
```

**Step 3: Update the command handler in main.rs**

Change the match arm (around line 353-360):

```rust
// OLD
        Commands::Add {
            description,
            no_ai,
            category,
            priority,
        } => {
            commands::add(description, no_ai, category, priority).await?;
        }

// NEW
        Commands::Add {
            description,
            ai,
            category,
            priority,
        } => {
            commands::add(description, ai, category, priority).await?;
        }
```

**Step 4: Update add.rs function signature and logic**

In `crates/todoee-cli/src/commands/add.rs`:

```rust
// OLD (line 8-11)
pub async fn run(
    description: Vec<String>,
    no_ai: bool,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {

// NEW
pub async fn run(
    description: Vec<String>,
    use_ai: bool,
    category: Option<String>,
    priority: Option<i32>,
) -> Result<()> {
```

And change the condition (around line 37-38):

```rust
// OLD
    let mut todo = if no_ai || config.ai.model.is_none() {

// NEW
    let mut todo = if !use_ai || config.ai.model.is_none() {
```

**Step 5: Run test to verify change**

Run: `./target/release/todoee add --help 2>&1 | grep -E "(ai|AI)"`
Expected: Shows `--ai` flag (not `--no-ai`)

**Step 6: Build and verify**

Run: `cargo build --release 2>&1 | tail -5`
Expected: Build succeeds

**Step 7: Commit**

```bash
git add crates/todoee-cli/src/main.rs crates/todoee-cli/src/commands/add.rs
git commit -m "feat(cli): change --no-ai to --ai for opt-in AI parsing

BREAKING CHANGE: AI parsing is now opt-in with --ai flag instead of
opt-out with --no-ai. This aligns with offline-first philosophy."
```

---

### Task 2: Update TUI Quick Add Behavior

**Files:**
- Modify: `crates/todoee-cli/src/tui/handler.rs:333-381`
- Modify: `crates/todoee-cli/src/tui/ui.rs:373` (help line)
- Modify: `crates/todoee-cli/src/tui/ui.rs:486-487` (help modal)

**Step 1: Update handler.rs - invert AI logic**

In `crates/todoee-cli/src/tui/handler.rs`, find the `handle_adding_mode` function and change line 342:

```rust
// OLD (line 341-343)
        KeyCode::Enter => {
            // Use AI if available and Shift not held
            let use_ai = app.has_ai() && !key.modifiers.contains(KeyModifiers::SHIFT);

// NEW
        KeyCode::Enter => {
            // Use AI only if Shift held AND AI is configured
            let use_ai = app.has_ai() && key.modifiers.contains(KeyModifiers::SHIFT);
```

**Step 2: Update the status line help text in ui.rs**

In `crates/todoee-cli/src/tui/ui.rs`, find line 373:

```rust
// OLD
        Mode::Adding => "Enter:submit  Shift+Enter:no-AI  Tab:priority  Esc:cancel",

// NEW
        Mode::Adding => "Enter:submit  Shift+Enter:with-AI  Tab:priority  Esc:cancel",
```

**Step 3: Update the help modal text in ui.rs**

Find lines 486-487:

```rust
// OLD
        Line::from("  Enter       Submit with AI parsing"),
        Line::from("  Shift+Enter Submit without AI"),

// NEW
        Line::from("  Enter       Submit (offline)"),
        Line::from("  Shift+Enter Submit with AI parsing"),
```

**Step 4: Build and verify**

Run: `cargo build --release 2>&1 | tail -5`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/handler.rs crates/todoee-cli/src/tui/ui.rs
git commit -m "feat(tui): make quick add offline by default, Shift+Enter for AI

Inverts the behavior so Enter submits without AI (offline-first)
and Shift+Enter enables AI parsing when configured."
```

---

### Task 3: Update CLI Help Command

**Files:**
- Modify: `crates/todoee-cli/src/commands/help.rs`

**Step 1: Update help.rs with offline-first messaging**

Change references to AI being default:

```rust
// Find and update these lines in HELP_TEXT:

// OLD (around line 15-16)
  Add a task:                 todoee add "Buy groceries"
  Add with AI parsing:        todoee add "Review PR by Friday high priority"

// NEW
  Add a task:                 todoee add "Buy groceries"
  Add with AI parsing:        todoee add "Review PR by Friday" --ai

// OLD (around line 24-27)
  add, a        Add a new task (AI parses natural language)
                  todoee add "task description"
                  todoee add "urgent task" -p 3 -c work
                  todoee add "plain text" --no-ai

// NEW
  add, a        Add a new task (offline by default)
                  todoee add "task description"
                  todoee add "urgent task" -p 3 -c work
                  todoee add "Review PR by Friday" --ai
```

**Step 2: Build and verify help output**

Run: `cargo build --release && ./target/release/todoee help | head -30`
Expected: Shows updated text without `--no-ai`

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/commands/help.rs
git commit -m "docs(cli): update help text for offline-first AI opt-in"
```

---

### Task 4: Update Main CLI Description

**Files:**
- Modify: `crates/todoee-cli/src/main.rs:7-29` (doc comments)
- Modify: `crates/todoee-cli/src/main.rs:28` (about text)

**Step 1: Update the struct-level documentation**

In `crates/todoee-cli/src/main.rs`, update the doc comments:

```rust
// OLD (lines 7-24)
/// todoee - A blazing-fast, AI-powered todo manager for developers
///
/// Todoee combines the power of a CLI with a beautiful TUI (terminal UI),
/// featuring git-like commands, smart AI parsing, and productivity tools
/// like focus timers and insights analytics.
///
/// GETTING STARTED:
///   todoee              Launch interactive TUI (recommended)
///   todoee add "task"   Add a task from command line
///   todoee list         List all pending tasks
///   todoee --help       Show all commands
///
/// EXAMPLES:
///   todoee add "Review PR #123 by tomorrow"    AI parses due date
///   todoee add "Fix bug" -p 3                  High priority task
///   todoee done abc1                           Complete task by short ID
///   todoee undo                                Undo last action
///   todoee focus                               Start 25-min focus session

// NEW
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
```

**Step 2: Update the about attribute**

```rust
// OLD (line 28)
#[command(about = "A blazing-fast, AI-powered todo manager for developers")]

// NEW
#[command(about = "A blazing-fast, offline-first todo manager for developers")]
```

**Step 3: Update the Add command description**

```rust
// OLD (around line 47-52)
    /// Add a new todo (supports natural language with AI)
    ///
    /// Examples:
    ///   todoee add "Buy groceries"
    ///   todoee add "Review PR by Friday" -p 3
    ///   todoee add "Call dentist tomorrow at 2pm"

// NEW
    /// Add a new todo (offline by default, --ai for natural language parsing)
    ///
    /// Examples:
    ///   todoee add "Buy groceries"
    ///   todoee add "Fix bug" -p 3 -c work
    ///   todoee add "Review PR by Friday" --ai
```

**Step 4: Build and verify**

Run: `cargo build --release && ./target/release/todoee --help | head -10`
Expected: Shows "offline-first" not "AI-powered"

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/main.rs
git commit -m "docs(cli): update branding to offline-first"
```

---

### Task 5: Update README.md

**Files:**
- Modify: `README.md`

**Step 1: Update README.md header and features**

Replace AI-centric messaging with offline-first:

```markdown
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
```

**Step 2: Update usage examples**

Change all examples showing AI as default:

```markdown
### Adding Tasks

\`\`\`bash
# Simple task (default, offline)
todoee add "Buy groceries"

# With priority and category
todoee add "Fix bug" --priority 3 --category work

# Enable AI parsing (requires API key)
todoee add "Review PR by Friday high priority" --ai
\`\`\`
```

**Step 3: Update Essential Keybindings table**

```markdown
| `A` | Quick add (offline, Shift+Enter for AI) |
```

**Step 4: Update AI Configuration section**

```markdown
## AI Configuration (Optional)

AI parsing is **opt-in** and not required for normal operation. To enable:

1. Add to `~/.config/todoee/config.toml`:

\`\`\`toml
[ai]
model = "gpt-4"  # or other OpenRouter-compatible model
api_key_env = "OPENROUTER_API_KEY"
\`\`\`

2. Set your API key:
\`\`\`bash
export OPENROUTER_API_KEY="your-api-key"
\`\`\`

3. Use `--ai` flag:
\`\`\`bash
todoee add "Call dentist tomorrow at 2pm" --ai
\`\`\`

Without AI configured, the `--ai` flag has no effect.
```

**Step 5: Commit**

```bash
git add README.md
git commit -m "docs: update README for offline-first philosophy

- Reorder features to emphasize offline-first
- Change examples to show --ai as opt-in
- Update keybindings documentation
- Clarify AI is optional"
```

---

### Task 6: Update TUI Help Modal Headers

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs`

**Step 1: Find and update Quick Add section header**

In `crates/todoee-cli/src/tui/ui.rs`, find the help modal section for Quick Add:

```rust
// OLD (around line 423)
        Line::from("  A           Quick add (single line, AI-powered)"),

// NEW
        Line::from("  A           Quick add (offline, Shift+Enter for AI)"),
```

**Step 2: Build and verify**

Run: `cargo build --release 2>&1 | tail -3`
Expected: Build succeeds

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "docs(tui): update help modal for offline-first quick add"
```

---

## Verification Checklist

After all tasks complete, verify:

1. `todoee --help` shows "offline-first" in description
2. `todoee add --help` shows `--ai` flag (not `--no-ai`)
3. `todoee help` shows offline-first examples
4. TUI help modal (?) shows offline as default for quick add
5. README.md emphasizes offline-first
6. All tests pass: `cargo test --workspace`
