use anyhow::Result;

const HELP_TEXT: &str = r#"
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              TODOEE HELP                                       ║
║                   A blazing-fast, AI-powered todo manager                      ║
╚═══════════════════════════════════════════════════════════════════════════════╝

┌─────────────────────────────────────────────────────────────────────────────────┐
│  QUICK START                                                                    │
└─────────────────────────────────────────────────────────────────────────────────┘

  Launch interactive TUI:     todoee
  Add a task:                 todoee add "Buy groceries"
  Add with AI parsing:        todoee add "Review PR by Friday high priority"
  List tasks:                 todoee list
  Complete a task:            todoee done abc1
  Undo last action:           todoee undo

┌─────────────────────────────────────────────────────────────────────────────────┐
│  CORE COMMANDS                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘

  add, a        Add a new task (AI parses natural language)
                  todoee add "task description"
                  todoee add "urgent task" -p 3 -c work
                  todoee add "plain text" --no-ai

  list, ls      List tasks with filters
                  todoee list                    # Pending tasks
                  todoee list --today            # Due today
                  todoee list --all              # Include completed
                  todoee list -c work            # By category

  done, d       Mark task as complete
                  todoee done abc1               # Use short ID prefix

  delete, rm    Permanently delete a task
                  todoee delete abc1

  edit          Modify a task
                  todoee edit abc1 --title "New title"
                  todoee edit abc1 -p 3 -c urgent

┌─────────────────────────────────────────────────────────────────────────────────┐
│  GIT-LIKE COMMANDS                                                              │
└─────────────────────────────────────────────────────────────────────────────────┘

  undo          Reverse the last operation
                  todoee undo

  redo          Re-apply the last undone operation
                  todoee redo

  log           View operation history
                  todoee log                     # Last 10 operations
                  todoee log -n 20               # Last 20
                  todoee log --oneline           # Compact format

  diff          Show recent changes
                  todoee diff                    # Last 24 hours
                  todoee diff --hours 48         # Last 48 hours

  stash         Temporarily hide tasks
                  todoee stash push abc1         # Stash a task
                  todoee stash push abc1 -m "WIP"
                  todoee stash pop               # Restore last stashed
                  todoee stash list              # View stash
                  todoee stash clear             # Clear all stashed

┌─────────────────────────────────────────────────────────────────────────────────┐
│  VIEW COMMANDS                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘

  head          Show most recently created tasks
                  todoee head 10

  tail          Show oldest tasks
                  todoee tail 10

  upcoming      Show tasks by due date (soonest first)
                  todoee upcoming 5

  overdue       Show all past-due tasks
                  todoee overdue

  search        Fuzzy search tasks
                  todoee search "meeting"

  show          View detailed task info
                  todoee show abc1

┌─────────────────────────────────────────────────────────────────────────────────┐
│  PRODUCTIVITY                                                                   │
└─────────────────────────────────────────────────────────────────────────────────┘

  now           Get smart recommendation for what to work on
                  todoee now

  focus         Start a Pomodoro focus session
                  todoee focus                   # 25 min, auto-picks task
                  todoee focus abc1              # Focus on specific task
                  todoee focus -d 45             # Custom duration (45 min)

                Focus mode controls:
                  Space    Pause/Resume
                  Enter    Complete early
                  q        Quit

  insights      View productivity analytics
                  todoee insights                # Last 30 days
                  todoee insights --days 7       # Last 7 days

┌─────────────────────────────────────────────────────────────────────────────────┐
│  BATCH OPERATIONS                                                               │
└─────────────────────────────────────────────────────────────────────────────────┘

  batch done      Complete multiple tasks at once
                    todoee batch done abc1 def2 ghi3

  batch delete    Delete multiple tasks
                    todoee batch delete abc1 def2

  batch priority  Set priority for multiple tasks
                    todoee batch priority 3 abc1 def2 ghi3

┌─────────────────────────────────────────────────────────────────────────────────┐
│  MAINTENANCE                                                                    │
└─────────────────────────────────────────────────────────────────────────────────┘

  gc            Clean up old completed tasks and history
                  todoee gc                      # Delete > 30 days old
                  todoee gc --days 7             # Delete > 7 days old
                  todoee gc --dry-run            # Preview only

  sync          Sync with remote server (if configured)
                  todoee sync

  config        Configure settings
                  todoee config --init           # Interactive setup

┌─────────────────────────────────────────────────────────────────────────────────┐
│  PRIORITY LEVELS                                                                │
└─────────────────────────────────────────────────────────────────────────────────┘

    -p 1    Low priority      (green  ! )
    -p 2    Medium priority   (yellow !!)
    -p 3    High priority     (red   !!!)

┌─────────────────────────────────────────────────────────────────────────────────┐
│  TASK IDs                                                                       │
└─────────────────────────────────────────────────────────────────────────────────┘

  Tasks have UUIDs, but you only need to type the first few characters:

    Full ID:    a1b2c3d4-e5f6-7890-abcd-ef1234567890
    Short ID:   a1b2  or  a1b2c3  (enough to be unique)

  Example:      todoee done a1b2

┌─────────────────────────────────────────────────────────────────────────────────┐
│  COMMON WORKFLOWS                                                               │
└─────────────────────────────────────────────────────────────────────────────────┘

  Morning routine:
    todoee overdue              # Check what's late
    todoee now                  # Get recommendation
    todoee focus                # Start working

  Quick capture:
    todoee add "idea or task"   # AI parses it for you

  End of day:
    todoee insights --days 1    # See today's progress
    todoee upcoming 5           # Plan for tomorrow

  Weekly cleanup:
    todoee gc --dry-run         # Preview cleanup
    todoee gc                   # Remove old items
    todoee insights             # Review productivity

┌─────────────────────────────────────────────────────────────────────────────────┐
│  MORE HELP                                                                      │
└─────────────────────────────────────────────────────────────────────────────────┘

  Command-specific help:        todoee <command> --help
  Interactive TUI help:         Press '?' in TUI
  Full documentation:           https://github.com/youruser/todoee

"#;

pub fn run() -> Result<()> {
    println!("{}", HELP_TEXT);
    Ok(())
}
