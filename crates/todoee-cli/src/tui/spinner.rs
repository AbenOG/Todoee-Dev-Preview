//! Spinner animations and progress bar utilities for TUI loading states.

/// ASCII spinner animation styles for loading indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum Spinner {
    /// Braille dots animation (default): `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`
    #[default]
    Braille,
    /// Dense dots animation: `⣾⣽⣻⢿⡿⣟⣯⣷`
    Dots,
    /// Classic line spinner: `|/-\`
    Line,
    /// Block corners animation: `▖▘▝▗`
    Blocks,
    /// Growing dots animation: `. . .   `
    GrowingDots,
    /// Arrow rotation: `←↖↑↗→↘↓↙`
    Arrow,
    /// Box bounce animation: `▌▀▐▄`
    BoxBounce,
    /// Star pulse animation: `✶✷✸✹✺✹✸✷`
    Star,
}

#[allow(dead_code)]
impl Spinner {
    const BRAILLE_FRAMES: &'static [char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    const DOTS_FRAMES: &'static [char] = &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];
    const LINE_FRAMES: &'static [char] = &['|', '/', '-', '\\'];
    const BLOCKS_FRAMES: &'static [char] = &['▖', '▘', '▝', '▗'];
    const GROWING_DOTS_FRAMES: &'static [char] = &['.', ' ', '.', ' ', '.', ' ', ' ', ' '];
    const ARROW_FRAMES: &'static [char] = &['←', '↖', '↑', '↗', '→', '↘', '↓', '↙'];
    const BOX_BOUNCE_FRAMES: &'static [char] = &['▌', '▀', '▐', '▄'];
    const STAR_FRAMES: &'static [char] = &['✶', '✷', '✸', '✹', '✺', '✹', '✸', '✷'];

    /// Returns the frame characters for this spinner style.
    pub fn frames(&self) -> &'static [char] {
        match self {
            Spinner::Braille => Self::BRAILLE_FRAMES,
            Spinner::Dots => Self::DOTS_FRAMES,
            Spinner::Line => Self::LINE_FRAMES,
            Spinner::Blocks => Self::BLOCKS_FRAMES,
            Spinner::GrowingDots => Self::GROWING_DOTS_FRAMES,
            Spinner::Arrow => Self::ARROW_FRAMES,
            Spinner::BoxBounce => Self::BOX_BOUNCE_FRAMES,
            Spinner::Star => Self::STAR_FRAMES,
        }
    }

    /// Returns the character at the given index, wrapping around if necessary.
    pub fn frame(&self, index: usize) -> char {
        let frames = self.frames();
        frames[index % frames.len()]
    }

    /// Returns the number of frames in this spinner animation.
    pub fn len(&self) -> usize {
        self.frames().len()
    }

    /// Returns true if the spinner has no frames (always false for valid spinners).
    pub fn is_empty(&self) -> bool {
        self.frames().is_empty()
    }
}

/// Creates a progress bar string with custom fill and empty characters.
///
/// # Arguments
/// * `progress` - Progress value from 0.0 to 1.0 (clamped to this range)
/// * `width` - Total width of the progress bar in characters
/// * `filled` - Character to use for filled portion
/// * `empty` - Character to use for empty portion
#[allow(dead_code)]
pub fn progress_bar(progress: f64, width: usize, filled: char, empty: char) -> String {
    let progress = progress.clamp(0.0, 1.0);
    let filled_count = (progress * width as f64).round() as usize;
    let empty_count = width.saturating_sub(filled_count);

    let mut result = String::with_capacity(width);
    for _ in 0..filled_count {
        result.push(filled);
    }
    for _ in 0..empty_count {
        result.push(empty);
    }
    result
}

/// Creates a bracketed progress bar in the format `[████░░░░]`.
///
/// # Arguments
/// * `progress` - Progress value from 0.0 to 1.0 (clamped to this range)
/// * `width` - Width of the progress bar content (excluding brackets)
#[allow(dead_code)]
pub fn bracketed_progress(progress: f64, width: usize) -> String {
    format!("[{}]", progress_bar(progress, width, '█', '░'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braille_has_10_frames() {
        let spinner = Spinner::Braille;
        assert_eq!(spinner.len(), 10);
    }

    #[test]
    fn braille_wraps_correctly() {
        let spinner = Spinner::Braille;
        assert_eq!(spinner.frame(0), '⠋');
        assert_eq!(spinner.frame(9), '⠏');
        assert_eq!(spinner.frame(10), '⠋'); // Wraps back to first frame
        assert_eq!(spinner.frame(11), '⠙'); // Second frame after wrap
    }

    #[test]
    fn dots_works() {
        let spinner = Spinner::Dots;
        assert_eq!(spinner.len(), 8);
        assert_eq!(spinner.frame(0), '⣾');
        assert_eq!(spinner.frame(7), '⣷');
        assert_eq!(spinner.frame(8), '⣾'); // Wraps
    }

    #[test]
    fn line_has_4_frames_starting_with_pipe() {
        let spinner = Spinner::Line;
        assert_eq!(spinner.len(), 4);
        assert_eq!(spinner.frame(0), '|');
        assert_eq!(spinner.frame(1), '/');
        assert_eq!(spinner.frame(2), '-');
        assert_eq!(spinner.frame(3), '\\');
    }

    #[test]
    fn progress_bar_at_zero_percent() {
        let bar = progress_bar(0.0, 8, '█', '░');
        assert_eq!(bar, "░░░░░░░░");
    }

    #[test]
    fn progress_bar_at_fifty_percent() {
        let bar = progress_bar(0.5, 8, '█', '░');
        assert_eq!(bar, "████░░░░");
    }

    #[test]
    fn progress_bar_at_hundred_percent() {
        let bar = progress_bar(1.0, 8, '█', '░');
        assert_eq!(bar, "████████");
    }

    #[test]
    fn bracketed_progress_format() {
        let bar = bracketed_progress(0.5, 8);
        assert_eq!(bar, "[████░░░░]");
    }

    #[test]
    fn progress_bar_clamps_values() {
        let below_zero = progress_bar(-0.5, 4, '█', '░');
        assert_eq!(below_zero, "░░░░");

        let above_one = progress_bar(1.5, 4, '█', '░');
        assert_eq!(above_one, "████");
    }

    #[test]
    fn default_spinner_is_braille() {
        let spinner = Spinner::default();
        assert_eq!(spinner, Spinner::Braille);
    }

    #[test]
    fn all_spinners_are_not_empty() {
        let spinners = [
            Spinner::Braille,
            Spinner::Dots,
            Spinner::Line,
            Spinner::Blocks,
            Spinner::GrowingDots,
            Spinner::Arrow,
            Spinner::BoxBounce,
            Spinner::Star,
        ];

        for spinner in spinners {
            assert!(!spinner.is_empty(), "{:?} should not be empty", spinner);
        }
    }
}
