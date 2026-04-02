//! Startup banner widget — dual-column box with OA block letters.

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
        let cyan = Style::default().fg(Color::Indexed(45));
        let blue = Style::default().fg(Color::Indexed(39));
        let white = Style::default().fg(Color::White);
        let bold_white = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
        let dim = Style::default().fg(Color::DarkGray);
        let orange = Style::default().fg(Color::Indexed(208));

        let mut lines = Vec::new();

        // ── Header line ──
        let header = format!("─── OpenAnalyst CLI v{} ", self.info.version);
        let pad_len = 72usize.saturating_sub(header.len());
        lines.push(Line::from(Span::styled(
            format!("{header}{}", "─".repeat(pad_len)),
            cyan,
        )));

        // ── Top border ──
        //  ┌─────────────────────────────────┬──────────────────────────────────┐
        lines.push(Line::from(Span::styled(
            "  ┌─────────────────────────────────┬──────────────────────────────────┐",
            dim,
        )));

        // ── Row 1: Welcome | Tips header ──
        let welcome = format!("  Welcome back, {}!", self.info.display_name);
        let welcome_pad = 34usize.saturating_sub(welcome.len());
        lines.push(Line::from(vec![
            Span::styled("  │", dim),
            Span::styled(welcome, bold_white),
            Span::raw(" ".repeat(welcome_pad)),
            Span::styled("│", dim),
            Span::styled(" Tips for getting started", white),
            Span::styled("          │", dim),
        ]));

        // ── Rows 2-6: OA logo | Tips content ──
        let logo: [&str; 6] = [
            " ████████    ████   ",
            " ██    ██   ██  ██  ",
            " ██    ██  ██    ██ ",
            " ██    ██  ████████ ",
            " ██    ██  ██    ██ ",
            " ████████  ██    ██ ",
        ];

        let tips: [&str; 6] = [
            " Run /init to create an          ",
            " OPENANALYST.md file with         ",
            " instructions for OpenAnalyst     ",
            "──────────────────────────────────",
            " Recent activity                  ",
            " No recent activity               ",
        ];

        for (i, (logo_line, tip_line)) in logo.iter().zip(tips.iter()).enumerate() {
            let logo_pad = 34usize.saturating_sub(logo_line.len() + 1);
            let is_separator = tip_line.starts_with('─');

            let mut spans = vec![
                Span::styled("  │", dim),
                Span::raw(" "),
                Span::styled(*logo_line, orange),
                Span::raw(" ".repeat(logo_pad)),
                Span::styled("│", dim),
            ];

            if is_separator {
                spans.push(Span::styled(*tip_line, dim));
                spans.push(Span::styled("│", dim));
            } else {
                spans.push(Span::styled(*tip_line, dim));
                spans.push(Span::styled("│", dim));
            }

            lines.push(Line::from(spans));

            // After last logo line, add blank
            if i == 5 {
                lines.push(Line::from(vec![
                    Span::styled("  │", dim),
                    Span::raw(" ".repeat(34)),
                    Span::styled("│", dim),
                    Span::raw(" ".repeat(34)),
                    Span::styled("│", dim),
                ]));
            }
        }

        // ── Model/provider row ──
        let model_line = format!("  {} · {}", self.info.model_display, self.info.provider_name);
        let model_pad = 34usize.saturating_sub(model_line.len());
        lines.push(Line::from(vec![
            Span::styled("  │", dim),
            Span::styled(model_line, white),
            Span::raw(" ".repeat(model_pad)),
            Span::styled("│", dim),
            Span::raw(" ".repeat(34)),
            Span::styled("│", dim),
        ]));

        // ── User info row ──
        let mut user_line = String::new();
        if let Some(ref email) = self.info.user_email {
            user_line.push_str(&format!("  {email}"));
            if let Some(ref org) = self.info.organization {
                user_line.push_str(&format!(" · {org}"));
            }
        }
        if !user_line.is_empty() {
            let user_pad = 34usize.saturating_sub(user_line.len());
            lines.push(Line::from(vec![
                Span::styled("  │", dim),
                Span::styled(user_line, dim),
                Span::raw(" ".repeat(user_pad)),
                Span::styled("│", dim),
                Span::raw(" ".repeat(34)),
                Span::styled("│", dim),
            ]));
        }

        // ── CWD row ──
        let cwd_display = if self.info.cwd.len() > 32 {
            format!("  …{}", &self.info.cwd[self.info.cwd.len()-30..])
        } else {
            format!("  {}", self.info.cwd)
        };
        let cwd_pad = 34usize.saturating_sub(cwd_display.len());
        lines.push(Line::from(vec![
            Span::styled("  │", dim),
            Span::styled(cwd_display, dim),
            Span::raw(" ".repeat(cwd_pad)),
            Span::styled("│", dim),
            Span::raw(" ".repeat(34)),
            Span::styled("│", dim),
        ]));

        // ── Bottom border ──
        lines.push(Line::from(Span::styled(
            "  └─────────────────────────────────┴──────────────────────────────────┘",
            dim,
        )));

        lines.push(Line::from(""));

        // ── Hint ──
        lines.push(Line::from(Span::styled(
            "  /help for commands · /model to switch · ctrl+c to exit",
            dim,
        )));

        lines.push(Line::from(""));

        lines
    }
}
