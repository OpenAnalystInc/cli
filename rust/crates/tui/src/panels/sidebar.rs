//! Sidebar panel — composites Agents + Files + Activity into a vertical column.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use events::{AgentStatus, AgentType};

/// Info about a tracked agent for sidebar display.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub agent_id: String,
    pub agent_type: AgentType,
    pub task_summary: String,
    pub status: AgentStatus,
}

/// Info about a file touched during the session.
#[derive(Debug, Clone)]
pub struct TouchedFile {
    pub path: String,
    pub action: FileAction,
}

#[derive(Debug, Clone, Copy)]
pub enum FileAction {
    Read,
    Edited,
    Created,
}

impl FileAction {
    fn icon(self) -> &'static str {
        match self {
            Self::Read => "○",
            Self::Edited => "●",
            Self::Created => "+",
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Read => Color::DarkGray,
            Self::Edited => Color::Yellow,
            Self::Created => Color::Green,
        }
    }
}

/// A background task.
#[derive(Debug, Clone)]
pub struct BackgroundTask {
    pub id: String,
    pub prompt_preview: String,
    pub status: BackgroundTaskStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundTaskStatus {
    Running,
    Completed,
    Failed,
}

/// Sidebar state.
pub struct SidebarState {
    pub agents: Vec<AgentInfo>,
    pub files: Vec<TouchedFile>,
    pub tool_call_count: u32,
    pub background_tasks: Vec<BackgroundTask>,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            agents: Vec::new(),
            files: Vec::new(),
            tool_call_count: 0,
            background_tasks: Vec::new(),
        }
    }
}

impl SidebarState {
    /// Track a file being touched by a tool call.
    pub fn track_file(&mut self, path: String, action: FileAction) {
        // Update if already tracked, otherwise add
        if let Some(existing) = self.files.iter_mut().find(|f| f.path == path) {
            // Upgrade action: Read < Edited < Created
            match (existing.action, action) {
                (FileAction::Read, FileAction::Edited | FileAction::Created) => {
                    existing.action = action;
                }
                (FileAction::Edited, FileAction::Created) => {
                    existing.action = action;
                }
                _ => {}
            }
        } else {
            // Keep most recent files at top, limit to 20
            self.files.insert(0, TouchedFile { path, action });
            if self.files.len() > 20 {
                self.files.pop();
            }
        }
    }

    /// Update or add agent info.
    pub fn update_agent(&mut self, agent_id: String, agent_type: AgentType, task: String, status: AgentStatus) {
        if let Some(existing) = self.agents.iter_mut().find(|a| a.agent_id == agent_id) {
            existing.status = status;
            if !task.is_empty() {
                existing.task_summary = task;
            }
        } else {
            self.agents.push(AgentInfo {
                agent_id,
                agent_type,
                task_summary: task,
                status,
            });
        }
    }
}

/// Render the sidebar into the given area.
pub fn render_sidebar(state: &SidebarState, tokens: u64, elapsed_secs: u64, area: Rect, buf: &mut Buffer) {
    let border_style = Style::default().fg(Color::Indexed(238));

    // Draw outer border
    let outer_block = Block::default()
        .borders(Borders::LEFT)
        .border_type(BorderType::Plain)
        .border_style(border_style);
    let inner = outer_block.inner(area);
    outer_block.render(area, buf);

    // Split sidebar into sections
    let agent_count = state.agents.len().min(4) as u16;
    let file_count = state.files.len().min(8) as u16;

    let sections = Layout::vertical([
        Constraint::Length(agent_count.max(1) + 2),  // Agents section
        Constraint::Length(file_count.max(1) + 2),   // Files section
        Constraint::Min(4),                           // Activity section
    ])
    .split(inner);

    // ── Agents Section ──
    render_agents_section(&state.agents, sections[0], buf);

    // ── Files Section ──
    render_files_section(&state.files, sections[1], buf);

    // ── Activity Section ──
    render_activity_section(state.tool_call_count, tokens, elapsed_secs, &state.background_tasks, sections[2], buf);
}

fn render_agents_section(agents: &[AgentInfo], area: Rect, buf: &mut Buffer) {
    let header = Line::from(vec![
        Span::styled(" Agents", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]);

    let mut lines = vec![header];

    if agents.is_empty() {
        lines.push(Line::from(Span::styled(" (none active)", Style::default().fg(Color::DarkGray))));
    } else {
        for agent in agents.iter().take(4) {
            let (icon, color) = match &agent.status {
                AgentStatus::Pending => ("◦", Color::DarkGray),
                AgentStatus::Running => ("●", Color::Blue),
                AgentStatus::Completed => ("✓", Color::Green),
                AgentStatus::Failed => ("✗", Color::Red),
            };
            let label = truncate_sidebar(&agent.agent_type.to_string(), area.width as usize - 5);
            lines.push(Line::from(vec![
                Span::styled(format!(" {icon} "), Style::default().fg(color)),
                Span::styled(label, Style::default().fg(Color::Indexed(252))),
            ]));
        }
    }

    let separator = Line::from(Span::styled(
        " ".to_string() + &"─".repeat((area.width as usize).saturating_sub(2)),
        Style::default().fg(Color::Indexed(238)),
    ));
    lines.push(separator);

    Paragraph::new(lines).render(area, buf);
}

fn render_files_section(files: &[TouchedFile], area: Rect, buf: &mut Buffer) {
    let header = Line::from(vec![
        Span::styled(" Files", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]);

    let mut lines = vec![header];

    if files.is_empty() {
        lines.push(Line::from(Span::styled(" (no files yet)", Style::default().fg(Color::DarkGray))));
    } else {
        let max_width = area.width as usize - 5;
        for file in files.iter().take(8) {
            let icon = file.action.icon();
            let color = file.action.color();
            // Show just the filename, not full path
            let display = file.path.rsplit(['/', '\\']).next().unwrap_or(&file.path);
            let display = truncate_sidebar(display, max_width);
            lines.push(Line::from(vec![
                Span::styled(format!(" {icon} "), Style::default().fg(color)),
                Span::styled(display, Style::default().fg(Color::Indexed(252))),
            ]));
        }
    }

    let separator = Line::from(Span::styled(
        " ".to_string() + &"─".repeat((area.width as usize).saturating_sub(2)),
        Style::default().fg(Color::Indexed(238)),
    ));
    lines.push(separator);

    Paragraph::new(lines).render(area, buf);
}

fn render_activity_section(
    tool_calls: u32,
    tokens: u64,
    elapsed_secs: u64,
    bg_tasks: &[BackgroundTask],
    area: Rect,
    buf: &mut Buffer,
) {
    let header = Line::from(vec![
        Span::styled(" Activity", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]);

    let token_str = if tokens < 1_000 {
        format!("{tokens}")
    } else if tokens < 1_000_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    };

    let time_str = if elapsed_secs < 60 {
        format!("{elapsed_secs}s")
    } else {
        format!("{}m {:02}s", elapsed_secs / 60, elapsed_secs % 60)
    };

    let mut lines = vec![
        header,
        Line::from(vec![
            Span::styled(" ↕ ", Style::default().fg(Color::Blue)),
            Span::styled(format!("{tool_calls} tool calls"), Style::default().fg(Color::Indexed(252))),
        ]),
        Line::from(vec![
            Span::styled(" ↓ ", Style::default().fg(Color::Green)),
            Span::styled(format!("{token_str} tokens"), Style::default().fg(Color::Indexed(252))),
        ]),
        Line::from(vec![
            Span::styled(" ◷ ", Style::default().fg(Color::Yellow)),
            Span::styled(format!("{time_str} elapsed"), Style::default().fg(Color::Indexed(252))),
        ]),
    ];

    // Background tasks
    if !bg_tasks.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Background",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        let max_w = area.width as usize - 5;
        for task in bg_tasks.iter().take(3) {
            let (icon, color) = match task.status {
                BackgroundTaskStatus::Running => ("⠋", Color::Blue),
                BackgroundTaskStatus::Completed => ("✓", Color::Green),
                BackgroundTaskStatus::Failed => ("✗", Color::Red),
            };
            let display = truncate_sidebar(&task.prompt_preview, max_w);
            lines.push(Line::from(vec![
                Span::styled(format!(" {icon} "), Style::default().fg(color)),
                Span::styled(display, Style::default().fg(Color::Indexed(252))),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" Ctrl+B hide", Style::default().fg(Color::DarkGray))));

    Paragraph::new(lines).render(area, buf);
}

/// Truncate a string for sidebar display width.
fn truncate_sidebar(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else if max > 3 {
        let t: String = s.chars().take(max - 3).collect();
        format!("{t}...")
    } else {
        s.chars().take(max).collect()
    }
}
