//! Sidebar panel — interactive, multi-section panel with agents, files, routing, and activity.
//!
//! Supports focused navigation (j/k or arrows when sidebar is focused), expandable items,
//! and a live routing table showing which model handles each action category.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use events::{AgentStatus, AgentType};
use orchestrator::router::{ActionCategory, ModelRouter};

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

/// Which sidebar section is currently focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarSection {
    Agents,
    Files,
    Routing,
    Activity,
}

impl SidebarSection {
    #[allow(dead_code)]
    const ALL: [Self; 4] = [Self::Agents, Self::Files, Self::Routing, Self::Activity];

    fn next(self) -> Self {
        match self {
            Self::Agents => Self::Files,
            Self::Files => Self::Routing,
            Self::Routing => Self::Activity,
            Self::Activity => Self::Agents,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Agents => Self::Activity,
            Self::Files => Self::Agents,
            Self::Routing => Self::Files,
            Self::Activity => Self::Routing,
        }
    }
}

/// Sidebar state.
pub struct SidebarState {
    pub agents: Vec<AgentInfo>,
    pub files: Vec<TouchedFile>,
    pub tool_call_count: u32,
    pub background_tasks: Vec<BackgroundTask>,
    /// Currently focused section (when sidebar has focus).
    pub active_section: SidebarSection,
    /// Selected item index within the active section.
    pub selected_index: usize,
    /// Whether the sidebar is interactive (has keyboard focus).
    pub has_focus: bool,
    /// Expanded file index (to show full path).
    pub expanded_file: Option<usize>,
    /// Expanded agent index (to show full task).
    pub expanded_agent: Option<usize>,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            agents: Vec::new(),
            files: Vec::new(),
            tool_call_count: 0,
            background_tasks: Vec::new(),
            active_section: SidebarSection::Agents,
            selected_index: 0,
            has_focus: false,
            expanded_file: None,
            expanded_agent: None,
        }
    }
}

impl SidebarState {
    /// Track a file being touched by a tool call.
    pub fn track_file(&mut self, path: String, action: FileAction) {
        if let Some(existing) = self.files.iter_mut().find(|f| f.path == path) {
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

    /// Move selection down within the current section.
    pub fn select_next(&mut self) {
        let max = self.section_item_count();
        if max > 0 {
            self.selected_index = (self.selected_index + 1).min(max - 1);
        }
    }

    /// Move selection up within the current section.
    pub fn select_prev(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    /// Cycle to next section.
    pub fn next_section(&mut self) {
        self.active_section = self.active_section.next();
        self.selected_index = 0;
        self.expanded_file = None;
        self.expanded_agent = None;
    }

    /// Cycle to previous section.
    pub fn prev_section(&mut self) {
        self.active_section = self.active_section.prev();
        self.selected_index = 0;
        self.expanded_file = None;
        self.expanded_agent = None;
    }

    /// Toggle expand on the selected item.
    pub fn toggle_expand(&mut self) {
        match self.active_section {
            SidebarSection::Files => {
                if self.expanded_file == Some(self.selected_index) {
                    self.expanded_file = None;
                } else {
                    self.expanded_file = Some(self.selected_index);
                }
            }
            SidebarSection::Agents => {
                if self.expanded_agent == Some(self.selected_index) {
                    self.expanded_agent = None;
                } else {
                    self.expanded_agent = Some(self.selected_index);
                }
            }
            _ => {}
        }
    }

    /// Number of selectable items in the current section.
    fn section_item_count(&self) -> usize {
        match self.active_section {
            SidebarSection::Agents => self.agents.len(),
            SidebarSection::Files => self.files.len(),
            SidebarSection::Routing => 4, // explore, research, code, write
            SidebarSection::Activity => 0, // not selectable
        }
    }
}

/// Render the sidebar into the given area.
pub fn render_sidebar(
    state: &SidebarState,
    tokens: u64,
    elapsed_secs: u64,
    permission_mode: &str,
    router: &ModelRouter,
    area: Rect,
    buf: &mut Buffer,
) {
    let border_style = Style::default().fg(Color::Indexed(238));

    // Draw outer border
    let outer_block = Block::default()
        .borders(Borders::LEFT)
        .border_type(BorderType::Plain)
        .border_style(border_style);
    let inner = outer_block.inner(area);
    outer_block.render(area, buf);

    // Split sidebar into 4 sections
    let agent_count = state.agents.len().min(4) as u16;
    let file_count = state.files.len().min(8) as u16;

    let sections = Layout::vertical([
        Constraint::Length(agent_count.max(1) + 2),  // Agents
        Constraint::Length(file_count.max(1) + 2),   // Files
        Constraint::Length(7),                         // Routing table (4 rows + header + separator)
        Constraint::Min(4),                            // Activity
    ])
    .split(inner);

    let focused = state.has_focus;

    // ── Agents Section ──
    render_agents_section(
        &state.agents,
        focused && state.active_section == SidebarSection::Agents,
        state.selected_index,
        state.expanded_agent,
        sections[0],
        buf,
    );

    // ── Files Section ──
    render_files_section(
        &state.files,
        focused && state.active_section == SidebarSection::Files,
        state.selected_index,
        state.expanded_file,
        sections[1],
        buf,
    );

    // ── Routing Section ──
    render_routing_section(
        router,
        focused && state.active_section == SidebarSection::Routing,
        state.selected_index,
        sections[2],
        buf,
    );

    // ── Activity Section ──
    render_activity_section(
        state.tool_call_count,
        tokens,
        elapsed_secs,
        permission_mode,
        &state.background_tasks,
        sections[3],
        buf,
    );
}

fn section_header(title: &str, is_focused: bool) -> Line<'static> {
    let style = if is_focused {
        Style::default().fg(Color::Rgb(255, 107, 0)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    };
    let indicator = if is_focused { "▸ " } else { "  " };
    Line::from(vec![
        Span::styled(indicator, style),
        Span::styled(title.to_string(), style),
    ])
}

fn section_separator(width: u16) -> Line<'static> {
    Line::from(Span::styled(
        " ".to_string() + &"─".repeat((width as usize).saturating_sub(2)),
        Style::default().fg(Color::Indexed(238)),
    ))
}

fn render_agents_section(
    agents: &[AgentInfo],
    is_focused: bool,
    selected: usize,
    expanded: Option<usize>,
    area: Rect,
    buf: &mut Buffer,
) {
    let mut lines = vec![section_header("Agents", is_focused)];

    if agents.is_empty() {
        lines.push(Line::from(Span::styled("  (none active)", Style::default().fg(Color::DarkGray))));
    } else {
        for (i, agent) in agents.iter().take(4).enumerate() {
            let (icon, color) = match &agent.status {
                AgentStatus::Pending => ("◦", Color::DarkGray),
                AgentStatus::Running => ("●", Color::Blue),
                AgentStatus::Completed => ("✓", Color::Green),
                AgentStatus::Failed => ("✗", Color::Red),
            };

            let is_selected = is_focused && i == selected;
            let is_expanded = expanded == Some(i);

            let label = if is_expanded {
                truncate_sidebar(&agent.task_summary, area.width as usize - 5)
            } else {
                truncate_sidebar(&agent.agent_type.to_string(), area.width as usize - 5)
            };

            let bg = if is_selected { Color::Indexed(236) } else { Color::Reset };
            lines.push(Line::from(vec![
                Span::styled(format!(" {icon} "), Style::default().fg(color).bg(bg)),
                Span::styled(label, Style::default().fg(Color::Indexed(252)).bg(bg)),
            ]));
        }
    }

    lines.push(section_separator(area.width));
    Paragraph::new(lines).render(area, buf);
}

fn render_files_section(
    files: &[TouchedFile],
    is_focused: bool,
    selected: usize,
    expanded: Option<usize>,
    area: Rect,
    buf: &mut Buffer,
) {
    let mut lines = vec![section_header("Files", is_focused)];

    if files.is_empty() {
        lines.push(Line::from(Span::styled("  (no files yet)", Style::default().fg(Color::DarkGray))));
    } else {
        let max_width = area.width as usize - 5;
        for (i, file) in files.iter().take(8).enumerate() {
            let icon = file.action.icon();
            let color = file.action.color();

            let is_selected = is_focused && i == selected;
            let is_expanded = expanded == Some(i);

            // Expanded: show full path; collapsed: just filename
            let display = if is_expanded {
                truncate_sidebar(&file.path, max_width)
            } else {
                let fname = file.path.rsplit(['/', '\\']).next().unwrap_or(&file.path);
                truncate_sidebar(fname, max_width)
            };

            let bg = if is_selected { Color::Indexed(236) } else { Color::Reset };
            lines.push(Line::from(vec![
                Span::styled(format!(" {icon} "), Style::default().fg(color).bg(bg)),
                Span::styled(display, Style::default().fg(Color::Indexed(252)).bg(bg)),
            ]));
        }
    }

    lines.push(section_separator(area.width));
    Paragraph::new(lines).render(area, buf);
}

fn render_routing_section(
    router: &ModelRouter,
    is_focused: bool,
    selected: usize,
    area: Rect,
    buf: &mut Buffer,
) {
    let mut lines = vec![section_header("Routing", is_focused)];

    for (i, cat) in ActionCategory::ALL.iter().enumerate() {
        let profile = router.table.get(*cat);
        let model = router.resolver.resolve(profile.model_tier);
        // Shorten model name for sidebar
        let short_model = model
            .strip_prefix("claude-")
            .or_else(|| model.strip_prefix("gpt-"))
            .or_else(|| model.strip_prefix("gemini-"))
            .unwrap_or(model);
        let short_model = truncate_sidebar(short_model, area.width as usize - 14);

        let cat_color = match cat {
            ActionCategory::Explore => Color::Blue,
            ActionCategory::Research => Color::Magenta,
            ActionCategory::Code => Color::Green,
            ActionCategory::Write => Color::Yellow,
        };

        let is_selected = is_focused && i == selected;
        let bg = if is_selected { Color::Indexed(236) } else { Color::Reset };

        lines.push(Line::from(vec![
            Span::styled(format!(" {:<8} ", cat.as_str()), Style::default().fg(cat_color).bg(bg)),
            Span::styled(short_model, Style::default().fg(Color::Indexed(245)).bg(bg)),
        ]));
    }

    lines.push(section_separator(area.width));
    Paragraph::new(lines).render(area, buf);
}

fn render_activity_section(
    tool_calls: u32,
    tokens: u64,
    elapsed_secs: u64,
    permission_mode: &str,
    bg_tasks: &[BackgroundTask],
    area: Rect,
    buf: &mut Buffer,
) {
    let header = section_header("Activity", false);

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

    // Permission mode indicator
    let (perm_icon, perm_color) = match permission_mode {
        "read-only" | "readonly" => ("R", Color::Blue),
        "workspace" | "workspace-write" => ("W", Color::Yellow),
        "prompt" | "ask" | "default" => ("P", Color::Cyan),
        "allow" | "allow-all" => ("A", Color::Green),
        "full" | "danger-full-access" | "yolo" => ("F", Color::Red),
        _ => ("?", Color::DarkGray),
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
        Line::from(vec![
            Span::styled(format!(" {perm_icon} "), Style::default().fg(perm_color)),
            Span::styled(format!("mode: {permission_mode}"), Style::default().fg(Color::Indexed(252))),
        ]),
    ];

    // Background tasks
    if !bg_tasks.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Background",
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
    lines.push(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Span::styled(":section ", Style::default().fg(Color::Indexed(238))),
        Span::styled("j/k", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Span::styled(":nav ", Style::default().fg(Color::Indexed(238))),
        Span::styled("Ctrl+B", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Span::styled(":hide", Style::default().fg(Color::Indexed(238))),
    ]));

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
