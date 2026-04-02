//! Color theme for the TUI, matching the existing CLI color palette.

use ratatui::style::{Color, Modifier, Style};

/// TUI color theme derived from the existing CLI `ColorTheme`.
pub struct TuiTheme {
    pub heading: Style,
    pub emphasis: Style,
    pub strong: Style,
    pub inline_code: Style,
    pub link: Style,
    pub border: Style,
    pub border_dim: Style,
    pub text: Style,
    pub text_dim: Style,
    pub accent: Style,
    pub spinner_active: Style,
    pub spinner_done: Style,
    pub spinner_failed: Style,
    pub user_prompt: Style,
}

impl Default for TuiTheme {
    fn default() -> Self {
        Self {
            heading: Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            emphasis: Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC),
            strong: Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            inline_code: Style::default().fg(Color::Green),
            link: Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
            border: Style::default().fg(Color::Indexed(39)),  // blue
            border_dim: Style::default().fg(Color::Indexed(240)),
            text: Style::default().fg(Color::Indexed(252)),
            text_dim: Style::default().fg(Color::DarkGray),
            accent: Style::default().fg(Color::Indexed(45)),  // cyan
            spinner_active: Style::default().fg(Color::Blue),
            spinner_done: Style::default().fg(Color::Green),
            spinner_failed: Style::default().fg(Color::Red),
            user_prompt: Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        }
    }
}
