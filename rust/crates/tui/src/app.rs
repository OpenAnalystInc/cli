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
    /// Double Ctrl+C to quit (Claude Code behavior).
    pub exit_pending: bool,

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

    // Session persistence — unique ID for this session
    pub session_id: String,
}

impl App {
    /// Create a new App with smart per-action routing based on the user's model.
    pub fn new(ui_rx: UiEventRx, action_tx: ActionTx, default_model: &str) -> Self {
        let mut app = Self {
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
            exit_pending: false,
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
            session_id: generate_session_id(),
        };
        // Discover project files and plans on startup for sidebar
        app.sidebar_state.discover_project_files();
        app.sidebar_state.discover_plans();
        app
    }

    /// Set banner info and inject the banner into the chat.
    pub fn set_banner(&mut self, info: BannerAccountInfo) {
        if !self.banner_shown {
            let banner = Banner::new(info.clone());
            let lines = banner.to_lines();
            self.chat.push_banner(lines);
            self.banner_shown = true;
        }
        self.banner_info = Some(info);
    }

    /// Check for recent sessions and offer to resume on startup.
    pub fn check_resume_on_startup(&mut self) {
        let latest = std::path::Path::new(".openanalyst").join("sessions").join("session-latest.json");
        if !latest.exists() {
            return;
        }
        let size = std::fs::metadata(&latest).map(|m| m.len()).unwrap_or(0);
        if size == 0 {
            return;
        }

        // Read metadata from the latest session (v3 fields)
        let content = std::fs::read_to_string(&latest).unwrap_or_default();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        let model = v.get("model").and_then(|m| m.as_str()).unwrap_or("default");
        let timestamp = v.get("timestamp").and_then(|t| t.as_str()).unwrap_or("unknown");
        let msg_count = v.get("messages").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);

        self.chat.push_system(format!(
            "Recent session available: {msg_count} messages ({:.1} KB)\n\
             Model: {model} | Saved: {timestamp}\n\
             Type /resume session-latest.json to continue, or start a new conversation.",
            size as f64 / 1024.0,
        ));
    }

    /// Auto-save current session to disk (v3 format with full metadata).
    pub fn auto_save_session(&self) {
        if self.chat.messages.is_empty() {
            return;
        }
        let sessions_dir = std::path::Path::new(".openanalyst").join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);

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
                ChatMessage::Banner { .. } | ChatMessage::InlineStatus { .. } => None,
            })
            .collect();

        // v3: full metadata for context preservation on resume
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp = format_timestamp(now.as_secs());

        let session = serde_json::json!({
            "version": 3,
            "session_id": self.session_id,
            "timestamp": timestamp,
            "model": self.status_bar.model_name,
            "permission_mode": self.permission_mode,
            "cwd": std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            "messages": messages,
            "tokens": self.status_bar.total_tokens,
        });

        match serde_json::to_string_pretty(&session) {
            Ok(json) => {
                // Write timestamped file
                let ts_path = sessions_dir.join(format!("{}.json", &self.session_id));
                let _ = std::fs::write(&ts_path, &json);
                // Write session-latest.json as a copy (no symlinks — Windows compat)
                let latest_path = sessions_dir.join("session-latest.json");
                let _ = std::fs::write(&latest_path, &json);
                // Prune old sessions (keep latest 20)
                prune_old_sessions(&sessions_dir, 20);
            }
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

    /// Initiate graceful exit (Claude Code behavior: double Ctrl+C to quit).
    pub fn request_exit(&mut self) {
        if self.is_streaming {
            // First Ctrl+C cancels the running agent
            self.cancel_current_agent();
        } else if self.exit_pending {
            // Second Ctrl+C → actually quit
            self.should_quit = true;
        } else {
            // First Ctrl+C when idle → warn, require confirmation
            self.exit_pending = true;
            self.chat.push_system("Press Ctrl+C again to exit.".to_string());
        }
    }

    /// Reset exit confirmation (call on any non-Ctrl+C input).
    pub fn clear_exit_pending(&mut self) {
        self.exit_pending = false;
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

        // Intercept common control words — execute locally without calling AI
        let lower = text.trim().to_ascii_lowercase();
        match lower.as_str() {
            "exit" | "quit" | "q" => {
                self.chat.push_user(text);
                self.chat.push_system("Saving session and exiting...".to_string());
                self.should_quit = true;
                return;
            }
            "clear" => {
                self.chat.messages.clear();
                self.chat.scroll_offset = 0;
                self.chat.focused_message = None;
                self.status_bar.total_tokens = 0;
                self.chat.push_system("Session cleared.".to_string());
                return;
            }
            "help" => {
                self.chat.push_user(text);
                let help = commands::render_slash_command_help();
                self.chat.push_system(help);
                return;
            }
            _ => {}
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
                self.status_bar.phase = AgentPhase::Idle;
                self.is_streaming = false;
                // Inject inline status at end of response (like Claude Code)
                if let Some(start) = self.turn_start.take() {
                    let elapsed = start.elapsed();
                    self.status_bar.elapsed = elapsed;
                    let time_str = format_duration_short(&elapsed);
                    let token_str = format_tokens_short(self.status_bar.total_tokens);
                    self.chat.push_inline_status(format!(
                        "✓ Done ({time_str} · ↓ {token_str} tokens)"
                    ), false);
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
                    diff: None,
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
                diff,
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
                            card.diff = diff.clone();
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
                // Inject inline error status at end of response
                if let Some(start) = self.turn_start.take() {
                    let elapsed = start.elapsed();
                    self.status_bar.elapsed = elapsed;
                    let time_str = format_duration_short(&elapsed);
                    self.chat.push_inline_status(format!(
                        "✗ Error ({time_str} · ↓ {} tokens)", format_tokens_short(self.status_bar.total_tokens)
                    ), true);
                }
                self.status_bar.phase = AgentPhase::Idle;
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

    /// Get the context tag for the input area (git branch, active agent, etc.).
    fn get_context_tag(&self) -> Option<String> {
        // Priority 1: Active agent name
        if let Some(agent) = self.sidebar_state.agents.iter().find(|a| {
            a.status == events::AgentStatus::Running
        }) {
            return Some(format!("{}", agent.agent_type));
        }

        // Priority 2: Git branch name (cached per render — fast enough for TUI)
        static CACHED_BRANCH: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
        let branch = CACHED_BRANCH.get_or_init(|| {
            std::process::Command::new("git")
                .args(["branch", "--show-current"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                        if s.is_empty() { None } else { Some(s) }
                    } else {
                        None
                    }
                })
        });

        branch.clone()
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

        // Chat panel — with focus-aware border
        let chat_focused = self.focus == PanelId::Chat || self.scroll_mode;
        self.chat.render_with_focus(layout.chat, buf, chat_focused);

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

        // Status line (full width, with hints + animated spinner color)
        let hints = build_status_hints(self.is_streaming, self.scroll_mode, self.sidebar_visible);
        let mut status = self.status_bar.clone();
        status.hints = hints;
        status.spinner_color = if self.is_streaming { Some(self.spinner_state.current_color()) } else { None };
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
            let context_tag = self.get_context_tag();
            let input = InputBox::default().mode(input_mode).context_tag(context_tag);
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
    hints.push("Ctrl+B:background");
    if sidebar_visible {
        hints.push("F2:hide");
    } else {
        hints.push("F2:sidebar");
    }
    hints.join(" · ")
}

/// Format duration as short string for inline status.
fn format_duration_short(d: &Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 { format!("{secs}s") }
    else { format!("{}m {:02}s", secs / 60, secs % 60) }
}

/// Format token count as short string.
fn format_tokens_short(tokens: u64) -> String {
    if tokens < 1_000 { format!("{tokens}") }
    else if tokens < 1_000_000 { format!("{:.1}k", tokens as f64 / 1_000.0) }
    else { format!("{:.1}M", tokens as f64 / 1_000_000.0) }
}

/// Generate a unique session ID based on timestamp.
fn generate_session_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format_timestamp(now.as_secs()).replace([' ', ':'], "-").replace(',', "")
}

/// Format a Unix timestamp into "YYYY-MM-DD HH:MM:SS" (UTC-like, no TZ dependency).
fn format_timestamp(epoch_secs: u64) -> String {
    // Simple epoch → date conversion without chrono dependency
    let secs_per_day: u64 = 86400;
    let days = epoch_secs / secs_per_day;
    let time_of_day = epoch_secs % secs_per_day;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since epoch to Y-M-D (simplified Gregorian)
    let mut y = 1970u64;
    let mut remaining_days = days;
    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if remaining_days < days_in_year { break; }
        remaining_days -= days_in_year;
        y += 1;
    }
    let mut m = 1u64;
    let days_in_months: [u64; 12] = [31, if is_leap_year(y) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for dim in &days_in_months {
        if remaining_days < *dim { break; }
        remaining_days -= dim;
        m += 1;
    }
    let d = remaining_days + 1;

    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}:{seconds:02}")
}

fn is_leap_year(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Prune old session files, keeping the most recent `keep` timestamped sessions.
fn prune_old_sessions(dir: &std::path::Path, keep: usize) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            s.ends_with(".json") && s != "session-latest.json"
        })
        .collect();
    if entries.len() <= keep {
        return;
    }
    // Sort oldest first by modified time
    entries.sort_by(|a, b| {
        a.metadata().and_then(|m| m.modified()).ok()
            .cmp(&b.metadata().and_then(|m| m.modified()).ok())
    });
    // Delete oldest beyond keep limit
    for entry in entries.iter().take(entries.len() - keep) {
        let _ = std::fs::remove_file(entry.path());
    }
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
