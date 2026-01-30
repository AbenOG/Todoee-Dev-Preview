# Modern SaaS-Inspired CLI Features

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform todoee into a blazing-fast, modern SaaS-inspired CLI that feels like Notion/Linear/Todoist meets the terminal - with innovative features that don't exist in other CLI todo apps.

**Architecture:** Event-driven async core, lazy evaluation, memory-mapped indexes, background workers, Unix philosophy integration.

**Tech Stack:** Rust (zero-cost abstractions), tokio (async), tantivy (full-text search), notify (file watching), tui-rs (reactive UI)

---

## Philosophy: The "Developer's Second Brain"

Not just a todo list - a **cognitive offload system** that:
1. Captures thoughts instantly (inbox zero latency)
2. Surfaces the right task at the right time (context-aware)
3. Learns your patterns (ML-lite productivity insights)
4. Integrates with your workflow (pipes, hooks, APIs)
5. Never gets in your way (< 50ms for any operation)

---

## PART 1: BLAZING FAST CORE

### 1.1 Instant Fuzzy Search (`todoee /`)

**The killer feature**: Type `/` anywhere and get instant fuzzy search across ALL todos, completed or not.

```bash
# Interactive fuzzy finder (like fzf)
todoee /

# Inline fuzzy search
todoee / "buy milk"  # Fuzzy matches "buy almond milk tomorrow"

# Search with filters
todoee / "meeting" --cat work --pri high
```

**Implementation:**
- Tantivy-based full-text index (rebuilt in background on changes)
- Trigram matching for typo tolerance
- Frecency scoring (frequent + recent = higher rank)
- Sub-10ms response time guaranteed

### 1.2 Smart Caching & Lazy Loading

```rust
// Memory-mapped todo index for instant access
// Only deserialize what's needed
// Background sync with SQLite
```

### 1.3 Parallel Command Execution

```bash
# Run multiple operations in parallel
todoee parallel "done abc123" "add 'New task'" "delete def456"
```

---

## PART 2: PRODUCTIVITY INTELLIGENCE

### 2.1 Focus Mode (`todoee focus`)

**Pomodoro meets deep work tracking:**

```bash
# Start a focus session on a specific todo
todoee focus abc123

# Start with custom duration
todoee focus abc123 --duration 45m

# Focus on category
todoee focus --cat "deep-work"

# Output:
# ╭──────────────────────────────────────╮
# │  FOCUS: Implement user auth          │
# │  ████████████░░░░░░░░  25:00 / 45:00 │
# │                                      │
# │  [p] pause  [s] skip  [d] done  [q]  │
# ╰──────────────────────────────────────╯
```

**Features:**
- Blocks other todos from view (zen mode)
- Tracks actual time spent (stored in todo metadata)
- Desktop notifications at intervals
- Integrates with system "Do Not Disturb"
- Completion triggers satisfaction animation

### 2.2 Smart Prioritization (`todoee now`)

**AI-powered "what should I do right now":**

```bash
todoee now

# Output:
# Based on: time of day, energy patterns, deadlines, context
#
# Recommended right now:
#  1. [!!!] Review PR #423 (due in 2h, you're most productive now)
#  2. [!! ] Write tests for auth (blocked by nothing, quick win)
#  3. [!  ] Plan sprint (usually do this at 10am)
#
# Not recommended:
#  - Deep work tasks (you have a meeting in 30min)
#  - Low energy tasks (your energy is high right now)
```

**Factors considered:**
- Time until deadline (urgency)
- Estimated duration vs available time
- Historical completion patterns (when do you do X?)
- Energy level (configurable or inferred from time)
- Context (home/work detected via network/location)
- Dependencies (what's unblocked?)
- Streak potential (will this help a streak?)

### 2.3 Insights & Analytics (`todoee insights`)

```bash
todoee insights

# ┌─────────────────────────────────────────────────────┐
# │ YOUR PRODUCTIVITY INSIGHTS (Last 30 days)           │
# ├─────────────────────────────────────────────────────┤
# │                                                     │
# │ Completion Rate:  ████████████░░░ 78% (+5% vs last) │
# │ Avg Time/Task:    47 min (estimate accuracy: 72%)   │
# │ Most Productive:  Tuesday 9-11am                    │
# │ Category Focus:   Work 45% | Personal 30% | Health  │
# │                                                     │
# │ Completion Heatmap (last 4 weeks):                  │
# │ Mon ██░█████░███░██░░                               │
# │ Tue █████████████████                               │
# │ Wed ███░░██████░███░█                               │
# │ Thu ██████░█████████░                               │
# │ Fri ░░██████░███░░░░░                               │
# │                                                     │
# │ Suggestions:                                        │
# │ • You complete 2x more tasks before noon            │
# │ • Tuesday is your power day - schedule hard tasks   │
# │ • You often overestimate by 20% - try smaller tasks │
# └─────────────────────────────────────────────────────┘

todoee insights --export json > productivity.json
```

### 2.4 Streaks & Gamification (`todoee streak`)

```bash
todoee streak

# 🔥 Current Streak: 12 days
# 📈 Longest Streak: 34 days
# ⭐ Level: Productivity Pro (Level 7)
# 🏆 Achievements Unlocked:
#    [x] First Task - Complete your first todo
#    [x] Streak Starter - 7 day streak
#    [x] Centurion - Complete 100 todos
#    [ ] Deep Worker - 10 focus sessions over 1 hour
#    [ ] Inbox Zero - Clear all todos in a day
```

### 2.5 Time Estimation Learning (`todoee estimate`)

```bash
# Add time estimate
todoee add "Build login page" --estimate 2h

# After completion, record actual
todoee done abc123 --actual 3h

# See estimation accuracy
todoee estimate stats
# Your estimates are typically 25% under actual
# Suggested multiplier: 1.25x

# Get AI-suggested estimate based on similar past tasks
todoee estimate "Build signup page"
# Suggested: 3h45m (based on "Build login page" + complexity)
```

---

## PART 3: WORKFLOW AUTOMATION

### 3.1 Smart Recurring (`todoee recur`)

Not just "every Monday" - intelligent recurrence:

```bash
# Basic recurrence
todoee add "Weekly review" --recur "every friday 5pm"

# Smart recurrence (reschedules based on completion)
todoee add "Haircut" --recur "every 3 weeks after completion"

# Workday-aware
todoee add "Standup" --recur "every workday 9am"

# Complex patterns
todoee add "Quarterly review" --recur "first monday of jan,apr,jul,oct"

# Habit-style (flexible window)
todoee add "Exercise" --recur "3x per week" --flexible
```

### 3.2 Dependencies & Chains (`todoee chain`)

```bash
# Create a sequence of dependent tasks
todoee chain "Design mockup" -> "Get feedback" -> "Implement UI" -> "Write tests"

# Shows as:
# ┌─ Design mockup [in progress]
# ├─ Get feedback [blocked]
# ├─ Implement UI [blocked]
# └─ Write tests [blocked]

# When you complete "Design mockup":
# ┌─ Design mockup [done]
# ├─ Get feedback [ready] ← auto-unblocked!
# ├─ Implement UI [blocked]
# └─ Write tests [blocked]

# View dependency graph
todoee chain show --graph
```

### 3.3 Templates (`todoee template`)

```bash
# Create a template
todoee template create "new-feature" << EOF
- [ ] Write RFC document
- [ ] Get team feedback
- [ ] Create branch
- [ ] Implement feature
- [ ] Write tests
- [ ] Update docs
- [ ] Create PR
- [ ] Address review comments
- [ ] Merge
EOF

# Use template (with variable substitution)
todoee template use "new-feature" --var feature="dark mode"

# Creates:
# - Write RFC document for dark mode
# - Get team feedback on dark mode
# ...

# List templates
todoee template list

# Share template
todoee template export "new-feature" > new-feature.yaml
todoee template import < sprint-planning.yaml
```

### 3.4 Hooks System (`todoee hook`)

Git-style hooks for automation:

```bash
# List available hooks
todoee hook list
# pre-add, post-add, pre-complete, post-complete,
# pre-delete, post-delete, daily, weekly

# Add a hook
todoee hook add post-complete "notify-send 'Task done!' '$TODOEE_TITLE'"

# Hook environment variables:
# $TODOEE_ID, $TODOEE_TITLE, $TODOEE_CATEGORY,
# $TODOEE_PRIORITY, $TODOEE_DUE_DATE

# Example hooks:
# - Post to Slack when high-priority task completes
# - Log time to external time tracker
# - Update project management tool
# - Play celebration sound
```

### 3.5 Smart Contexts (`todoee ctx`)

GTD-style contexts with auto-detection:

```bash
# Define contexts
todoee ctx add @work --detect "network:corp-wifi OR time:9-17 weekday"
todoee ctx add @home --detect "network:home-wifi"
todoee ctx add @errands --detect "location:outside"
todoee ctx add @low-energy --detect "time:after-8pm"

# Tag todos with context
todoee add "Review budget" @work
todoee add "Buy groceries" @errands
todoee add "Watch tutorial" @low-energy

# Auto-filter based on current context
todoee list --auto-context
# Automatically shows @work tasks when on corp-wifi

# See context-appropriate tasks
todoee ctx now
# Current context: @work (detected: corp-wifi)
# Showing 12 @work tasks...
```

---

## PART 4: UNIX PHILOSOPHY INTEGRATION

### 4.1 Pipe-Friendly Output (`todoee --pipe`)

```bash
# JSON output for piping
todoee list --json | jq '.[] | select(.priority == 3)'

# TSV for spreadsheets
todoee list --tsv > todos.tsv

# Markdown for docs
todoee list --md >> daily-log.md

# Feed into other tools
todoee list --json | gron | grep title

# Create from pipe
echo "Buy milk\nCall mom\nReview PR" | todoee add --batch

# Bulk operations
todoee list --ids-only --cat work | xargs -I {} todoee done {}
```

### 4.2 Watch Mode (`todoee watch`)

```bash
# Live-updating TUI dashboard
todoee watch

# Watch specific filter
todoee watch --today --cat work

# Minimal watch (single line, for tmux status)
todoee watch --minimal
# Output: 📋 5 pending | ⚠️ 2 overdue | 🔥 8 day streak
```

### 4.3 Integration Commands

```bash
# Calendar sync
todoee cal sync --google
todoee cal show --week

# Git integration
todoee git  # Shows todos related to current branch/repo

# Slack integration
todoee slack post --channel #standup --template daily

# Export/Import
todoee export --format todoist > backup.json
todoee import --format things < things-export.json
todoee import --format markdown < todos.md
```

---

## PART 5: INNOVATIVE UI/UX

### 5.1 Zen Mode (`todoee zen`)

```bash
todoee zen

# Full-screen, distraction-free single task view:
#
# ┌─────────────────────────────────────────────────────────────┐
# │                                                             │
# │                                                             │
# │                                                             │
# │                    Implement user auth                      │
# │                                                             │
# │                         !!!  HIGH                           │
# │                      Due: 2 hours                           │
# │                                                             │
# │                                                             │
# │                                                             │
# │              [d] done    [s] skip    [q] quit               │
# └─────────────────────────────────────────────────────────────┘
```

### 5.2 Quick Capture (`todoee inbox` / `todoee i`)

```bash
# Ultra-fast capture (no AI, no parsing, just dump)
todoee i "random thought about the project"

# Opens nano-style quick input
todoee inbox
# Type anything, Ctrl+D to save, each line = 1 todo

# Review inbox later
todoee inbox review  # Guided triage of unprocessed items
```

### 5.3 Natural Language Everything

```bash
# These all work:
todoee "call mom tomorrow at 5pm"
todoee "buy milk !high @errands"
todoee "review PR #423 in 2 hours"
todoee "meeting with john next tuesday 2pm for 1 hour"
todoee "exercise 3x this week"

# Smart parsing understands:
# - Relative dates: tomorrow, next week, in 3 days
# - Times: 5pm, 17:00, noon, midnight
# - Priority: !high, !low, !!!, p1, urgent
# - Categories: @work, @home, #project-name
# - Duration: for 1 hour, takes 30min
# - Recurrence: every day, weekly, monthly
# - People: with @john, assign @sarah
# - Links: automatically extracts URLs
```

### 5.4 Kanban View (`todoee board`)

```bash
todoee board

# ┌──────────────┬──────────────┬──────────────┬──────────────┐
# │   BACKLOG    │   TODAY      │  IN PROGRESS │    DONE      │
# ├──────────────┼──────────────┼──────────────┼──────────────┤
# │ ░ Design     │ █ Review PR  │ █ Auth impl  │ ▓ Setup CI   │
# │ ░ Research   │ █ Write docs │              │ ▓ DB schema  │
# │ ░ Plan Q2    │              │              │ ▓ API design │
# │              │              │              │              │
# └──────────────┴──────────────┴──────────────┴──────────────┘
#
# [h/l] move task  [j/k] select  [Enter] details  [a] add  [q] quit
```

### 5.5 Timeline View (`todoee timeline`)

```bash
todoee timeline

# ═══════════════════════════════════════════════════════════════
#  TODAY                    TOMORROW               THIS WEEK
# ═══════════════════════════════════════════════════════════════
#  09:00 ▓ Standup          10:00 ▓ Review PR     Mon: Sprint plan
#  10:00 ░░░░░░░░░░░        14:00 ▓ 1:1 meeting   Wed: Deploy v2
#  11:00 █ Implement auth                         Fri: Retrospective
#  12:00 ░░░░░░░░░░░
#  14:00 █ Team sync
#  15:00 ░░░░░░░░░░░
# ═══════════════════════════════════════════════════════════════
```

### 5.6 Review Mode (`todoee review`)

Weekly review wizard:

```bash
todoee review

# ╭─────────────────────────────────────────────────────────╮
# │              WEEKLY REVIEW                              │
# ├─────────────────────────────────────────────────────────┤
# │ Step 1/5: Incomplete from last week                     │
# │                                                         │
# │ These tasks are overdue. What do you want to do?        │
# │                                                         │
# │ > [ ] Review Q4 budget (7 days overdue)                 │
# │   [r] reschedule  [d] delete  [→] defer  [✓] done      │
# │                                                         │
# │ > [ ] Call dentist (3 days overdue)                     │
# │   [r] reschedule  [d] delete  [→] defer  [✓] done      │
# ╰─────────────────────────────────────────────────────────╯
```

---

## PART 6: COLLABORATION & SHARING

### 6.1 Share Links (`todoee share`)

```bash
# Generate shareable link (no account needed)
todoee share abc123
# https://todoee.sh/t/x7k9m2 (expires in 7 days)

# Share a filtered list
todoee share --filter "cat:work AND due:this-week"
# https://todoee.sh/l/p3n8q1

# Share with edit access
todoee share abc123 --edit
```

### 6.2 Delegate (`todoee delegate`)

```bash
# Delegate a task
todoee delegate abc123 --to "john@example.com" --message "Can you handle this?"

# Track delegated tasks
todoee list --delegated
# Shows tasks you've assigned to others

# Waiting for
todoee list --waiting
# Shows tasks blocked on others
```

### 6.3 Comments & Activity (`todoee comment`)

```bash
# Add comment
todoee comment abc123 "Waiting on design team feedback"

# View activity
todoee activity abc123
# Shows: created, edited, commented, status changes

# Subscribe to updates
todoee watch abc123  # Get notified of any changes
```

---

## PART 7: EXTENSIBILITY

### 7.1 Plugin System (`todoee plugin`)

```bash
# List available plugins
todoee plugin search

# Install plugin
todoee plugin install todist-sync
todoee plugin install pomodoro-sounds
todoee plugin install ai-categorizer

# Plugin API exposes:
# - Hooks (pre/post events)
# - Custom commands
# - UI widgets
# - Data transformers
```

### 7.2 Custom Commands (`todoee alias`)

```bash
# Create custom commands
todoee alias morning "list --today --sort due"
todoee alias standup "list --completed-yesterday --format standup"
todoee alias cleanup "gc --days 7 && archive --older 30d"

# Use them
todoee morning
todoee standup
```

### 7.3 Local API Server (`todoee serve`)

```bash
# Start local REST API
todoee serve --port 8080

# Endpoints:
# GET  /todos
# POST /todos
# PUT  /todos/:id
# DELETE /todos/:id
# POST /todos/:id/complete
# GET  /stats
# POST /focus/start
# GET  /focus/status

# Use from scripts, Alfred, Raycast, etc.
curl localhost:8080/todos | jq
```

### 7.4 MCP Server (`todoee mcp`)

```bash
# Expose as Model Context Protocol server
todoee mcp serve

# Now Claude/GPT can:
# - Query your todos
# - Add tasks from conversations
# - Mark things complete
# - Suggest prioritization
```

---

## PART 8: SPEED OPTIMIZATIONS

### 8.1 Architecture for Speed

```
┌─────────────────────────────────────────────────────────────┐
│                    TODOEE SPEED ARCHITECTURE                 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  CLI Input ──→ Command Parser (< 1ms)                       │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           MEMORY-MAPPED INDEX (mmap)                 │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐    │   │
│  │  │ ID Index │ │ FTS Index│ │ Frecency Cache   │    │   │
│  │  │ (B-tree) │ │(Tantivy) │ │ (LRU, 1000 items)│    │   │
│  │  └──────────┘ └──────────┘ └──────────────────────┘    │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  SQLite (WAL mode) ◄──── Background Sync Worker            │
│       │                                                     │
│       ▼                                                     │
│  Response (< 50ms guaranteed)                               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 8.2 Benchmarks Target

| Operation | Target | How |
|-----------|--------|-----|
| `todoee list` | < 10ms | Memory-mapped index |
| `todoee add` | < 20ms | Async write, instant ack |
| `todoee /` (search) | < 15ms | Tantivy FTS |
| `todoee done` | < 15ms | Optimistic update |
| TUI startup | < 100ms | Lazy widget loading |
| TUI input latency | < 16ms | 60fps rendering |

### 8.3 Lazy Everything

```rust
// Only load what's needed
// - First 50 todos loaded immediately
// - Rest loaded on scroll
// - Categories loaded on demand
// - Completed todos lazy-loaded
// - History loaded on undo/log only
```

---

## COMMAND SUMMARY

### Capture & Create
| Command | Description |
|---------|-------------|
| `todoee "task"` | Natural language add |
| `todoee i "thought"` | Quick inbox capture |
| `todoee inbox` | Batch quick capture |
| `todoee template use` | Create from template |

### View & Query
| Command | Description |
|---------|-------------|
| `todoee /` | Fuzzy search |
| `todoee now` | AI-recommended next task |
| `todoee board` | Kanban view |
| `todoee timeline` | Calendar timeline |
| `todoee zen` | Single task focus |
| `todoee watch` | Live dashboard |

### Productivity
| Command | Description |
|---------|-------------|
| `todoee focus` | Pomodoro timer |
| `todoee streak` | Gamification stats |
| `todoee insights` | Analytics |
| `todoee estimate` | Time estimation |
| `todoee review` | Weekly review wizard |

### Workflow
| Command | Description |
|---------|-------------|
| `todoee chain` | Task dependencies |
| `todoee recur` | Smart recurring |
| `todoee ctx` | Context detection |
| `todoee hook` | Automation hooks |
| `todoee defer` | Smart snooze |

### Collaboration
| Command | Description |
|---------|-------------|
| `todoee share` | Share links |
| `todoee delegate` | Assign to others |
| `todoee comment` | Add comments |

### Integration
| Command | Description |
|---------|-------------|
| `todoee cal` | Calendar sync |
| `todoee git` | Git integration |
| `todoee slack` | Slack posting |
| `todoee serve` | REST API |
| `todoee mcp` | AI assistant integration |

### Meta
| Command | Description |
|---------|-------------|
| `todoee plugin` | Plugin management |
| `todoee alias` | Custom commands |
| `todoee export/import` | Data portability |

---

## DIFFERENTIATORS vs COMPETITION

| Feature | todoee | Todoist | Things | taskwarrior |
|---------|--------|---------|--------|-------------|
| CLI-first | ✅ | ❌ | ❌ | ✅ |
| Instant fuzzy search | ✅ | ❌ | ❌ | ❌ |
| AI prioritization | ✅ | ❌ | ❌ | ❌ |
| Pomodoro built-in | ✅ | ❌ | ❌ | ❌ |
| Unix pipes | ✅ | ❌ | ❌ | ✅ |
| Productivity insights | ✅ | Basic | Basic | ❌ |
| Context auto-detect | ✅ | ❌ | ❌ | ❌ |
| MCP/AI integration | ✅ | ❌ | ❌ | ❌ |
| < 50ms operations | ✅ | ❌ | ❌ | ✅ |
| Offline-first | ✅ | ❌ | ✅ | ✅ |
| Share without signup | ✅ | ❌ | ❌ | ❌ |

---

## IMPLEMENTATION PRIORITY

### Phase 1: Speed Foundation (Week 1)
1. Memory-mapped index
2. Tantivy FTS integration
3. Async write pipeline
4. Benchmark suite

### Phase 2: Core Intelligence (Week 2)
5. `todoee /` fuzzy search
6. `todoee now` smart prioritization
7. `todoee focus` pomodoro
8. Natural language parsing improvements

### Phase 3: Productivity Features (Week 3)
9. `todoee insights` analytics
10. `todoee streak` gamification
11. `todoee estimate` time tracking
12. `todoee review` wizard

### Phase 4: Workflow (Week 4)
13. `todoee chain` dependencies
14. `todoee recur` smart recurring
15. `todoee ctx` contexts
16. `todoee hook` automation

### Phase 5: Views (Week 5)
17. `todoee board` kanban
18. `todoee timeline` view
19. `todoee zen` mode
20. `todoee watch` live dashboard

### Phase 6: Integration (Week 6)
21. `todoee serve` API
22. `todoee mcp` server
23. Calendar integration
24. Export/import formats

---

## TAGLINE IDEAS

- "Your terminal. Your tasks. Zero friction."
- "The todo app that thinks like a developer."
- "git for your life."
- "Capture everything. Find anything. Do what matters."
- "Finally, a todo app as fast as your thoughts."
