use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use todoee_core::Config;

use crate::tui::app::SettingsSection;

pub struct SettingsWidget<'a> {
    config: &'a Config,
    section: SettingsSection,
}

impl<'a> SettingsWidget<'a> {
    pub fn new(config: &'a Config, section: SettingsSection) -> Self {
        Self { config, section }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25),  // Sidebar
                Constraint::Min(40),     // Content
            ])
            .split(area);

        self.render_sidebar(frame, chunks[0]);
        self.render_content(frame, chunks[1]);
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        let sections = [
            ("AI Settings", SettingsSection::Ai),
            ("Display", SettingsSection::Display),
            ("Notifications", SettingsSection::Notifications),
            ("Database", SettingsSection::Database),
        ];

        let items: Vec<ListItem> = sections
            .iter()
            .map(|(label, sec)| {
                let is_selected = self.section == *sec;
                let style = if is_selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let prefix = if is_selected { "▸ " } else { "  " };
                ListItem::new(format!("{}{}", prefix, label)).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Sections ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
            );

        frame.render_widget(list, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect) {
        let content = match self.section {
            SettingsSection::Ai => self.render_ai_settings(),
            SettingsSection::Display => self.render_display_settings(),
            SettingsSection::Notifications => self.render_notification_settings(),
            SettingsSection::Database => self.render_database_settings(),
        };

        let title = match self.section {
            SettingsSection::Ai => " AI Configuration ",
            SettingsSection::Display => " Display Settings ",
            SettingsSection::Notifications => " Notification Settings ",
            SettingsSection::Database => " Database Settings ",
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
            );

        frame.render_widget(paragraph, area);
    }

    fn render_ai_settings(&self) -> Vec<Line<'static>> {
        let model_status = self.config.ai.model.as_ref()
            .map(|m| format!("✓ {}", m))
            .unwrap_or_else(|| "✗ Not configured".to_string());

        let api_key_status = std::env::var(&self.config.ai.api_key_env)
            .map(|_| "✓ Set".to_string())
            .unwrap_or_else(|_| "✗ Not set".to_string());

        vec![
            Line::from(vec![
                Span::styled("Provider: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(self.config.ai.provider.clone()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Model: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(model_status, Style::default().fg(
                    if self.config.ai.model.is_some() { Color::Green } else { Color::Red }
                )),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("API Key Env: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(self.config.ai.api_key_env.clone()),
            ]),
            Line::from(vec![
                Span::styled("  Status: ", Style::default().fg(Color::DarkGray)),
                Span::styled(api_key_status, Style::default().fg(
                    if std::env::var(&self.config.ai.api_key_env).is_ok() { Color::Green } else { Color::Red }
                )),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Edit ~/.config/todoee/config.toml to configure",
                Style::default().fg(Color::DarkGray)
            )),
        ]
    }

    fn render_display_settings(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(vec![
                Span::styled("Theme: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(self.config.display.theme.clone()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Date Format: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(self.config.display.date_format.clone()),
            ]),
        ]
    }

    fn render_notification_settings(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(vec![
                Span::styled("Enabled: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if self.config.notifications.enabled { "Yes" } else { "No" },
                    Style::default().fg(if self.config.notifications.enabled { Color::Green } else { Color::Red })
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Sound: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if self.config.notifications.sound { "Yes" } else { "No" },
                    Style::default().fg(if self.config.notifications.sound { Color::Green } else { Color::Red })
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Advance Notice: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{} minutes", self.config.notifications.advance_minutes)),
            ]),
        ]
    }

    fn render_database_settings(&self) -> Vec<Line<'static>> {
        let neon_status = std::env::var(&self.config.database.url_env)
            .map(|_| "✓ Configured".to_string())
            .unwrap_or_else(|_| "✗ Not configured (local only)".to_string());

        let local_db = self.config.local_db_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "Error loading path".to_string());

        vec![
            Line::from(vec![
                Span::styled("Cloud Sync Env: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(self.config.database.url_env.clone()),
            ]),
            Line::from(vec![
                Span::styled("  Status: ", Style::default().fg(Color::DarkGray)),
                Span::styled(neon_status, Style::default().fg(
                    if std::env::var(&self.config.database.url_env).is_ok() { Color::Green } else { Color::Yellow }
                )),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Local Database: ", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled(local_db, Style::default().fg(Color::DarkGray))),
        ]
    }
}
