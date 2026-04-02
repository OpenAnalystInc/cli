//! Startup banner widget — Claude Code-style dual-column box with OA block letters.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Account info for the banner display.
#[derive(Debug, Clone, Default)]
pub struct BannerAccountInfo {
    pub display_name: String,
    pub model_display: String,
    pub provider_name: String,
    pub user_email: Option<String>,
    pub organization: Option<String>,
    pub cwd: String,
    pub version: String,
}

/// The startup banner widget.
pub struct Banner {
    pub info: BannerAccountInfo,
}

impl Banner {
    #[must_use]
    pub fn new(info: BannerAccountInfo) -> Self {
        Self { info }
    }

    /// Get the banner as a Vec of Lines for embedding in the chat scroll buffer.
    #[must_use]
    pub fn to_lines(&self) -> Vec<Line<'static>> {
        let blue = Style::default().fg(Color::Indexed(39));
        let cyan = Style::default().fg(Color::Indexed(45));
        let text = Style::default().fg(Color::Indexed(252));
        let dim = Style::default().fg(Color::DarkGray);
        let _bold = Style::default().add_modifier(Modifier::BOLD);

        let mut lines = Vec::new();

        // Header line
        let header = format!("─── OpenAnalyst CLI v{} ", self.info.version);
        let pad_len = 80usize.saturating_sub(header.len());
        lines.push(Line::from(Span::styled(
            format!("{header}{}", "─".repeat(pad_len)),
            cyan,
        )));

        // OA block letters
        let mascot = [
            ("  ██████  ", " █████ "),
            (" ██    ██ ", "██   ██"),
            (" ██    ██ ", "███████"),
            (" ██    ██ ", "██   ██"),
            ("  ██████  ", "██   ██"),
        ];

        // Welcome line
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Welcome back, {}!", self.info.display_name),
            text.add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Mascot
        for (left, right) in &mascot {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(*left, blue),
                Span::styled(*right, cyan),
            ]));
        }

        lines.push(Line::from(""));

        // Model + provider
        lines.push(Line::from(Span::styled(
            format!("  {} · {}", self.info.model_display, self.info.provider_name),
            text,
        )));

        // User info
        let mut user_parts = Vec::new();
        if let Some(ref email) = self.info.user_email {
            user_parts.push(email.clone());
        }
        if let Some(ref org) = self.info.organization {
            user_parts.push(format!("{org}'s Organization"));
        }
        if !user_parts.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  {}", user_parts.join(" · ")),
                dim,
            )));
        }

        // Working directory
        lines.push(Line::from(Span::styled(
            format!("  {}", self.info.cwd),
            dim,
        )));

        lines.push(Line::from(""));

        // Hint
        lines.push(Line::from(Span::styled(
            "  (ctrl+b to run in background)",
            dim,
        )));

        lines.push(Line::from(""));

        lines
    }
}
