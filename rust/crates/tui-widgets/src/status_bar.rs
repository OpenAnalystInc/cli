//! Status bar widget — persistent line between chat and input.
//!
//! Shows: spinner + phase label + elapsed time + token count.
//! Example: `* Thinking... (4m 55s · ↓ 5.0k tokens)`

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
    fn label(&self) -> &str {
        match self {
            Self::Idle => "Ready",
            Self::Thinking => "Thinking...",
            Self::ReadingFile(_) => "Reading file...",
            Self::EditingFile(_) => "Editing...",
            Self::RunningBash => "Running bash...",
            Self::Searching => "Searching...",
            Self::Done => "Done",
            Self::Error => "Error",
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
}

impl Default for StatusBar {
    fn default() -> Self {
        Self {
            phase: AgentPhase::Idle,
            elapsed: Duration::ZERO,
            total_tokens: 0,
            model_name: String::new(),
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
        let phase_color = self.phase.color();
        let icon = self.phase.icon();
        let label = self.phase.label();
        let time_str = Self::format_duration(&self.elapsed);
        let token_str = Self::format_tokens(self.total_tokens);

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
        ]);

        Paragraph::new(line).render(area, buf);
    }
}
