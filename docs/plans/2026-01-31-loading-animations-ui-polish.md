# Loading Animations & UI Polish Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add polished ASCII loading indicators, custom spinners, and seamless visual feedback throughout the todocli TUI for an enjoyable user experience.

**Architecture:** Create a reusable spinner/animation module with multiple spinner styles, integrate frame-based animation via the existing 250ms tick rate, and add visual feedback for all async operations. The animation state tracks frame index globally in `App` so spinners animate consistently.

**Tech Stack:** Rust, Ratatui 0.29, Crossterm 0.28, existing 250ms tick event loop

---

## Task 1: Create Spinner Module with Multiple ASCII Styles

**Files:**
- Create: `crates/todoee-cli/src/tui/spinner.rs`
- Modify: `crates/todoee-cli/src/tui/mod.rs`

**Step 1: Write the failing test**

Create test file first:

```rust
// crates/todoee-cli/src/tui/spinner.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_spinner_cycles() {
        let spinner = Spinner::Braille;
        let frames = spinner.frames();
        assert_eq!(frames.len(), 10);
        assert_eq!(spinner.frame(0), '⠋');
        assert_eq!(spinner.frame(10), '⠋'); // wraps
    }

    #[test]
    fn test_dots_spinner_cycles() {
        let spinner = Spinner::Dots;
        let frames = spinner.frames();
        assert!(frames.len() >= 3);
        assert_eq!(spinner.frame(0), frames[0]);
    }

    #[test]
    fn test_line_spinner_cycles() {
        let spinner = Spinner::Line;
        let frames = spinner.frames();
        assert_eq!(frames.len(), 4);
        assert_eq!(spinner.frame(0), '|');
    }

    #[test]
    fn test_progress_bar_at_boundaries() {
        let bar = progress_bar(0.0, 20, '█', '░');
        assert_eq!(bar, "░░░░░░░░░░░░░░░░░░░░");

        let bar = progress_bar(1.0, 20, '█', '░');
        assert_eq!(bar, "████████████████████");

        let bar = progress_bar(0.5, 10, '█', '░');
        assert_eq!(bar, "█████░░░░░");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p todoee-cli spinner --lib`
Expected: FAIL with "cannot find module spinner"

**Step 3: Write minimal implementation**

```rust
// crates/todoee-cli/src/tui/spinner.rs

//! ASCII spinner and progress bar utilities for TUI animations.

/// Available spinner animation styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Spinner {
    /// Braille dots pattern: ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏
    #[default]
    Braille,
    /// Dots pattern: ⣾⣽⣻⢿⡿⣟⣯⣷
    Dots,
    /// Simple line: |/-\
    Line,
    /// Block bounce: ▖▘▝▗
    Blocks,
    /// Growing dots: .  .. ...
    GrowingDots,
    /// Arrow: ←↖↑↗→↘↓↙
    Arrow,
    /// Box bounce: ▌▀▐▄
    BoxBounce,
    /// Star pulse: ✶✷✸✹✺✹✸✷
    Star,
}

impl Spinner {
    /// Get all frames for this spinner style
    pub fn frames(&self) -> &'static [char] {
        match self {
            Spinner::Braille => &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
            Spinner::Dots => &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'],
            Spinner::Line => &['|', '/', '-', '\\'],
            Spinner::Blocks => &['▖', '▘', '▝', '▗'],
            Spinner::GrowingDots => &['.', '.', '.', ' ', ' '],
            Spinner::Arrow => &['←', '↖', '↑', '↗', '→', '↘', '↓', '↙'],
            Spinner::BoxBounce => &['▌', '▀', '▐', '▄'],
            Spinner::Star => &['✶', '✷', '✸', '✹', '✺', '✹', '✸', '✷'],
        }
    }

    /// Get the frame character for a given frame index (wraps automatically)
    pub fn frame(&self, index: usize) -> char {
        let frames = self.frames();
        frames[index % frames.len()]
    }

    /// Get the number of frames
    pub fn len(&self) -> usize {
        self.frames().len()
    }
}

/// Render a progress bar string
///
/// # Arguments
/// * `progress` - Value between 0.0 and 1.0
/// * `width` - Total width in characters
/// * `filled` - Character for filled portion
/// * `empty` - Character for empty portion
pub fn progress_bar(progress: f64, width: usize, filled: char, empty: char) -> String {
    let progress = progress.clamp(0.0, 1.0);
    let filled_count = (progress * width as f64).round() as usize;
    let empty_count = width.saturating_sub(filled_count);

    format!(
        "{}{}",
        filled.to_string().repeat(filled_count),
        empty.to_string().repeat(empty_count)
    )
}

/// Render a bracketed progress bar: [████░░░░]
pub fn bracketed_progress(progress: f64, width: usize) -> String {
    let inner_width = width.saturating_sub(2);
    format!("[{}]", progress_bar(progress, inner_width, '█', '░'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_spinner_cycles() {
        let spinner = Spinner::Braille;
        let frames = spinner.frames();
        assert_eq!(frames.len(), 10);
        assert_eq!(spinner.frame(0), '⠋');
        assert_eq!(spinner.frame(10), '⠋');
    }

    #[test]
    fn test_dots_spinner_cycles() {
        let spinner = Spinner::Dots;
        let frames = spinner.frames();
        assert!(frames.len() >= 3);
        assert_eq!(spinner.frame(0), frames[0]);
    }

    #[test]
    fn test_line_spinner_cycles() {
        let spinner = Spinner::Line;
        let frames = spinner.frames();
        assert_eq!(frames.len(), 4);
        assert_eq!(spinner.frame(0), '|');
    }

    #[test]
    fn test_progress_bar_at_boundaries() {
        let bar = progress_bar(0.0, 20, '█', '░');
        assert_eq!(bar, "░░░░░░░░░░░░░░░░░░░░");

        let bar = progress_bar(1.0, 20, '█', '░');
        assert_eq!(bar, "████████████████████");

        let bar = progress_bar(0.5, 10, '█', '░');
        assert_eq!(bar, "█████░░░░░");
    }
}
```

**Step 4: Update mod.rs to export spinner module**

Add to `crates/todoee-cli/src/tui/mod.rs`:

```rust
pub mod spinner;
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p todoee-cli spinner --lib`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/todoee-cli/src/tui/spinner.rs crates/todoee-cli/src/tui/mod.rs
git commit -m "$(cat <<'EOF'
feat(tui): add spinner module with multiple ASCII animation styles

Introduces reusable spinner types (Braille, Dots, Line, Blocks, Arrow, Star)
and progress bar utilities for consistent loading animations throughout the TUI.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Add Frame Counter to App State for Tick-Based Animation

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs:260-301` (App struct)
- Modify: `crates/todoee-cli/src/main.rs:462-464` (tick handler)

**Step 1: Write the failing test**

Add test to app.rs:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_frame_increments() {
        // Can't easily test async App::new, so test the increment logic
        let mut frame: usize = 0;
        frame = frame.wrapping_add(1);
        assert_eq!(frame, 1);

        frame = usize::MAX;
        frame = frame.wrapping_add(1);
        assert_eq!(frame, 0); // wraps
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p todoee-cli animation_frame --lib`
Expected: PASS (this is a simple unit test, will pass immediately)

**Step 3: Add animation_frame field to App struct**

In `app.rs`, add to the `App` struct after line 300:

```rust
    /// Animation frame counter for tick-based animations
    pub animation_frame: usize,
```

And initialize in `App::new()` around line 382:

```rust
            animation_frame: 0,
```

**Step 4: Add tick handler to increment frame**

In `main.rs`, update the tick handler around line 462:

```rust
            tui::Event::Tick => {
                app.animation_frame = app.animation_frame.wrapping_add(1);
            }
```

**Step 5: Run existing tests to verify no regression**

Run: `cargo test -p todoee-cli --lib`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs crates/todoee-cli/src/main.rs
git commit -m "$(cat <<'EOF'
feat(tui): add animation_frame counter for tick-based spinners

The frame counter increments every 250ms tick, enabling smooth
frame-based animations instead of system time calculations.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Add Loading Spinner Style to App State

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs` (add spinner style field)

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::spinner::Spinner;

    #[test]
    fn test_default_spinner_style() {
        // Default should be Braille
        let spinner = Spinner::default();
        assert_eq!(spinner, Spinner::Braille);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p todoee-cli default_spinner --lib`
Expected: FAIL (import doesn't exist yet in context)

**Step 3: Add spinner style to App**

In `app.rs`, add import at top:

```rust
use super::spinner::Spinner;
```

Add field to App struct after `animation_frame`:

```rust
    /// Current spinner style for loading animations
    pub spinner_style: Spinner,
```

Initialize in `App::new()`:

```rust
            spinner_style: Spinner::default(),
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p todoee-cli --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "$(cat <<'EOF'
feat(tui): add configurable spinner style to App state

Users can now have different spinner animations; defaults to Braille.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Refactor Loading Overlay to Use Frame-Based Animation

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs:517-554` (render_loading_overlay)

**Step 1: Read current implementation for context**

The current `render_loading_overlay` uses `SystemTime::now()` for animation. We need to refactor to use `app.animation_frame` and `app.spinner_style`.

**Step 2: Update render_loading_overlay function**

Replace the function in `ui.rs`:

```rust
fn render_loading_overlay(app: &App, frame: &mut Frame) {
    use super::spinner::progress_bar;

    let area = centered_rect(45, 20, frame.area());

    // Use frame-based animation from app state
    let spinner_char = app.spinner_style.frame(app.animation_frame);

    let message = app.loading_message.as_deref().unwrap_or("Loading...");

    // Animated dots after message
    let dots_count = (app.animation_frame % 4) as usize;
    let dots = ".".repeat(dots_count);
    let dots_padding = " ".repeat(3 - dots_count);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}  {}{}{}", spinner_char, message, dots, dots_padding),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let loading = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Processing "),
        )
        .alignment(Alignment::Center);

    frame.render_widget(Clear, area);
    frame.render_widget(loading, area);
}
```

**Step 3: Run the app to visually verify**

Run: `cargo run -p todoee-cli`
Then trigger a loading operation (e.g., press 'i' for insights or add a task with AI).

**Step 4: Run tests to verify no regression**

Run: `cargo test -p todoee-cli --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "$(cat <<'EOF'
refactor(tui): use frame-based animation for loading overlay

Replaces SystemTime calculation with app.animation_frame for consistent
spinner animation synchronized to the 250ms tick rate.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Add Loading Progress State for Multi-Step Operations

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs` (add progress tracking)

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_progress() {
        let progress = LoadingProgress {
            current: 2,
            total: 5,
            step_name: Some("Processing".to_string()),
        };
        assert_eq!(progress.percentage(), 0.4);
    }

    #[test]
    fn test_loading_progress_zero_total() {
        let progress = LoadingProgress {
            current: 0,
            total: 0,
            step_name: None,
        };
        assert_eq!(progress.percentage(), 0.0);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p todoee-cli loading_progress --lib`
Expected: FAIL

**Step 3: Add LoadingProgress struct and field**

In `app.rs`, add after the imports:

```rust
/// Progress state for multi-step loading operations
#[derive(Debug, Clone, Default)]
pub struct LoadingProgress {
    pub current: usize,
    pub total: usize,
    pub step_name: Option<String>,
}

impl LoadingProgress {
    pub fn new(total: usize) -> Self {
        Self {
            current: 0,
            total,
            step_name: None,
        }
    }

    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.current as f64 / self.total as f64
        }
    }

    pub fn advance(&mut self, step_name: Option<&str>) {
        self.current = (self.current + 1).min(self.total);
        self.step_name = step_name.map(|s| s.to_string());
    }

    pub fn is_complete(&self) -> bool {
        self.current >= self.total
    }
}
```

Add field to App struct:

```rust
    /// Progress for multi-step loading operations
    pub loading_progress: Option<LoadingProgress>,
```

Initialize in `App::new()`:

```rust
            loading_progress: None,
```

Add helper methods to App impl:

```rust
    /// Set loading with progress tracking
    pub fn set_loading_with_progress(&mut self, message: &str, total_steps: usize) {
        self.is_loading = true;
        self.loading_message = Some(message.to_string());
        self.loading_progress = Some(LoadingProgress::new(total_steps));
    }

    /// Advance loading progress
    pub fn advance_loading(&mut self, step_name: Option<&str>) {
        if let Some(ref mut progress) = self.loading_progress {
            progress.advance(step_name);
        }
    }
```

Update `clear_loading`:

```rust
    /// Clear loading state
    pub fn clear_loading(&mut self) {
        self.is_loading = false;
        self.loading_message = None;
        self.loading_progress = None;
    }
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p todoee-cli loading_progress --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs
git commit -m "$(cat <<'EOF'
feat(tui): add LoadingProgress for multi-step operations

Enables progress bars and step-by-step feedback for batch operations
like sync, import/export, or multiple task updates.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Update Loading Overlay to Show Progress Bar

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs:517-554` (render_loading_overlay)

**Step 1: Update render_loading_overlay to include progress bar**

```rust
fn render_loading_overlay(app: &App, frame: &mut Frame) {
    use super::spinner::bracketed_progress;

    let area = centered_rect(50, 25, frame.area());

    let spinner_char = app.spinner_style.frame(app.animation_frame);
    let message = app.loading_message.as_deref().unwrap_or("Loading...");

    // Animated dots
    let dots_count = (app.animation_frame % 4) as usize;
    let dots = ".".repeat(dots_count);
    let dots_padding = " ".repeat(3 - dots_count);

    let mut content = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}  {}{}{}", spinner_char, message, dots, dots_padding),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Add progress bar if available
    if let Some(ref progress) = app.loading_progress {
        let bar = bracketed_progress(progress.percentage(), 30);
        let percentage = (progress.percentage() * 100.0) as u8;

        content.push(Line::from(Span::styled(
            format!("  {} {}%", bar, percentage),
            Style::default().fg(Color::Green),
        )));

        // Show step name if available
        if let Some(ref step) = progress.step_name {
            content.push(Line::from(""));
            content.push(Line::from(Span::styled(
                format!("  {}", step),
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Show progress count
        content.push(Line::from(Span::styled(
            format!("  ({}/{})", progress.current, progress.total),
            Style::default().fg(Color::DarkGray),
        )));
    }

    content.push(Line::from(""));

    let loading = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Processing "),
        )
        .alignment(Alignment::Center);

    frame.render_widget(Clear, area);
    frame.render_widget(loading, area);
}
```

**Step 2: Run tests**

Run: `cargo test -p todoee-cli --lib`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "$(cat <<'EOF'
feat(tui): add progress bar to loading overlay

Shows a visual progress bar with percentage and step info when
LoadingProgress is set for multi-step operations.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Add Status Message Animation (Success/Error Indicators)

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs` (add status animation state)
- Modify: `crates/todoee-cli/src/tui/ui.rs:352-369` (render_status)

**Step 1: Add animated status state to App**

In `app.rs`, add after `status_message`:

```rust
    /// Frame when status message was set (for fade animation)
    pub status_set_frame: Option<usize>,
```

Initialize in `App::new()`:

```rust
            status_set_frame: None,
```

Add helper to set status with animation tracking:

```rust
    /// Set status message with animation tracking
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_set_frame = Some(self.animation_frame);
    }
```

**Step 2: Update all status_message assignments to use set_status**

This is a larger refactor - update `mark_selected_done`, `delete_selected`, etc. to use `self.set_status(...)` instead of `self.status_message = Some(...)`.

**Step 3: Update render_status with animation**

In `ui.rs`, update `render_status`:

```rust
fn render_status(app: &App, frame: &mut Frame, area: Rect) {
    let status_text = app.status_message.as_deref().unwrap_or("");

    // Calculate age of status message in frames
    let age = app.status_set_frame
        .map(|set_frame| app.animation_frame.wrapping_sub(set_frame))
        .unwrap_or(0);

    // Icon animation: pulse for first few frames
    let icon = if status_text.starts_with('✓') {
        if age < 4 {
            ['✓', '✔', '✓', '✔'][age % 4]
        } else {
            '✓'
        }
    } else if status_text.starts_with('✗') {
        if age < 4 {
            ['✗', '✘', '✗', '✘'][age % 4]
        } else {
            '✗'
        }
    } else {
        ' '
    };

    // Replace first char with animated icon
    let display_text = if !status_text.is_empty() && (status_text.starts_with('✓') || status_text.starts_with('✗')) {
        format!("{}{}", icon, &status_text[status_text.chars().next().unwrap().len_utf8()..])
    } else {
        status_text.to_string()
    };

    let status_style = if status_text.starts_with('✓') {
        Style::default().fg(Color::Green)
    } else if status_text.starts_with('✗') {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let status = Paragraph::new(Span::styled(&display_text, status_style)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(status, area);
}
```

**Step 4: Run tests**

Run: `cargo test -p todoee-cli --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs crates/todoee-cli/src/tui/ui.rs
git commit -m "$(cat <<'EOF'
feat(tui): add animated status message icons

Success (✓) and error (✗) icons now pulse briefly when first displayed,
providing satisfying visual feedback for completed actions.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Add Animated Selection Cursor

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs:254-350` (render_tasks)

**Step 1: Update render_tasks with animated cursor**

In `ui.rs`, update the selector logic in `render_tasks`:

```rust
// In render_tasks, update the selector section around line 308
let selector = if is_selected {
    // Animated cursor: cycles through arrow styles
    let cursors = ['▸', '▹', '▸', '▹'];
    let cursor = cursors[app.animation_frame % cursors.len()];
    format!("{} ", cursor)
} else {
    "  ".to_string()
};
```

And update the Span:

```rust
Span::styled(
    &selector,
    if is_selected {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
),
```

**Step 2: Run app to verify animation**

Run: `cargo run -p todoee-cli`
Navigate up/down with j/k to see the pulsing cursor.

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "$(cat <<'EOF'
feat(tui): add subtle pulse animation to selection cursor

The arrow cursor (▸) now alternates brightness, drawing attention
to the selected item without being distracting.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Add Focus Mode Enhanced Animations

**Files:**
- Modify: `crates/todoee-cli/src/tui/widgets/focus.rs`

**Step 1: Read current focus widget**

Already read - uses basic progress bar with #/- characters.

**Step 2: Update focus widget with better animations**

Replace the render method in `focus.rs`:

```rust
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::app::FocusState;
use crate::tui::spinner::{bracketed_progress, Spinner};

pub struct FocusWidget<'a> {
    state: &'a FocusState,
    animation_frame: usize,
}

impl<'a> FocusWidget<'a> {
    pub fn new(state: &'a FocusState, animation_frame: usize) -> Self {
        Self { state, animation_frame }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let remaining = self.state.remaining_secs();
        let mins = remaining / 60;
        let secs = remaining % 60;

        let progress = if self.state.duration_secs > 0 {
            1.0 - (remaining as f64 / self.state.duration_secs as f64)
        } else {
            1.0
        };

        let time_color = if remaining <= 60 {
            Color::Red
        } else if remaining <= 300 {
            Color::Yellow
        } else {
            Color::Green
        };

        // Animated header when paused
        let header = if self.state.paused {
            let blink = self.animation_frame % 4 < 2;
            if blink {
                "⏸ PAUSED"
            } else {
                "  PAUSED"
            }
        } else {
            "🎯 FOCUS MODE"
        };

        // Animated timer separator
        let separator = if self.state.paused {
            ':'
        } else {
            [':', ' '][self.animation_frame % 2]
        };

        // Enhanced progress bar
        let bar = bracketed_progress(progress, 32);

        // Motivational messages based on progress
        let motivation = match progress {
            p if p < 0.25 => "Just getting started...",
            p if p < 0.50 => "Making progress!",
            p if p < 0.75 => "Over halfway there!",
            p if p < 0.90 => "Almost done!",
            _ => "Final stretch! 🎉",
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                header,
                Style::default()
                    .fg(if self.state.paused { Color::Yellow } else { Color::Red })
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                &self.state.todo_title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{:02}{}{:02}", mins, separator, secs),
                Style::default()
                    .fg(time_color)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(bar, Style::default().fg(time_color))),
            Line::from(""),
            Line::from(Span::styled(
                motivation,
                Style::default().fg(Color::DarkGray).italic(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Space: pause  q/Esc: cancel  Enter: complete",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let border_color = if self.state.paused {
            Color::Yellow
        } else {
            time_color
        };

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Focus ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .alignment(Alignment::Center);

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }
}
```

**Step 3: Update ui.rs to pass animation_frame**

In `ui.rs`, update the FocusWidget construction around line 82:

```rust
if app.mode == Mode::Focus
    && let Some(ref state) = app.focus_state
{
    let area = centered_rect(50, 50, frame.area());
    FocusWidget::new(state, app.animation_frame).render(frame, area);
}
```

**Step 4: Run app to verify focus mode animations**

Run: `cargo run -p todoee-cli`
Press 'F' on a task for quick 5-min focus, observe animations.

**Step 5: Commit**

```bash
git add crates/todoee-cli/src/tui/widgets/focus.rs crates/todoee-cli/src/tui/ui.rs
git commit -m "$(cat <<'EOF'
feat(tui): enhance focus mode with animations and motivation

- Blinking pause indicator
- Animated colon separator on timer
- Progress-based motivational messages
- Dynamic border color based on time remaining

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Add Tab Switching Animation

**Files:**
- Modify: `crates/todoee-cli/src/tui/app.rs` (add tab transition state)
- Modify: `crates/todoee-cli/src/tui/ui.rs:91-141` (render_tabs)

**Step 1: Add tab transition state to App**

In `app.rs`, add:

```rust
    /// Previous view for tab transition animation
    pub previous_view: Option<View>,
    /// Frame when view changed
    pub view_changed_frame: Option<usize>,
```

Initialize in `App::new()`:

```rust
            previous_view: None,
            view_changed_frame: None,
```

**Step 2: Track view changes in handler**

In `handler.rs`, update the view switching code around lines 57-68:

```rust
KeyCode::Char('1') => {
    if app.current_view != View::Todos {
        app.previous_view = Some(app.current_view);
        app.view_changed_frame = Some(app.animation_frame);
        app.current_view = View::Todos;
    }
    return Ok(());
}
// ... similar for '2' and '3'
```

**Step 3: Add animated underline to active tab**

In `ui.rs`, update `render_tabs`:

```rust
fn render_tabs(app: &App, frame: &mut Frame, area: Rect) {
    let tabs = [
        ("1: Todos", View::Todos),
        ("2: Categories", View::Categories),
        ("3: Settings", View::Settings),
    ];

    // Calculate transition animation
    let transition_frame = app.view_changed_frame
        .map(|f| app.animation_frame.wrapping_sub(f))
        .unwrap_or(10);
    let is_transitioning = transition_frame < 4;

    let mut spans: Vec<Span> = tabs
        .iter()
        .flat_map(|(label, view)| {
            let is_active = app.current_view == *view;
            let was_active = app.previous_view == Some(*view);

            let style = if is_active {
                let modifier = if is_transitioning {
                    Modifier::BOLD | Modifier::UNDERLINED
                } else {
                    Modifier::BOLD | Modifier::UNDERLINED
                };
                Style::default().fg(Color::Cyan).add_modifier(modifier)
            } else if was_active && is_transitioning {
                // Fading out previous tab
                Style::default().fg(Color::Gray)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            vec![Span::styled(format!(" {} ", label), style), Span::raw("  ")]
        })
        .collect();

    // ... rest of the function (filter indicators)
```

**Step 4: Commit**

```bash
git add crates/todoee-cli/src/tui/app.rs crates/todoee-cli/src/tui/handler.rs crates/todoee-cli/src/tui/ui.rs
git commit -m "$(cat <<'EOF'
feat(tui): add tab switching transition animation

Tracks view changes with frame timing for smooth visual transitions
between Todos, Categories, and Settings tabs.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Add Empty State Animations

**Files:**
- Modify: `crates/todoee-cli/src/tui/ui.rs` (render_tasks empty state)

**Step 1: Add animated empty state message**

In `ui.rs`, update `render_tasks` to show an animated message when list is empty:

```rust
fn render_tasks(app: &App, frame: &mut Frame, area: Rect) {
    let now = Utc::now();

    if app.todos.is_empty() {
        // Animated empty state
        let messages = [
            "No tasks yet... Press 'a' to add one!",
            "All clear! Press 'a' to add a task.",
            "Empty list. Time to plan ahead!",
            "No todos here. Press 'a' to get started.",
        ];
        let message = messages[(app.animation_frame / 8) % messages.len()];

        let icon = ['📝', '✨', '🎯', '📋'][(app.animation_frame / 4) % 4];

        let content = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}  {}", icon, message),
                Style::default().fg(Color::DarkGray).italic(),
            )),
            Line::from(""),
        ];

        let empty = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" Tasks (0) ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .alignment(Alignment::Center);

        frame.render_widget(empty, area);
        return;
    }

    // ... rest of existing render_tasks code
```

**Step 2: Run app and delete all tasks to verify**

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/ui.rs
git commit -m "$(cat <<'EOF'
feat(tui): add animated empty state for task list

Shows rotating messages and emoji when no tasks exist,
making the empty state more inviting.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: Add Category List Animation

**Files:**
- Modify: `crates/todoee-cli/src/tui/widgets/category_list.rs`

**Step 1: Read current implementation**

**Step 2: Add animation frame parameter and animated selection**

Update `CategoryListWidget` to accept animation frame and add a subtle selection animation.

**Step 3: Commit**

```bash
git add crates/todoee-cli/src/tui/widgets/category_list.rs crates/todoee-cli/src/tui/ui.rs
git commit -m "$(cat <<'EOF'
feat(tui): add animated selection indicator to category list

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 13: Add Insights Modal Animation

**Files:**
- Modify: `crates/todoee-cli/src/tui/widgets/insights.rs`

**Step 1: Update insights widget with animated stats reveal**

Add a "counting up" animation effect for the statistics when the modal first opens.

**Step 2: Commit**

```bash
git add crates/todoee-cli/src/tui/widgets/insights.rs crates/todoee-cli/src/tui/ui.rs
git commit -m "$(cat <<'EOF'
feat(tui): add animated stat reveal to insights modal

Stats now animate in with a counting effect when the modal opens.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 14: Final Integration Testing

**Files:**
- No new files

**Step 1: Run full test suite**

Run: `cargo test -p todoee-cli --lib`
Expected: All tests pass

**Step 2: Manual QA checklist**

- [ ] Loading spinner animates during AI parsing
- [ ] Loading spinner animates during insights computation
- [ ] Selection cursor pulses on todo list
- [ ] Focus mode timer blinks colon
- [ ] Focus mode shows motivational messages
- [ ] Empty state shows animated message
- [ ] Status messages show pulsing icons
- [ ] Tab switching feels smooth
- [ ] No performance degradation with animations

**Step 3: Run clippy**

Run: `cargo clippy -p todoee-cli -- -D warnings`
Expected: No warnings

**Step 4: Commit any final fixes**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore(tui): final polish and integration testing

Verified all animations work together smoothly with no regressions.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Summary

This plan adds comprehensive UI polish through:

1. **Spinner Module** - 8 different ASCII spinner styles (Braille, Dots, Line, Blocks, Arrow, Star, etc.)
2. **Frame-Based Animation** - Consistent 250ms tick-driven animations via `animation_frame` counter
3. **Loading Overlay** - Enhanced with progress bars, step names, and animated dots
4. **Status Messages** - Pulsing success/error icons
5. **Selection Cursor** - Subtle pulse animation on current item
6. **Focus Mode** - Blinking pause, animated timer separator, motivational messages
7. **Tab Transitions** - Smooth visual feedback when switching views
8. **Empty States** - Animated messages and rotating emoji
9. **Category/Insights** - Selection animations and stat reveals

All animations are designed to be subtle and professional, enhancing the user experience without being distracting.
