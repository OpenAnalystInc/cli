//! Status bar widget — persistent line between chat and input.
//!
//! Shows: spinner + phase label + elapsed time + token count on the left,
//! keybinding hints on the right.
//! Example: `* Thinking... (4m 55s · ↓ 5.0k tokens)          Esc:scroll · Ctrl+C:quit`

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

/// Current phase of the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentPhase {
    Idle,
    Thinking,
    ReadingFile(String),
    EditingFile(String),
    RunningBash,
    Searching,
    Done,
    Error,
}

impl AgentPhase {
    fn label(&self) -> String {
        match self {
            Self::Idle => "Ready".to_string(),
            Self::Thinking => "Thinking...".to_string(),
            Self::ReadingFile(f) => {
                let name = f.rsplit(['/', '\\']).next().unwrap_or(f);
                format!("Reading {name}")
            }
            Self::EditingFile(f) => {
                let name = f.rsplit(['/', '\\']).next().unwrap_or(f);
                format!("Editing {name}")
            }
            Self::RunningBash => "Running bash...".to_string(),
            Self::Searching => "Searching...".to_string(),
            Self::Done => "Done".to_string(),
            Self::Error => "Error".to_string(),
        }
    }

    fn color(&self) -> Color {
        match self {
            Self::Idle | Self::Done => Color::Green,
            Self::Error => Color::Red,
            _ => Color::Blue,
        }
    }

    fn icon(&self) -> &str {
        match self {
            Self::Idle => "✓",
            Self::Done => "✓",
            Self::Error => "✗",
            _ => "*",
        }
    }
}

/// Persistent status bar between chat area and input prompt.
#[derive(Debug, Clone)]
pub struct StatusBar {
    pub phase: AgentPhase,
    pub elapsed: Duration,
    pub total_tokens: u64,
    pub model_name: String,
    /// Right-aligned keybinding hints (e.g., "Esc:scroll · Ctrl+C:quit").
    pub hints: String,
    /// Animated spinner color (cycles through brand colors).
    pub spinner_color: Option<Color>,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self {
            phase: AgentPhase::Idle,
            elapsed: Duration::ZERO,
            total_tokens: 0,
            model_name: String::new(),
            hints: String::new(),
            spinner_color: None,
        }
    }
}

impl StatusBar {
    fn format_duration(duration: &Duration) -> String {
        let total_secs = duration.as_secs();
        if total_secs < 60 {
            format!("{total_secs}s")
        } else {
            let minutes = total_secs / 60;
            let seconds = total_secs % 60;
            format!("{minutes}m {seconds:02}s")
        }
    }

    fn format_tokens(tokens: u64) -> String {
        if tokens < 1_000 {
            format!("{tokens}")
        } else if tokens < 1_000_000 {
            format!("{:.1}k", tokens as f64 / 1_000.0)
        } else {
            format!("{:.1}M", tokens as f64 / 1_000_000.0)
        }
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Use animated spinner color when active, otherwise use phase color
        let phase_color = match (&self.phase, self.spinner_color) {
            (AgentPhase::Thinking | AgentPhase::ReadingFile(_) | AgentPhase::EditingFile(_)
            | AgentPhase::RunningBash | AgentPhase::Searching, Some(c)) => c,
            _ => self.phase.color(),
        };
        let icon = self.phase.icon();
        let label = self.phase.label();
        let time_str = Self::format_duration(&self.elapsed);
        let token_str = Self::format_tokens(self.total_tokens);

        // Build left side
        let left_text = format!("{icon} {label} ({time_str} · ↓ {token_str} tokens)");
        let left_len = left_text.chars().count();

        // Build right side (hints)
        let hints_len = self.hints.chars().count();

        // Calculate padding
        let total_width = area.width as usize;
        let pad = total_width.saturating_sub(left_len + hints_len + 1);

        let line = Line::from(vec![
            Span::styled(
                format!("{icon} {label}"),
                Style::default()
                    .fg(phase_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({time_str} · ↓ {token_str} tokens)"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" ".repeat(pad)),
            Span::styled(
                self.hints,
                Style::default().fg(Color::Indexed(240)),
            ),
        ]);

        Paragraph::new(line).render(area, buf);
    }
}
