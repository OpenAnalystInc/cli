//! Main TUI application state and rendering.

use std::time::{Duration, Instant};

use events::{Action, ActionTx, AgentStatus, PanelId, UiEvent, UiEventRx};
use orchestrator::router::ModelRouter;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget};

use tui_widgets::status_bar::AgentPhase;
use tui_widgets::{InputBox, InputBoxState, PermissionDialog, StatusBar, ToolCallCard, ToolCallStatus};

use crate::autocomplete::{InputHistory, SlashSuggestions};
use crate::banner::{Banner, BannerAccountInfo};
use crate::layout::compute_layout;
use crate::panels::chat::{ChatMessage, ChatPanel};
use crate::panels::sidebar::{self, BackgroundTaskStatus, FileAction, SidebarState};

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
    /// Set after Ctrl+C cancel — prevents StreamEnd from draining pending queue.
    pub cancelled: bool,

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

    // Smart per-action model router — routes prompts to (model, effort) by category
    pub router: ModelRouter,

    // Per-prompt model override — used for one prompt then reverts
    pub model_override: Option<String>,

    // Current permission mode (tracked for display, orchestrator has canonical copy)
    pub permission_mode: String,

    // Slash command autocomplete
    pub suggestions: SlashSuggestions,

    // Input history
    pub history: InputHistory,

    // Voice input state
    pub voice: crate::voice::VoiceState,
}

impl App {
    /// Create a new App with smart per-action routing based on the user's model.
    pub fn new(ui_rx: UiEventRx, action_tx: ActionTx, default_model: &str) -> Self {
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
            cancelled: false,
            ui_rx,
            action_tx,
            banner_info: None,
            banner_shown: false,
            spinner_state: tui_widgets::spinner::SpinnerState::default(),
            pending_queue: Vec::new(),
            router: ModelRouter::from_default_model(default_model),
            model_override: None,
            permission_mode: "danger-full-access".to_string(),
            suggestions: SlashSuggestions::default(),
            history: InputHistory::default(),
            voice: crate::voice::VoiceState::default(),
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

    /// Auto-save current session to disk.
    pub fn auto_save_session(&self) {
        if self.chat.messages.is_empty() {
            return;
        }
        let sessions_dir = std::path::Path::new(".openanalyst").join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);

        // Use a stable filename based on the startup time
        let path = sessions_dir.join("session-latest.json");
        let messages: Vec<serde_json::Value> = self
            .chat
            .messages
            .iter()
            .filter_map(|msg| match msg {
                ChatMessage::User { text } => Some(serde_json::json!({
                    "role": "user",
                    "content": text,
                })),
                ChatMessage::Assistant { markdown, .. } => {
                    let raw = markdown.raw();
                    if raw.is_empty() {
                        None
                    } else {
                        Some(serde_json::json!({
                            "role": "assistant",
                            "content": raw,
                        }))
                    }
                }
                ChatMessage::ToolCall { card } => Some(serde_json::json!({
                    "role": "tool_call",
                    "tool_name": card.tool_name,
                    "input": card.input_preview,
                    "output": card.output,
                    "status": format!("{:?}", card.status),
                })),
                ChatMessage::System { text } => Some(serde_json::json!({
                    "role": "system",
                    "content": text,
                })),
                ChatMessage::FileOutput { path, description, .. } => Some(serde_json::json!({
                    "role": "file_output",
                    "path": path,
                    "description": description,
                })),
            })
            .collect();

        let session = serde_json::json!({
            "version": 2,
            "messages": messages,
            "tokens": self.status_bar.total_tokens,
        });

        match serde_json::to_string_pretty(&session) {
            Ok(json) => { let _ = std::fs::write(&path, json); }
            Err(e) => {
                eprintln!("[auto_save_session] Failed to serialize session: {e}");
            }
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

    /// Cancel all running agents. Sets `cancelled` flag to prevent
    /// StreamEnd from draining the pending queue (race condition fix).
    pub fn cancel_current_agent(&mut self) {
        if self.is_streaming {
            self.chat.finish_assistant();
            self.chat.push_system("Request cancelled.".to_string());
            self.status_bar.phase = AgentPhase::Done;
            self.is_streaming = false;
            self.cancelled = true; // Prevent StreamEnd from draining queue
            // Clear pending queue — user cancelled, don't auto-send
            self.pending_queue.clear();
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
                    if tx.send(Action::CancelAgent(id)).await.is_err() {
                        eprintln!("[tui] orchestrator channel closed");
                    }
                }
                // Always cancel "primary" as fallback
                if tx.send(Action::CancelAgent("primary".to_string())).await.is_err() {
                    eprintln!("[tui] orchestrator channel closed");
                }
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
            if tx.send(Action::SubmitPrompt { text, effort_budget: None, model_override: None }).await.is_err() {
                eprintln!("[tui] orchestrator channel closed");
            }
        });
    }

    /// Submit user input — detects `/` commands and routes accordingly.
    /// Slash commands always execute immediately (even mid-stream).
    /// Regular prompts are queued if streaming, sent immediately otherwise.
    pub fn submit_prompt(&mut self, text: String) {
        // Ignore empty or whitespace-only input
        if text.trim().is_empty() {
            return;
        }

        // Record in history
        self.history.push(text.clone());
        // Dismiss autocomplete
        self.suggestions.dismiss();

        // Detect chained slash commands (e.g., "/bughunter /commit /pr")
        if text.starts_with('/') {
            let slash_count = text.matches(" /").count() + 1;
            if slash_count >= 2 {
                // Multiple commands — route to MOE or sequential
                self.chat.push_user(text.clone());
                let commands: Vec<String> = text.split(" /")
                    .enumerate()
                    .map(|(i, part)| if i == 0 { part.to_string() } else { format!("/{part}") })
                    .collect();
                if commands.len() >= 3 {
                    self.chat.push_system(format!(
                        "[MOE] Dispatching {} commands as parallel agents...", commands.len()
                    ));
                } else {
                    self.chat.push_system(format!(
                        "Running {} commands sequentially...", commands.len()
                    ));
                }
                let tx = self.action_tx.clone();
                tokio::spawn(async move {
                    if tx.send(Action::MoeDispatch { commands }).await.is_err() {
                        eprintln!("[tui] orchestrator channel closed");
                    }
                });
                self.is_streaming = true;
                self.turn_start = Some(Instant::now());
                self.status_bar.phase = AgentPhase::Thinking;
                return;
            }

            // Single slash command — handle normally
            if crate::slash_commands::handle_slash_command(self, &text) {
                return;
            }
        }

        // Mid-task skill injection: slash command while agents are running
        if self.is_streaming && text.starts_with('/') {
            // Inject as a new skill agent
            self.chat.push_user(text.clone());
            self.chat.push_system(format!("[skill injection] {}", &text));
            let tx = self.action_tx.clone();
            let cmd = text.clone();
            tokio::spawn(async move {
                if tx.send(Action::InjectSkill(cmd)).await.is_err() {
                    eprintln!("[tui] orchestrator channel closed");
                }
            });
            return;
        }

        // Regular prompts: queue if streaming, send immediately otherwise
        if self.is_streaming {
            const MAX_PENDING_QUEUE: usize = 50;
            if self.pending_queue.len() >= MAX_PENDING_QUEUE {
                self.chat.push_system(format!(
                    "Queue full ({MAX_PENDING_QUEUE} items) — prompt dropped. Wait for current turn to finish."
                ));
                return;
            }
            self.pending_queue.push(text.clone());
            self.chat.push_system(format!("[queued] {}", truncate_display(&text, 60)));
            return;
        }

        self.submit_prompt_internal(text);
    }

    /// Send a prompt directly to the orchestrator (used by slash commands too).
    /// Uses the smart per-action router to determine model + effort from prompt content.
    pub fn submit_prompt_internal(&mut self, text: String) {
        if !self.chat.messages.last().is_some_and(|m| matches!(m, ChatMessage::User { .. })) {
            self.chat.push_user(text.clone());
        }
        self.chat.start_assistant();
        self.turn_start = Some(Instant::now());
        self.is_streaming = true;
        self.status_bar.phase = AgentPhase::Thinking;
        self.chat.auto_scroll = true;

        // Smart routing: classify prompt → pick (model, effort) from routing table
        let route = self.router.route_prompt(&text);
        let model_override = self.model_override.take().or(Some(route.model));
        let effort_budget = Some(route.effort_budget);

        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            if tx.send(Action::SubmitPrompt { text, effort_budget, model_override }).await.is_err() {
                eprintln!("[tui] orchestrator channel closed");
            }
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
            if tx
                .send(Action::PermissionResponse { request_id, allow })
                .await
                .is_err()
            {
                eprintln!("[tui] orchestrator channel closed");
            }
        });
    }

    /// Handle a backend UI event.
    pub fn handle_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::StreamDelta { text, .. } => {
                self.chat.push_delta(&text);
            }
            UiEvent::StreamEnd { agent_id } => {
                self.chat.finish_assistant();
                self.status_bar.phase = AgentPhase::Done;
                self.is_streaming = false;
                if let Some(start) = self.turn_start.take() {
                    self.status_bar.elapsed = start.elapsed();
                }
                // Update background task matching this agent
                if let Some(bt) = self.sidebar_state.background_tasks.iter_mut().find(|bt| bt.id == agent_id && bt.status == BackgroundTaskStatus::Running) {
                    bt.status = BackgroundTaskStatus::Completed;
                }
                // Auto-compact if session is getting very large (>500 messages)
                if self.chat.messages.len() > 500 {
                    crate::slash_commands::auto_compact_if_needed(self);
                }
                // Auto-send next queued prompt (skip if user cancelled)
                if self.cancelled {
                    self.cancelled = false;
                } else {
                    self.drain_pending_queue();
                }
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
                // Update background task matching this agent
                if let Some(bt) = self.sidebar_state.background_tasks.iter_mut().find(|bt| bt.id == agent_id && bt.status == BackgroundTaskStatus::Running) {
                    bt.status = BackgroundTaskStatus::Failed;
                }
                // Auto-send next queued prompt (skip if user cancelled)
                if self.cancelled {
                    self.cancelled = false;
                } else {
                    self.drain_pending_queue();
                }
            }
            UiEvent::Tick => {
                self.spinner_state.calc_next();
                if let Some(start) = &self.turn_start {
                    self.status_bar.elapsed = start.elapsed();
                }
                // Auto-stop voice recording if max duration exceeded
                if self.voice.should_auto_stop() {
                    self.voice.is_recording.store(false, std::sync::atomic::Ordering::SeqCst);
                    self.chat.push_system("Voice recording auto-stopped (60s limit).".to_string());
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
            self.sidebar_state.has_focus = self.focus == PanelId::Sidebar;
            sidebar::render_sidebar(
                &self.sidebar_state,
                tokens,
                elapsed_secs,
                &self.permission_mode,
                &self.router,
                sidebar_area,
                buf,
            );
        }

        // Status line (full width, with hints)
        let hints = build_status_hints(self.is_streaming, self.scroll_mode, self.sidebar_visible);
        let mut status = self.status_bar.clone();
        status.hints = hints;
        status.render(layout.status, buf);

        // Input box — voice mode or normal
        if self.voice.is_recording.load(std::sync::atomic::Ordering::SeqCst) {
            // Voice recording mode — show VU meter instead of input box
            let voice_lines = crate::voice::render_voice_indicator(&self.voice, layout.input.width);
            let voice_block = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(50, 130, 255)))
                .title(" Voice Input ")
                .title_style(ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(50, 130, 255)).add_modifier(ratatui::style::Modifier::BOLD));
            let inner = voice_block.inner(layout.input);
            voice_block.render(layout.input, buf);
            Paragraph::new(voice_lines).render(inner, buf);
        } else {
            let input_mode = self.current_input_mode();
            let input = InputBox::default().mode(input_mode);
            input.render_with_state(layout.input, buf, &mut self.input_state);
        }

        // Slash command autocomplete overlay (above input)
        if self.suggestions.active {
            self.suggestions.render(layout.input, buf);
        }

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
