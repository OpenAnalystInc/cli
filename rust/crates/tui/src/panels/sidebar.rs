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

/// A project agent definition loaded from .openanalyst/agents/*.md files.
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    /// Display name (derived from filename).
    pub name: String,
    /// Description (first line of the markdown body).
    pub description: String,
    /// Full system prompt (the file contents after frontmatter).
    pub system_prompt: String,
    /// Source path for reference.
    pub source: String,
}

/// Which sidebar section is currently focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarSection {
    Agents,
    Files,
    Plans,
    Routing,
    Activity,
}

impl SidebarSection {
    #[allow(dead_code)]
    const ALL: [Self; 5] = [Self::Agents, Self::Files, Self::Plans, Self::Routing, Self::Activity];

    fn next(self) -> Self {
        match self {
            Self::Agents => Self::Files,
            Self::Files => Self::Plans,
            Self::Plans => Self::Routing,
            Self::Routing => Self::Activity,
            Self::Activity => Self::Agents,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Agents => Self::Activity,
            Self::Files => Self::Agents,
            Self::Plans => Self::Files,
            Self::Routing => Self::Plans,
            Self::Activity => Self::Routing,
        }
    }
}

/// A tracked plan.
#[derive(Debug, Clone)]
pub struct PlanInfo {
    pub name: String,
    pub status: PlanStatus,
    pub source: String, // "file" or "session"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    Todo,
    InProgress,
    Done,
}

/// Sidebar state.
pub struct SidebarState {
    pub agents: Vec<AgentInfo>,
    pub files: Vec<TouchedFile>,
    pub plans: Vec<PlanInfo>,
    pub tool_call_count: u32,
    pub background_tasks: Vec<BackgroundTask>,
    /// Project agent definitions discovered from .openanalyst/agents/*.md.
    pub available_agents: Vec<AgentDefinition>,
    /// Index of the currently selected/active agent (None = default, no agent).
    pub selected_agent_index: Option<usize>,
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
    /// Available models from all configured providers (cached on startup).
    pub available_models: Vec<String>,
    /// Per-category model index for cycling through available_models.
    /// [explore, research, code, write] — each indexes into available_models.
    pub routing_model_index: [usize; 4],
}

impl Default for SidebarState {
    fn default() -> Self {
        // Discover available models from all configured providers
        let available_models: Vec<String> = api::available_models()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        Self {
            agents: Vec::new(),
            files: Vec::new(),
            plans: Vec::new(),
            tool_call_count: 0,
            background_tasks: Vec::new(),
            available_agents: Vec::new(),
            selected_agent_index: None,
            active_section: SidebarSection::Agents,
            selected_index: 0,
            has_focus: false,
            expanded_file: None,
            expanded_agent: None,
            available_models,
            routing_model_index: [0; 4],
        }
    }
}

impl SidebarState {
    /// Discover project agents from .openanalyst/agents/*.md files.
    /// Also checks ~/.openanalyst/agents/ for user-level agents.
    pub fn discover_agents_from_files(&mut self) {
        self.available_agents.clear();

        let dirs_to_scan: Vec<std::path::PathBuf> = {
            let mut dirs = Vec::new();
            // Project-level agents
            if let Ok(cwd) = std::env::current_dir() {
                dirs.push(cwd.join(".openanalyst").join("agents"));
            }
            // User-level agents
            if let Some(home) = dirs_get_home() {
                dirs.push(home.join(".openanalyst").join("agents"));
            }
            dirs
        };

        for agents_dir in dirs_to_scan {
            if !agents_dir.is_dir() {
                continue;
            }
            let entries = match std::fs::read_dir(&agents_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.extension().map_or(false, |ext| ext == "md") {
                    continue;
                }
                let name = path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                if name.is_empty() {
                    continue;
                }
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                // Parse simple frontmatter (---\n...\n---) and extract body
                let (description, system_prompt) = parse_agent_md(&content);

                // Skip duplicates (project agents override user agents)
                if self.available_agents.iter().any(|a| a.name == name) {
                    continue;
                }

                self.available_agents.push(AgentDefinition {
                    name,
                    description,
                    system_prompt,
                    source: path.to_string_lossy().to_string(),
                });
            }
        }
    }

    /// Get the currently selected agent definition (if any).
    #[must_use]
    pub fn selected_agent(&self) -> Option<&AgentDefinition> {
        self.selected_agent_index.and_then(|i| self.available_agents.get(i))
    }

    /// Toggle agent selection at the given index. Returns the selected agent name or None.
    pub fn toggle_agent_selection(&mut self, index: usize) -> Option<String> {
        if self.selected_agent_index == Some(index) {
            // Deselect
            self.selected_agent_index = None;
            None
        } else if index < self.available_agents.len() {
            self.selected_agent_index = Some(index);
            Some(self.available_agents[index].name.clone())
        } else {
            None
        }
    }

    /// Discover plans from .openanalyst/plans/ directory.
    pub fn discover_plans(&mut self) {
        let cwd = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };

        let plans_dir = cwd.join(".openanalyst").join("plans");
        if !plans_dir.is_dir() {
            return;
        }

        let entries = match std::fs::read_dir(&plans_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "md") {
                let name = path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                // Detect status from content (look for [DONE], [IN PROGRESS] markers)
                let status = if let Ok(content) = std::fs::read_to_string(&path) {
                    let lower = content.to_ascii_lowercase();
                    if lower.contains("[done]") || lower.contains("completed") {
                        PlanStatus::Done
                    } else if lower.contains("[in progress]") || lower.contains("in_progress") {
                        PlanStatus::InProgress
                    } else {
                        PlanStatus::Todo
                    }
                } else {
                    PlanStatus::Todo
                };
                self.plans.push(PlanInfo {
                    name,
                    status,
                    source: "file".to_string(),
                });
            }
        }
    }

    /// Discover project files on startup — scans CWD for important files.
    /// Similar to Gemini CLI's FileDiscoveryService but lightweight.
    pub fn discover_project_files(&mut self) {
        let cwd = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };

        // Priority files to show (config, entrypoints, docs)
        let priority_names: &[&str] = &[
            "OPENANALYST.md", "CLAUDE.md", "README.md", "Cargo.toml",
            "package.json", "pyproject.toml", "go.mod", "Makefile",
            ".gitignore", "tsconfig.json", "rust-toolchain.toml",
        ];

        // First: add priority files that exist
        for name in priority_names {
            let path = cwd.join(name);
            if path.exists() {
                self.files.push(TouchedFile {
                    path: name.to_string(),
                    action: FileAction::Read,
                });
            }
        }

        // If no priority files found, scan CWD root for any files
        if self.files.is_empty() {
            if let Ok(entries) = std::fs::read_dir(&cwd) {
                for entry in entries.flatten().take(15) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') || name == "target" || name == "node_modules" {
                        continue;
                    }
                    let is_dir = entry.path().is_dir();
                    let display = if is_dir { format!("{name}/") } else { name };
                    self.files.push(TouchedFile {
                        path: display,
                        action: FileAction::Read,
                    });
                }
            }
        }

        // Second: scan top-level source directories for code files
        let source_dirs: &[&str] = &["src", "crates", "packages", "lib", "app", "rust"];
        for dir_name in source_dirs {
            let dir = cwd.join(dir_name);
            if !dir.is_dir() {
                continue;
            }
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                if self.files.len() >= 20 {
                    return;
                }
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip hidden files and common non-code dirs
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }
                let display = format!("{}/{}", dir_name, name);
                if path.is_dir() {
                    // Show directories as entries
                    if !self.files.iter().any(|f| f.path == display) {
                        self.files.push(TouchedFile {
                            path: display,
                            action: FileAction::Read,
                        });
                    }
                } else if path.extension().map_or(false, |ext| {
                    matches!(ext.to_str(), Some("rs" | "py" | "ts" | "tsx" | "js" | "go" | "toml" | "json" | "md"))
                }) {
                    if !self.files.iter().any(|f| f.path == display) {
                        self.files.push(TouchedFile {
                            path: display,
                            action: FileAction::Read,
                        });
                    }
                }
            }
        }
    }

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
            if self.files.len() > 50 {
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

    /// Cycle to the next available model for a routing category (0=explore, 1=research, 2=code, 3=write).
    /// Returns the new model name, or None if no models available.
    pub fn cycle_routing_model(&mut self, category_idx: usize) -> Option<String> {
        if self.available_models.is_empty() || category_idx >= 4 {
            return None;
        }
        let current = self.routing_model_index[category_idx];
        let next = (current + 1) % self.available_models.len();
        self.routing_model_index[category_idx] = next;
        Some(self.available_models[next].clone())
    }

    /// Number of selectable items in the current section.
    fn section_item_count(&self) -> usize {
        match self.active_section {
            SidebarSection::Agents => self.agents.len() + self.available_agents.len(),
            SidebarSection::Files => self.files.len(),
            SidebarSection::Plans => self.plans.len(),
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
    // Border changes color when sidebar is focused
    let border_color = if state.has_focus {
        Color::Yellow
    } else {
        Color::Rgb(50, 130, 255)
    };
    let border_style = Style::default().fg(border_color);

    // Draw outer border with focus indicator
    let outer_block = Block::default()
        .borders(Borders::LEFT)
        .border_type(BorderType::Plain)
        .border_style(border_style);
    let inner = outer_block.inner(area);
    outer_block.render(area, buf);

    // Split sidebar into 5 sections
    let agent_count = (state.agents.len() + state.available_agents.len()).min(12) as u16;
    let file_count = state.files.len().min(5) as u16;
    let plan_count = state.plans.len().min(5) as u16;

    let sections = Layout::vertical([
        Constraint::Length(agent_count.max(1) + 2),  // Agents
        Constraint::Length(file_count.max(1) + 2),   // Files
        Constraint::Length(plan_count.max(1) + 2),   // Plans
        Constraint::Length(7),                         // Routing table
        Constraint::Min(4),                            // Activity
    ])
    .split(inner);

    let focused = state.has_focus;

    // ── Agents Section ──
    render_agents_section(
        &state.agents,
        &state.available_agents,
        state.selected_agent_index,
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

    // ── Plans Section ──
    render_plans_section(
        &state.plans,
        focused && state.active_section == SidebarSection::Plans,
        state.selected_index,
        sections[2],
        buf,
    );

    // ── Routing Section ──
    render_routing_section(
        router,
        focused && state.active_section == SidebarSection::Routing,
        state.selected_index,
        sections[3],
        buf,
    );

    // ── Activity Section ──
    render_activity_section(
        state.tool_call_count,
        tokens,
        elapsed_secs,
        permission_mode,
        &state.background_tasks,
        sections[4],
        buf,
    );
}

fn section_header(title: &str, is_focused: bool) -> Line<'static> {
    if is_focused {
        // Bright yellow background highlight for active section
        let bg = Color::Indexed(239);
        Line::from(vec![
            Span::styled("▸ ", Style::default().fg(Color::Yellow).bg(bg).add_modifier(Modifier::BOLD)),
            Span::styled(title.to_string(), Style::default().fg(Color::Yellow).bg(bg).add_modifier(Modifier::BOLD)),
            Span::styled(" ◂", Style::default().fg(Color::Yellow).bg(bg)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::Cyan)),
            Span::styled(title.to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ])
    }
}

fn section_separator(width: u16, is_next_focused: bool) -> Line<'static> {
    let color = if is_next_focused {
        Color::Yellow
    } else {
        Color::Indexed(238)
    };
    Line::from(Span::styled(
        " ".to_string() + &"─".repeat((width as usize).saturating_sub(2)),
        Style::default().fg(color),
    ))
}

fn render_agents_section(
    agents: &[AgentInfo],
    available_agents: &[AgentDefinition],
    active_agent_idx: Option<usize>,
    is_focused: bool,
    selected: usize,
    expanded: Option<usize>,
    area: Rect,
    buf: &mut Buffer,
) {
    let mut lines = vec![section_header("Agents", is_focused)];
    let max_label = area.width as usize - 5;
    let mut item_idx = 0usize;

    // Running agents first (scrollable)
    let max_visible_agents = 6;
    if !agents.is_empty() {
        let agent_scroll = if is_focused && selected >= max_visible_agents { selected - (max_visible_agents - 1) } else { 0 };
        for (i, agent) in agents.iter().enumerate().skip(agent_scroll).take(max_visible_agents) {
            let (icon, color) = match &agent.status {
                AgentStatus::Pending => ("◦", Color::DarkGray),
                AgentStatus::Running => ("●", Color::Blue),
                AgentStatus::Completed => ("✓", Color::Green),
                AgentStatus::Failed => ("✗", Color::Red),
            };

            let is_selected = is_focused && item_idx == selected;
            let is_expanded = expanded == Some(i);

            let label = if is_expanded {
                truncate_sidebar(&agent.task_summary, max_label)
            } else {
                truncate_sidebar(&agent.agent_type.to_string(), max_label)
            };

            let bg = if is_selected { Color::Indexed(239) } else { Color::Reset };
            let text_color = if is_selected { Color::White } else { Color::Indexed(252) };
            let sel_prefix = if is_selected { "▸" } else { " " };
            lines.push(Line::from(vec![
                Span::styled(sel_prefix, Style::default().fg(Color::Yellow).bg(bg)),
                Span::styled(format!("{icon} "), Style::default().fg(color).bg(bg)),
                Span::styled(label, Style::default().fg(text_color).bg(bg)),
            ]));
            item_idx += 1;
        }
    }

    // Available project agents (selectable)
    if !available_agents.is_empty() {
        if !agents.is_empty() {
            lines.push(Line::from(Span::styled("  ─ project ─", Style::default().fg(Color::Indexed(238)))));
        }
        for (def_idx, def) in available_agents.iter().take(8).enumerate() {
            let is_active = active_agent_idx == Some(def_idx);
            let is_selected = is_focused && item_idx == selected;

            let icon = if is_active { "◆" } else { "◇" };
            let icon_color = if is_active { Color::Rgb(50, 130, 255) } else { Color::Indexed(245) };

            let label = truncate_sidebar(&def.name, max_label);
            let bg = if is_selected { Color::Indexed(239) } else { Color::Reset };
            let text_color = if is_active {
                Color::Rgb(50, 130, 255)
            } else if is_selected {
                Color::White
            } else {
                Color::Indexed(252)
            };
            let sel_prefix = if is_selected { "▸" } else { " " };

            lines.push(Line::from(vec![
                Span::styled(sel_prefix, Style::default().fg(Color::Yellow).bg(bg)),
                Span::styled(format!("{icon} "), Style::default().fg(icon_color).bg(bg)),
                Span::styled(label, Style::default().fg(text_color).bg(bg).add_modifier(
                    if is_active { Modifier::BOLD } else { Modifier::empty() }
                )),
            ]));
            item_idx += 1;
        }
    }

    if agents.is_empty() && available_agents.is_empty() {
        lines.push(Line::from(Span::styled("  (none active)", Style::default().fg(Color::DarkGray))));
    }

    lines.push(section_separator(area.width, is_focused));
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
        // Show 5 files max, scrollable via selected index
        let scroll_offset = if selected >= 5 { selected - 4 } else { 0 };
        for (i, file) in files.iter().enumerate().skip(scroll_offset).take(5) {
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

            let bg = if is_selected { Color::Indexed(239) } else { Color::Reset };
            let text_color = if is_selected { Color::White } else { Color::Indexed(252) };
            let sel_prefix = if is_selected { "▸" } else { " " };
            lines.push(Line::from(vec![
                Span::styled(sel_prefix, Style::default().fg(Color::Yellow).bg(bg)),
                Span::styled(format!("{icon} "), Style::default().fg(color).bg(bg)),
                Span::styled(display, Style::default().fg(text_color).bg(bg)),
            ]));
        }
    }

    lines.push(section_separator(area.width, is_focused));
    Paragraph::new(lines).render(area, buf);
}

fn render_plans_section(
    plans: &[PlanInfo],
    is_focused: bool,
    selected: usize,
    area: Rect,
    buf: &mut Buffer,
) {
    let mut lines = vec![section_header("Plans", is_focused)];

    if plans.is_empty() {
        lines.push(Line::from(Span::styled("  (no plans)", Style::default().fg(Color::DarkGray))));
    } else {
        let done = plans.iter().filter(|p| p.status == PlanStatus::Done).count();
        let in_progress = plans.iter().filter(|p| p.status == PlanStatus::InProgress).count();
        let todo = plans.iter().filter(|p| p.status == PlanStatus::Todo).count();

        // Summary line
        lines.push(Line::from(vec![
            Span::styled(format!(" {done}"), Style::default().fg(Color::Green)),
            Span::styled(" done ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{in_progress}"), Style::default().fg(Color::Yellow)),
            Span::styled(" active ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{todo}"), Style::default().fg(Color::Indexed(245))),
            Span::styled(" todo", Style::default().fg(Color::DarkGray)),
        ]));

        let plan_scroll = if is_focused && selected >= 5 { selected - 4 } else { 0 };
        for (i, plan) in plans.iter().enumerate().skip(plan_scroll).take(5) {
            let is_selected = is_focused && i == selected;
            let bg = if is_selected { Color::Indexed(239) } else { Color::Reset };
            let (icon, icon_color) = match plan.status {
                PlanStatus::Done => ("✓", Color::Green),
                PlanStatus::InProgress => ("●", Color::Yellow),
                PlanStatus::Todo => ("○", Color::Indexed(245)),
            };
            let sel_prefix = if is_selected { "▸" } else { " " };
            let text_color = if is_selected { Color::White } else { Color::Indexed(252) };
            let name = truncate_sidebar(&plan.name, area.width as usize - 5);
            lines.push(Line::from(vec![
                Span::styled(sel_prefix, Style::default().fg(Color::Yellow).bg(bg)),
                Span::styled(format!("{icon} "), Style::default().fg(icon_color).bg(bg)),
                Span::styled(name, Style::default().fg(text_color).bg(bg)),
            ]));
        }
    }

    lines.push(section_separator(area.width, is_focused));
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
        let short_model = shorten_model_name(model);

        let cat_color = match cat {
            ActionCategory::Explore => Color::Blue,
            ActionCategory::Research => Color::Magenta,
            ActionCategory::Code => Color::Green,
            ActionCategory::Write => Color::Yellow,
        };

        // Tier indicator color
        let tier_color = match profile.model_tier {
            orchestrator::router::ModelTier::Fast => Color::Cyan,
            orchestrator::router::ModelTier::Balanced => Color::Yellow,
            orchestrator::router::ModelTier::Capable => Color::Green,
        };

        let is_selected = is_focused && i == selected;
        let bg = if is_selected { Color::Indexed(239) } else { Color::Reset };
        let sel_prefix = if is_selected { "▸" } else { " " };

        // Show: ▸ explore  ● model-name
        // The ● color indicates the tier; Enter cycles it
        let max_model = area.width as usize - 14;
        let short_model = truncate_sidebar(&short_model, max_model);
        lines.push(Line::from(vec![
            Span::styled(sel_prefix, Style::default().fg(Color::Yellow).bg(bg)),
            Span::styled(format!("{:<8}", cat.as_str()), Style::default().fg(cat_color).bg(bg)),
            Span::styled(" ● ", Style::default().fg(tier_color).bg(bg)),
            Span::styled(short_model, Style::default().fg(Color::Indexed(245)).bg(bg)),
        ]));
    }

    lines.push(section_separator(area.width, is_focused));
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

    // Permission mode indicator — short label for sidebar
    let (perm_icon, perm_color, perm_label) = match permission_mode {
        "read-only" | "readonly" => ("R", Color::Blue, "read-only"),
        "workspace" | "workspace-write" => ("W", Color::Yellow, "workspace"),
        "prompt" | "ask" | "default" => ("P", Color::Cyan, "prompt"),
        "allow" | "allow-all" => ("A", Color::Green, "allow-all"),
        "full" | "danger-full-access" | "yolo" => ("F", Color::Red, "full-access"),
        _ => ("?", Color::DarkGray, permission_mode),
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
            Span::styled(format!("mode: {perm_label}"), Style::default().fg(Color::Indexed(252))),
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
        for task in bg_tasks.iter().take(8) {
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
        Span::styled("Esc", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Span::styled(":back ", Style::default().fg(Color::Indexed(238))),
        Span::styled("F2", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Span::styled(":hide", Style::default().fg(Color::Indexed(238))),
    ]));

    Paragraph::new(lines).render(area, buf);
}

/// Shorten a model name for sidebar display (public for keybinding handler).
pub fn shorten_model_name_pub(model: &str) -> String {
    shorten_model_name(model)
}

/// Shorten a model name for sidebar display by stripping common prefixes
/// and abbreviating known patterns.
fn shorten_model_name(model: &str) -> String {
    // Strip common provider prefixes
    let short = model
        .strip_prefix("claude-")
        .or_else(|| model.strip_prefix("gpt-"))
        .or_else(|| model.strip_prefix("gemini-"))
        .or_else(|| model.strip_prefix("grok-"))
        .or_else(|| model.strip_prefix("openanalyst-"))
        .or_else(|| model.strip_prefix("openrouter/"))
        .or_else(|| model.strip_prefix("bedrock/"))
        .unwrap_or(model);
    // Further abbreviations for long names
    short
        .replace("sonnet-4-6", "son-4.6")
        .replace("opus-4-6", "opus-4.6")
        .replace("haiku-4-5", "haiku-4.5")
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

/// Parse an agent .md file: extract description (first non-empty body line) and system prompt.
/// Supports optional YAML frontmatter delimited by `---`.
fn parse_agent_md(content: &str) -> (String, String) {
    let trimmed = content.trim();

    // Check for frontmatter
    let body = if trimmed.starts_with("---") {
        // Find closing ---
        if let Some(end) = trimmed[3..].find("\n---") {
            trimmed[3 + end + 4..].trim()
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    // First non-empty line is the description
    let description = body
        .lines()
        .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .or_else(|| body.lines().find(|l| !l.trim().is_empty()))
        .unwrap_or("")
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_string();

    (description, body.to_string())
}

/// Get the user's home directory.
fn dirs_get_home() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
}
