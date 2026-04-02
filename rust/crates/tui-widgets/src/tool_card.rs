//! Tool call card widget — displays tool invocations inline in the chat.
//!
//! Custom widget (no existing crate). Renders as a bordered box with:
//! - Tool name in the title
//! - Input preview
//! - Status indicator (spinner while running, checkmark/x when done)
//! - Collapsible output section

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget, Wrap};

/// Status of a tool call execution.
#[derive(Debug, Clone)]
pub enum ToolCallStatus {
    /// Tool is currently running.
    Running { elapsed: Duration },
    /// Tool completed successfully.
    Completed { duration: Duration },
    /// Tool failed with an error.
    Failed { duration: Duration },
}

impl ToolCallStatus {
    fn icon(&self) -> Span<'static> {
        match self {
            Self::Running { .. } => Span::styled("⠋ ", Style::default().fg(Color::Blue)),
            Self::Completed { .. } => Span::styled("✓ ", Style::default().fg(Color::Green)),
            Self::Failed { .. } => Span::styled("✗ ", Style::default().fg(Color::Red)),
        }
    }

    pub fn duration_label(&self) -> String {
        let duration = match self {
            Self::Running { elapsed } | Self::Completed { duration: elapsed } | Self::Failed { duration: elapsed } => elapsed,
        };
        let ms = duration.as_millis();
        if ms < 1000 {
            format!("{ms}ms")
        } else {
            format!("{:.1}s", duration.as_secs_f64())
        }
    }
}

/// A tool call card rendered inline in the chat.
#[derive(Debug, Clone)]
pub struct ToolCallCard {
    /// Tool name (e.g., "Read", "Edit", "Bash").
    pub tool_name: String,
    /// Preview of the tool input.
    pub input_preview: String,
    /// Current execution status.
    pub status: ToolCallStatus,
    /// Tool output (populated after completion).
    pub output: Option<String>,
    /// Whether the output section is expanded.
    pub expanded: bool,
}

impl ToolCallCard {
    /// Toggle the expanded/collapsed state of the output.
    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }
}

impl Widget for ToolCallCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = match &self.status {
            ToolCallStatus::Running { .. } => Color::Blue,
            ToolCallStatus::Completed { .. } => Color::Indexed(245), // gray
            ToolCallStatus::Failed { .. } => Color::Red,
        };

        let duration_text = self.status.duration_label();
        let title_line = Line::from(vec![
            Span::styled("─ ", Style::default().fg(border_color)),
            self.status.icon(),
            Span::styled(
                self.tool_name.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ── {duration_text} "),
                Style::default().fg(border_color),
            ),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(title_line)
            .padding(Padding::horizontal(1));

        let inner = block.inner(area);
        block.render(area, buf);

        // Render input preview (and optionally output)
        let mut lines: Vec<Line<'_>> = vec![
            Line::from(Span::styled(
                truncate(&self.input_preview, inner.width as usize),
                Style::default().fg(Color::Indexed(252)),
            )),
        ];

        if self.expanded {
            if let Some(ref output) = self.output {
                lines.push(Line::from(""));
                let max_output_lines = 20;
                for (i, line) in output.lines().enumerate() {
                    if i >= max_output_lines {
                        lines.push(Line::from(Span::styled(
                            format!("... ({} more lines)", output.lines().count() - max_output_lines),
                            Style::default().fg(Color::DarkGray),
                        )));
                        break;
                    }
                    lines.push(Line::from(Span::raw(
                        truncate(line, inner.width as usize),
                    )));
                }
            }
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        paragraph.render(inner, buf);
    }
}

/// UTF-8 safe string truncation. Never panics on multi-byte characters.
fn truncate(s: &str, max_width: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_width {
        s.to_string()
    } else if max_width > 3 {
        let truncated: String = s.chars().take(max_width - 3).collect();
        format!("{truncated}...")
    } else {
        s.chars().take(max_width).collect()
    }
}

/// Calculate the height needed to render a tool card.
#[must_use]
pub fn tool_card_height(card: &ToolCallCard, width: u16) -> u16 {
    let base = 3; // border top + input line + border bottom
    if card.expanded && card.output.is_some() {
        let output_lines = card
            .output
            .as_ref()
            .map_or(0, |o| o.lines().count().min(20) + 1);
        base + output_lines as u16 + 1 // +1 for blank separator
    } else {
        let _ = width; // suppress unused warning
        base
    }
}
