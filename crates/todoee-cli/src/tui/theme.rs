use ratatui::style::{Color, Modifier, Style};

/// Application theme colors
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub muted: Color,
    pub border: Color,
    pub border_focused: Color,
    pub selection_bg: Color,
    pub text: Color,
    pub text_muted: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            muted: Color::DarkGray,
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            selection_bg: Color::Rgb(40, 40, 50),
            text: Color::White,
            text_muted: Color::Gray,
        }
    }
}

impl Theme {
    pub fn border_style(&self, focused: bool) -> Style {
        Style::default().fg(if focused { self.border_focused } else { self.border })
    }

    pub fn title_style(&self) -> Style {
        Style::default().fg(self.primary).add_modifier(Modifier::BOLD)
    }

    pub fn selected_style(&self) -> Style {
        Style::default().bg(self.selection_bg)
    }

    pub fn priority_high(&self) -> Style {
        Style::default().fg(self.error).add_modifier(Modifier::BOLD)
    }

    pub fn priority_medium(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn priority_low(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn completed_style(&self) -> Style {
        Style::default()
            .fg(self.muted)
            .add_modifier(Modifier::CROSSED_OUT)
    }
}
