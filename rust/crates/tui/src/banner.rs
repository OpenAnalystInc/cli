//! Startup banner widget — branded OpenAnalyst banner matching Claude Code style.
//!
//! Blue branded box with rounded corners, OA logo, tips, and account info.
//! Spans the full chat width like Claude Code's orange crab banner.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Account info for the banner display.
#[derive(Debug, Clone, Default)]
pub struct BannerAccountInfo {
    pub display_name: String,
    pub model_display: String,
    pub provider_name: String,
    pub user_email: Option<String>,
    pub credits: Option<String>,
    pub organization: Option<String>,
    pub cwd: String,
    pub version: String,
}

impl BannerAccountInfo {
    /// Whether the current provider is OpenAnalyst (show OA branding).
    pub fn is_openanalyst(&self) -> bool {
        self.provider_name == "OpenAnalyst Inc"
    }

    /// App title shown in the banner header — dynamic based on provider.
    pub fn app_title(&self) -> String {
        if self.is_openanalyst() {
            format!("OpenAnalyst CLI v{}", self.version)
        } else {
            format!("OpenAnalyst CLI v{} · {}", self.version, self.provider_name)
        }
    }
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

    /// Generate the banner as styled Lines for the chat scroll buffer.
    ///
    /// Design matches Claude Code's banner style:
    /// - Colored header line with version
    /// - Rounded-corner box with two columns
    /// - Left: welcome + OA logo + account info
    /// - Right: tips + recent activity
    /// - Blue brand color (OpenAnalyst) instead of Claude's orange
    ///
    /// `available_width` is the chat panel width (0 = use default).
    #[must_use]
    pub fn to_lines_with_width(&self, available_width: usize) -> Vec<Line<'static>> {
        // Brand colors
        let brand = Style::default().fg(Color::Rgb(50, 130, 255)).add_modifier(Modifier::BOLD);
        let brand_bold = brand;
        let accent = Style::default().fg(Color::Rgb(80, 160, 255));    // lighter blue (RGB)
        let white_bold = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
        let white = Style::default().fg(Color::White);
        let dim = Style::default().fg(Color::Indexed(245));
        let green = Style::default().fg(Color::Indexed(40));
        let logo_color = Style::default().fg(Color::Rgb(255, 140, 0)); // orange OA logo

        // Column widths — adapt to available terminal width
        let title_text = self.info.app_title();
        let title_len = title_text.chars().count() + 3; // " title " + leading "─"
        let right_w: usize = 38;
        // Use available width minus borders (3 for ╭│╮) and right column
        let left_w: usize = if available_width > right_w + 10 {
            (available_width - right_w - 3).max(title_len).max(40)
        } else {
            title_len.max(40)
        };
        let _total_inner = left_w + 1 + right_w; // +1 for middle │

        let mut lines = Vec::new();

        // ── Top border with version inline (like Claude Code) ──
        // ╭─ OpenAnalyst CLI v1.0.98 · Anthropic ──┬──────────────────╮
        let ver_text = format!(" {} ", self.info.app_title());
        let ver_len = ver_text.chars().count();
        let left_pad = left_w.saturating_sub(ver_len + 1); // +1 for leading ─
        let right_border = "─".repeat(right_w);
        lines.push(Line::from(vec![
            Span::styled("╭─", brand),
            Span::styled(ver_text, brand_bold),
            Span::styled(format!("{}┬{}╮", "─".repeat(left_pad), right_border), brand),
        ]));

        // Helper: build a branded dual-column row
        let brow = |left: &str, ls: Style, right: &str, rs: Style| -> Line<'static> {
            let lp = left_w.saturating_sub(left.chars().count());
            let rp = right_w.saturating_sub(right.chars().count());
            Line::from(vec![
                Span::styled("│", brand),
                Span::styled(left.to_string(), ls),
                Span::raw(" ".repeat(lp)),
                Span::styled("│", brand),
                Span::styled(right.to_string(), rs),
                Span::raw(" ".repeat(rp)),
                Span::styled("│", brand),
            ])
        };

        // ── Row: Welcome | Tips header ──
        let welcome_text = format!("  Welcome back, {}!", self.info.display_name);
        lines.push(brow(&welcome_text, white_bold, " Tips for getting started", green));

        // ── Blank spacer ──
        lines.push(brow("", dim, "", dim));

        // ── OA block logo in orange (5 rows) ──
        let logo: [&str; 5] = [
            "   ████████   ████         ",
            "   ██    ██  ██  ██        ",
            "   ██    ██  ██████        ",
            "   ██    ██  ██  ██        ",
            "   ████████  ██  ██        ",
        ];

        let tip_lines: [(&str, Style); 5] = if self.info.is_openanalyst() {
            [
                (" Run /init to create an", dim),
                (" OPENANALYST.md file with", dim),
                (" instructions for OpenAnalyst", dim),
                (" Recent activity", green),
                (" No recent activity", dim),
            ]
        } else {
            [
                (" Run /init to create a", dim),
                (" project config file with", dim),
                (" instructions for the agent", dim),
                (" Recent activity", green),
                (" No recent activity", dim),
            ]
        };

        for (logo_line, (tip, tip_style)) in logo.iter().zip(tip_lines.iter()) {
            let lp = left_w.saturating_sub(logo_line.chars().count());
            let rp = right_w.saturating_sub(tip.chars().count());
            lines.push(Line::from(vec![
                Span::styled("│", brand),
                Span::styled(logo_line.to_string(), logo_color),
                Span::raw(" ".repeat(lp)),
                Span::styled("│", brand),
                Span::styled(tip.to_string(), *tip_style),
                Span::raw(" ".repeat(rp)),
                Span::styled("│", brand),
            ]));
        }

        // ── Blank separator ──
        lines.push(brow("", dim, "", dim));

        // ── Model + provider (left-aligned) ──
        let model_line = format!(
            " {} · {}",
            self.info.model_display, self.info.provider_name
        );
        lines.push(brow(&model_line, white, "", dim));

        // ── Email + org (left-aligned) ──
        if let Some(ref email) = self.info.user_email {
            let mut info = format!(" {email}");
            if let Some(ref org) = self.info.organization {
                info = format!("{info} · {org}'s Organization");
            }
            // Truncate if too long
            if info.chars().count() > left_w {
                let t: String = info.chars().take(left_w - 3).collect();
                info = format!("{t}...");
            }
            lines.push(brow(&info, dim, "", dim));
        }

        // ── Credits balance (left-aligned) ──
        if let Some(ref credits) = self.info.credits {
            let credits_line = format!(" Credits: {credits}");
            let credits_style = Style::default().fg(Color::Indexed(40)); // green
            lines.push(brow(&credits_line, credits_style, "", dim));
        }

        // ── CWD (left-aligned) ──
        let cwd_display = if self.info.cwd.chars().count() > left_w - 4 {
            let keep = left_w - 5;
            let start = self.info.cwd.chars().count() - keep;
            let truncated: String = self.info.cwd.chars().skip(start).collect();
            format!(" …{truncated}")
        } else {
            format!(" {}", self.info.cwd)
        };
        lines.push(brow(&cwd_display, dim, "", dim));

        // ── Bottom border with rounded corners ──
        lines.push(Line::from(vec![
            Span::styled(
                format!("╰{}┴{}╯", "─".repeat(left_w), "─".repeat(right_w)),
                brand,
            ),
        ]));

        lines.push(Line::from(""));

        // ── Hint line ──
        lines.push(Line::from(vec![
            Span::styled("  /help", accent),
            Span::styled(" for commands · ", dim),
            Span::styled("/model", accent),
            Span::styled(" to switch · ", dim),
            Span::styled("ctrl+c", accent),
            Span::styled(" to exit", dim),
        ]));

        lines.push(Line::from(""));

        lines
    }

    /// Generate banner lines with default width.
    #[must_use]
    pub fn to_lines(&self) -> Vec<Line<'static>> {
        self.to_lines_with_width(0)
    }
}
