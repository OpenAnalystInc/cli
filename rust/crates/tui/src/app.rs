//! Main TUI application state and rendering.

use std::time::{Duration, Instant};

use events::{Action, ActionTx, AgentStatus, PanelId, UiEvent, UiEventRx};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use tui_widgets::status_bar::AgentPhase;
use tui_widgets::{InputBox, InputBoxState, PermissionDialog, StatusBar, ToolCallCard, ToolCallStatus};

use crate::banner::{Banner, BannerAccountInfo};
use crate::layout::compute_layout;
use crate::panels::chat::{ChatMessage, ChatPanel};
use crate::panels::sidebar::{self, FileAction, SidebarState};

/// Exit state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitState {
    /// Normal operation.
    Running,
    /// Waiting for user confirmation to save and quit.
    ConfirmExit,
}

/// The main TUI application state.
pub struct App {
    // Panels
    pub chat: ChatPanel,
    pub status_bar: StatusBar,
    pub input_state: InputBoxState,
    pub sidebar_state: SidebarState,

    // Modal overlays
    pub permission_dialog: Option<PermissionDialog>,

    // Layout state
    pub sidebar_visible: bool,
    pub focus: PanelId,

    // State
    pub scroll_mode: bool,
    pub should_quit: bool,
    pub exit_state: ExitState,
    pub turn_start: Option<Instant>,
    pub is_streaming: bool,

    // Channels
    pub ui_rx: UiEventRx,
    pub action_tx: ActionTx,

    // Banner info
    pub banner_info: Option<BannerAccountInfo>,
    pub banner_shown: bool,

    // Spinner state
    pub spinner_state: tui_widgets::spinner::SpinnerState,

    // Input queue — prompts submitted while streaming, sent after current turn ends
    pub pending_queue: Vec<String>,

    // Effort level — controls thinking budget for supported models
    pub effort: EffortLevel,

    // Per-action model override — used for one prompt then reverts
    pub model_override: Option<String>,
}

/// Thinking effort level — maps to token budgets for extended thinking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
    Max,
}

impl Default for EffortLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl EffortLevel {
    /// Human-readable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Max => "max",
        }
    }

    /// Thinking budget tokens for Anthropic extended thinking.
    #[must_use]
    pub const fn thinking_budget(self) -> u32 {
        match self {
            Self::Low => 1_024,
            Self::Medium => 8_192,
            Self::High => 32_000,
            Self::Max => 128_000,
        }
    }

    /// Parse from string.
    #[must_use]
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "low" | "l" | "1" => Some(Self::Low),
            "medium" | "med" | "m" | "2" => Some(Self::Medium),
            "high" | "h" | "3" => Some(Self::High),
            "max" | "x" | "4" => Some(Self::Max),
            _ => None,
        }
    }
}

impl App {
    /// Create a new App.
    pub fn new(ui_rx: UiEventRx, action_tx: ActionTx) -> Self {
        Self {
            chat: ChatPanel::default(),
            status_bar: StatusBar::default(),
            input_state: InputBoxState::default(),
            sidebar_state: SidebarState::default(),
            permission_dialog: None,
            sidebar_visible: true,
            focus: PanelId::Input,
            scroll_mode: false,
            should_quit: false,
            exit_state: ExitState::Running,
            turn_start: None,
            is_streaming: false,
            ui_rx,
            action_tx,
            banner_info: None,
            banner_shown: false,
            spinner_state: tui_widgets::spinner::SpinnerState::default(),
            pending_queue: Vec::new(),
            effort: EffortLevel::default(),
            model_override: None,
        }
    }

    /// Set banner info and inject the banner into the chat.
    pub fn set_banner(&mut self, info: BannerAccountInfo) {
        if !self.banner_shown {
            let banner = Banner::new(info.clone());
            let lines = banner.to_lines();
            let banner_text = lines
                .iter()
                .map(|l| {
                    l.spans
                        .iter()
                        .map(|s| s.content.to_string())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            self.chat.push_system(banner_text);
            self.banner_shown = true;
        }
        self.banner_info = Some(info);
    }

    /// Check for recent sessions and offer to resume on startup.
    pub fn check_resume_on_startup(&mut self) {
        let sessions_dir = std::path::Path::new(".openanalyst").join("sessions");
        if !sessions_dir.exists() {
            return;
        }
        let mut entries: Vec<_> = std::fs::read_dir(&sessions_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();
        if entries.is_empty() {
            return;
        }
        // Sort by modified time (newest first)
        entries.sort_by(|a, b| {
            b.metadata()
                .and_then(|m| m.modified())
                .ok()
                .cmp(&a.metadata().and_then(|m| m.modified()).ok())
        });
        let newest = &entries[0];
        let name = newest.file_name();
        let size = newest.metadata().map(|m| m.len()).unwrap_or(0);
        if size > 0 {
            self.chat.push_system(format!(
                "Recent session available: {} ({:.1} KB)\n\
                 Type /resume {} to continue, or start a new conversation.",
                name.to_string_lossy(),
                size as f64 / 1024.0,
                name.to_string_lossy()
            ));
        }
    }

    /// Toggle sidebar visibility.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
        if !self.sidebar_visible && self.focus == PanelId::Sidebar {
            self.focus = PanelId::Input;
        }
    }

    /// Cycle focus between panels.
    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            PanelId::Input => PanelId::Chat,
            PanelId::Chat => {
                if self.sidebar_visible {
                    PanelId::Sidebar
                } else {
                    PanelId::Input
                }
            }
            PanelId::Sidebar | PanelId::AgentPanel => PanelId::Input,
        };
        // Entering chat focus enables scroll mode
        self.scroll_mode = self.focus == PanelId::Chat;
    }

    /// Cancel all running agents.
    pub fn cancel_current_agent(&mut self) {
        if self.is_streaming {
            self.chat.finish_assistant();
            self.chat.push_system("Request cancelled.".to_string());
            self.status_bar.phase = AgentPhase::Done;
            self.is_streaming = false;
            if let Some(start) = self.turn_start.take() {
                self.status_bar.elapsed = start.elapsed();
            }
            // Cancel all known agents in the orchestrator
            let agent_ids: Vec<String> = self.sidebar_state.agents
                .iter()
                .filter(|a| a.status == AgentStatus::Running)
                .map(|a| a.agent_id.clone())
                .collect();
            let tx = self.action_tx.clone();
            tokio::spawn(async move {
                for id in agent_ids {
                    let _ = tx.send(Action::CancelAgent(id)).await;
                }
                // Always cancel "primary" as fallback
                let _ = tx.send(Action::CancelAgent("primary".to_string())).await;
            });
        }
    }

    /// Initiate graceful exit.
    pub fn request_exit(&mut self) {
        if self.is_streaming {
            // First Ctrl+C cancels the running agent
            self.cancel_current_agent();
        } else {
            // Direct quit (session saving handled by the caller)
            self.should_quit = true;
        }
    }

    /// Run a prompt in the background — sends to orchestrator and tracks in sidebar.
    pub fn run_in_background(&mut self, text: String) {
        use crate::panels::sidebar::{BackgroundTask, BackgroundTaskStatus};

        let task_id = format!("bg-{}", self.sidebar_state.background_tasks.len() + 1);
        let preview = if text.chars().count() > 30 {
            let t: String = text.chars().take(27).collect();
            format!("{t}...")
        } else {
            text.clone()
        };

        self.sidebar_state.background_tasks.push(BackgroundTask {
            id: task_id.clone(),
            prompt_preview: preview,
            status: BackgroundTaskStatus::Running,
        });

        self.chat.push_system(format!("Running in background: {}", &text[..text.len().min(60)]));

        // Send to orchestrator as a regular prompt — orchestrator handles the rest
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(Action::SubmitPrompt(text)).await;
        });
    }

    /// Submit user input — detects `/` commands and routes accordingly.
    /// Slash commands always execute immediately (even mid-stream).
    /// Regular prompts are queued if streaming, sent immediately otherwise.
    pub fn submit_prompt(&mut self, text: String) {
        // Slash commands always execute immediately — even mid-stream
        if text.starts_with('/') {
            if crate::slash_commands::handle_slash_command(self, &text) {
                return;
            }
        }

        // Regular prompts: queue if streaming, send immediately otherwise
        if self.is_streaming {
            self.pending_queue.push(text.clone());
            self.chat.push_system(format!("[queued] {}", truncate_display(&text, 60)));
            return;
        }

        self.submit_prompt_internal(text);
    }

    /// Send a prompt directly to the orchestrator (used by slash commands too).
    pub fn submit_prompt_internal(&mut self, text: String) {
        if !self.chat.messages.last().is_some_and(|m| matches!(m, ChatMessage::User { .. })) {
            self.chat.push_user(text.clone());
        }
        self.chat.start_assistant();
        self.turn_start = Some(Instant::now());
        self.is_streaming = true;
        self.status_bar.phase = AgentPhase::Thinking;
        self.chat.auto_scroll = true;

        // Clear model_override after use (will be passed via Action when orchestrator supports it)
        let _model_override = self.model_override.take();
        let _effort = self.effort;

        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(Action::SubmitPrompt(text)).await;
        });
    }

    /// Drain the pending queue — called when streaming ends.
    fn drain_pending_queue(&mut self) {
        if let Some(next) = self.pending_queue.first().cloned() {
            self.pending_queue.remove(0);
            self.submit_prompt_internal(next);
        }
    }

    /// Resolve a permission dialog.
    pub fn resolve_permission(&self, request_id: String, allow: bool) {
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            let _ = tx
                .send(Action::PermissionResponse { request_id, allow })
                .await;
        });
    }

    /// Handle a backend UI event.
    pub fn handle_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::StreamDelta { text, .. } => {
                self.chat.push_delta(&text);
            }
            UiEvent::StreamEnd { .. } => {
                self.chat.finish_assistant();
                self.status_bar.phase = AgentPhase::Done;
                self.is_streaming = false;
                if let Some(start) = self.turn_start.take() {
                    self.status_bar.elapsed = start.elapsed();
                }
                // Auto-send next queued prompt
                self.drain_pending_queue();
            }
            UiEvent::ToolCallStart {
                tool_name,
                input_preview,
                ..
            } => {
                self.status_bar.phase = match tool_name.as_str() {
                    "read_file" | "Read" => AgentPhase::ReadingFile(input_preview.clone()),
                    "edit_file" | "Edit" | "write_file" | "Write" => {
                        AgentPhase::EditingFile(input_preview.clone())
                    }
                    "bash" | "Bash" => AgentPhase::RunningBash,
                    "glob_search" | "grep_search" | "Glob" | "Grep" => AgentPhase::Searching,
                    _ => AgentPhase::Thinking,
                };
                self.chat.push_tool_call(ToolCallCard {
                    tool_name: tool_name.clone(),
                    input_preview: input_preview.clone(),
                    status: ToolCallStatus::Running {
                        elapsed: Duration::ZERO,
                    },
                    output: None,
                    expanded: false,
                });
                self.sidebar_state.tool_call_count += 1;

                // Track files in sidebar
                match tool_name.as_str() {
                    "read_file" | "Read" => {
                        self.sidebar_state.track_file(input_preview, FileAction::Read);
                    }
                    "edit_file" | "Edit" => {
                        self.sidebar_state.track_file(input_preview, FileAction::Edited);
                    }
                    "write_file" | "Write" => {
                        self.sidebar_state.track_file(input_preview, FileAction::Created);
                    }
                    _ => {}
                }
            }
            UiEvent::ToolCallEnd {
                output,
                is_error,
                duration,
                ..
            } => {
                for msg in self.chat.messages.iter_mut().rev() {
                    if let ChatMessage::ToolCall { card } = msg {
                        if matches!(card.status, ToolCallStatus::Running { .. }) {
                            card.status = if is_error {
                                ToolCallStatus::Failed { duration }
                            } else {
                                ToolCallStatus::Completed { duration }
                            };
                            card.output = Some(output.clone());
                            break;
                        }
                    }
                }
                self.status_bar.phase = AgentPhase::Thinking;
            }
            UiEvent::PermissionRequest {
                request_id,
                agent_id,
                tool_name,
                input,
                required_mode,
            } => {
                self.permission_dialog = Some(PermissionDialog {
                    request_id,
                    agent_id,
                    tool_name,
                    input_preview: input,
                    required_mode,
                    selected: 0,
                });
            }
            UiEvent::UsageUpdate {
                input_tokens,
                output_tokens,
                ..
            } => {
                self.status_bar.total_tokens += u64::from(input_tokens + output_tokens);
            }
            UiEvent::AgentSpawned {
                agent_id,
                agent_type,
                task,
                ..
            } => {
                self.sidebar_state.update_agent(
                    agent_id.clone(),
                    agent_type.clone(),
                    task.clone(),
                    AgentStatus::Running,
                );
                self.chat
                    .push_system(format!("[{agent_type}] Agent spawned: {}", truncate_display(&task, 80)));
            }
            UiEvent::AgentStatusChanged { agent_id, status } => {
                if let Some(agent) = self.sidebar_state.agents.iter_mut().find(|a| a.agent_id == agent_id) {
                    agent.status = status.clone();
                }
                self.chat.push_system(format!(
                    "[Agent {agent_id}] Status: {status:?}"
                ));
            }
            UiEvent::AgentCompleted { agent_id, result } => {
                if let Some(agent) = self.sidebar_state.agents.iter_mut().find(|a| a.agent_id == agent_id) {
                    agent.status = AgentStatus::Completed;
                }
                self.chat.push_system(format!(
                    "Agent completed: {}",
                    truncate_display(&result, 120)
                ));
            }
            UiEvent::AgentFailed { agent_id, error } => {
                if let Some(agent) = self.sidebar_state.agents.iter_mut().find(|a| a.agent_id == agent_id) {
                    agent.status = AgentStatus::Failed;
                }
                self.chat.finish_assistant();
                self.is_streaming = false;
                self.chat.push_system(format!("Error: {error}"));
                self.status_bar.phase = AgentPhase::Error;
                // Auto-send next queued prompt
                self.drain_pending_queue();
            }
            UiEvent::Tick => {
                self.spinner_state.calc_next();
                if let Some(start) = &self.turn_start {
                    self.status_bar.elapsed = start.elapsed();
                }
            }
        }
    }

    /// Handle animation tick.
    pub fn tick(&mut self) {
        self.handle_ui_event(UiEvent::Tick);
    }

    /// Determine the current input mode based on app state.
    fn current_input_mode(&self) -> tui_widgets::InputMode {
        if self.is_streaming {
            // Check if we have a running agent other than primary
            if let Some(agent) = self.sidebar_state.agents.iter().find(|a| {
                a.status == AgentStatus::Running && a.agent_type != events::AgentType::Primary
            }) {
                return tui_widgets::InputMode::AgentRunning {
                    label: format!("{} — {}", agent.agent_type, truncate_display(&agent.task_summary, 40)),
                };
            }
            // Check if phase suggests planning
            if matches!(self.status_bar.phase, AgentPhase::Thinking) {
                return tui_widgets::InputMode::Streaming;
            }
            // Default streaming mode with phase info
            let phase_label = match &self.status_bar.phase {
                AgentPhase::ReadingFile(f) => format!("Reading {}", f.rsplit(['/', '\\']).next().unwrap_or(f)),
                AgentPhase::EditingFile(f) => format!("Editing {}", f.rsplit(['/', '\\']).next().unwrap_or(f)),
                AgentPhase::RunningBash => "Running bash...".to_string(),
                AgentPhase::Searching => "Searching...".to_string(),
                _ => "Working...".to_string(),
            };
            tui_widgets::InputMode::AgentRunning { label: phase_label }
        } else {
            tui_widgets::InputMode::Ready
        }
    }

    /// Render the full application.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Dynamic input height based on editor content
        let input_height = self.input_state.line_count();
        let layout = compute_layout(area, self.sidebar_visible, input_height);

        // Chat panel
        self.chat.render(layout.chat, buf);

        // Sidebar (if visible)
        if let Some(sidebar_area) = layout.sidebar {
            let elapsed_secs = self.status_bar.elapsed.as_secs();
            let tokens = self.status_bar.total_tokens;
            sidebar::render_sidebar(
                &self.sidebar_state,
                tokens,
                elapsed_secs,
                sidebar_area,
                buf,
            );
        }

        // Status line (full width, with hints)
        let hints = build_status_hints(self.is_streaming, self.scroll_mode, self.sidebar_visible);
        let mut status = self.status_bar.clone();
        status.hints = hints;
        status.render(layout.status, buf);

        // Input box with mode-aware styling
        let input_mode = self.current_input_mode();
        let input = InputBox::default().mode(input_mode);
        input.render_with_state(layout.input, buf, &mut self.input_state);

        // Permission dialog overlay (if active)
        if let Some(dialog) = self.permission_dialog.clone() {
            dialog.render(area, buf);
        }
    }
}

/// Build the right-aligned keybinding hints for the status bar.
fn build_status_hints(is_streaming: bool, scroll_mode: bool, sidebar_visible: bool) -> String {
    let mut hints = Vec::new();
    if scroll_mode {
        hints.push("Esc:input");
        hints.push("j/k:scroll");
        hints.push("Enter:expand");
    } else {
        hints.push("Esc:scroll");
    }
    if is_streaming {
        hints.push("Ctrl+C:cancel");
    } else {
        hints.push("Ctrl+C:quit");
    }
    if sidebar_visible {
        hints.push("Ctrl+B:hide");
    } else {
        hints.push("Ctrl+B:sidebar");
    }
    hints.join(" · ")
}

/// UTF-8 safe string truncation for display.
fn truncate_display(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else if max_chars > 3 {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{truncated}...")
    } else {
        s.chars().take(max_chars).collect()
    }
}
