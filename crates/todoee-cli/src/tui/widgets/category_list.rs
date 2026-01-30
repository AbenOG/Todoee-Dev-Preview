use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};
use todoee_core::Category;

pub struct CategoryListWidget<'a> {
    categories: &'a [Category],
    selected: usize,
}

impl<'a> CategoryListWidget<'a> {
    pub fn new(categories: &'a [Category], selected: usize) -> Self {
        Self {
            categories,
            selected,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected;
                let selector = if is_selected { "▸ " } else { "  " };

                let color = cat
                    .color
                    .as_ref()
                    .and_then(|c| parse_hex_color(c))
                    .unwrap_or(Color::White);

                let ai_badge = if cat.is_ai_generated {
                    Span::styled(" [AI]", Style::default().fg(Color::Magenta))
                } else {
                    Span::raw("")
                };

                let content = Line::from(vec![
                    Span::styled(selector, Style::default().fg(Color::Cyan)),
                    Span::styled("● ", Style::default().fg(color)),
                    Span::raw(&cat.name),
                    ai_badge,
                ]);

                let style = if is_selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(format!(" Categories ({}) ", self.categories.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

        frame.render_widget(list, area);
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
