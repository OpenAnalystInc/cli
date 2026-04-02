//! Startup banner widget — OA block letters with account info.

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
        let accent = Style::default().fg(Color::Indexed(208)); // Orange
        let blue = Style::default().fg(Color::Indexed(39));
        let cyan = Style::default().fg(Color::Indexed(45));
        let white = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
        let text = Style::default().fg(Color::Indexed(252));
        let dim = Style::default().fg(Color::DarkGray);

        let mut lines = Vec::new();
        lines.push(Line::from(""));

        // ── OA block letters (orange gradient) ──
        let logo: &[(&str, &str)] = &[
            (" ████████  ", "  ████   "),
            (" ██    ██  ", " ██  ██  "),
            (" ██    ██  ", "██    ██ "),
            (" ██    ██  ", "████████ "),
            (" ██    ██  ", "██    ██ "),
            (" ████████  ", "██    ██ "),
        ];

        for (o_part, a_part) in logo {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(*o_part, accent),
                Span::styled(*a_part, accent),
            ]));
        }

        lines.push(Line::from(""));

        // ── Title line ──
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("OpenAnalyst CLI", white),
            Span::styled(format!("  v{}", self.info.version), dim),
        ]));

        // ── Welcome ──
        if !self.info.display_name.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("Welcome back, {}", self.info.display_name),
                    cyan,
                ),
            ]));
        }

        lines.push(Line::from(""));

        // ── Model + provider ──
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(self.info.model_display.clone(), blue),
            Span::styled(" · ", dim),
            Span::styled(self.info.provider_name.clone(), text),
        ]));

        // ── Working directory ──
        lines.push(Line::from(Span::styled(
            format!("  {}", self.info.cwd),
            dim,
        )));

        // ── User info (email, org) ──
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

        lines.push(Line::from(""));

        // ── Hints ──
        lines.push(Line::from(Span::styled(
            "  /help for commands · /model to switch · ctrl+c to exit",
            dim,
        )));

        lines.push(Line::from(""));

        lines
    }
}
