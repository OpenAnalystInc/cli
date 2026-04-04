//! Main TUI application state and rendering.

use std::time::{Duration, Instant};

use events::{Action, ActionTx, AgentStatus, PanelId, UiEvent, UiEventRx};
use orchestrator::router::ModelRouter;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget};

use tui_widgets::status_bar::AgentPhase;
use tui_widgets::{InputBox, InputBoxState, PermissionDialog, PermissionLevel, StatusBar, ToolCallCard, ToolCallStatus};

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

/// Current mode of the AskUser dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskUserMode {
    /// Selecting from multiple-choice options.
    Choice,
    /// Typing a custom free-text response.
    Type,
}

/// AskUser dialog state — modal that blocks the agent until user responds.
#[derive(Debug, Clone)]
pub struct AskUserDialog {
    pub request_id: String,
    pub question: String,
    pub options: Vec<String>,
    pub default: Option<String>,
    pub selected_index: usize,
    pub text_input: String,
    /// Current interaction mode (choice or type).
    pub mode: AskUserMode,
}

impl AskUserDialog {
    pub fn new(
        request_id: String,
        question: String,
        options: Option<Vec<String>>,
        default: Option<String>,
    ) -> Self {
        let has_options = options.as_ref().map_or(false, |o| !o.is_empty());
        Self {
            request_id,
            question,
            options: options.unwrap_or_default(),
            default,
            selected_index: 0,
            text_input: String::new(),
            mode: if has_options { AskUserMode::Choice } else { AskUserMode::Type },
        }
    }

    /// Get the current response text.
    pub fn response(&self) -> String {
        match self.mode {
            AskUserMode::Choice => {
                self.options.get(self.selected_index).cloned().unwrap_or_default()
            }
            AskUserMode::Type => {
                if self.text_input.is_empty() {
                    self.default.clone().unwrap_or_default()
                } else {
                    self.text_input.clone()
                }
            }
        }
    }
}

/// Reversible TUI action for the undo stack.
#[derive(Debug, Clone)]
pub enum UndoAction {
    AddContextFile(String),
    RemoveContextFile(String),
    SelectAgent(Option<String>),
    SelectPlan(Option<usize>),
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
    pub ask_user_dialog: Option<AskUserDialog>,

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

    // Permission level for the input box mode indicator
    pub permission_level: PermissionLevel,

    // Active agent name selected from sidebar (changes input box title + system prompt)
    pub active_agent_name: Option<String>,

    // Context files added from sidebar (injected into prompts)
    pub context_files: Vec<String>,

    // Undo stack for reversible TUI actions (max 20)
    pub undo_stack: Vec<UndoAction>,

    // Timestamp of last Esc press for double-Esc undo detection
    pub last_esc_time: Option<Instant>,

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
            ask_user_dialog: None,
            sidebar_visible: false,
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
            permission_level: PermissionLevel::Default,
            active_agent_name: None,
            context_files: Vec::new(),
            undo_stack: Vec::new(),
            last_esc_time: None,
            suggestions: SlashSuggestions::default(),
            history: InputHistory::default(),
            voice: crate::voice::VoiceState::default(),
            session_id: generate_session_id(),
        };
        // Discover project files, plans, and agents on startup for sidebar
        app.sidebar_state.discover_project_files();
        app.sidebar_state.discover_plans();
        app.sidebar_state.discover_agents_from_files();
        app
    }

    /// Set banner info and inject the banner into the chat.
    pub fn set_banner(&mut self, info: BannerAccountInfo) {
        if !self.banner_shown {
            let banner = Banner::new(info.clone());
            // Use terminal width for banner so project path isn't truncated
            let width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(0);
            let lines = banner.to_lines_with_width(width);
            self.chat.push_banner(lines);
            self.banner_shown = true;
        }
        self.banner_info = Some(info);
    }

    /// Check for recent sessions — silent on startup (use /resume to load).
    pub fn check_resume_on_startup(&mut self) {
        // No-op: clean startup without session noise.
        // Users can type /resume to load a previous session.
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
                ChatMessage::KnowledgeResult { card } => Some(serde_json::json!({
                    "role": "knowledge_result",
                    "query": card.query,
                    "intent": card.intent,
                    "query_id": card.query_id,
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

    /// Cycle to the next permission level (Default → Plan → AcceptEdits → Danger → Default).
    /// Sends Action::UpdatePermissions to the orchestrator.
    /// Mode change is reflected in the input box border/icon — no chat clutter.
    pub fn cycle_permission_level(&mut self) {
        self.permission_level = self.permission_level.next();
        let mode_str = self.permission_level.to_permission_mode().to_string();
        self.permission_mode = mode_str.clone();
        // Update status bar with mode info (no system message in chat)
        self.status_bar.phase = AgentPhase::Idle;
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            if tx.send(Action::UpdatePermissions(mode_str)).await.is_err() {
                eprintln!("[tui] orchestrator channel closed");
            }
        });
    }

    /// Set the active agent from sidebar selection.
    pub fn set_active_agent(&mut self, name: Option<String>) {
        if let Some(ref agent_name) = name {
            self.chat.push_system(format!("Agent: {agent_name}"));
        } else if self.active_agent_name.is_some() {
            self.chat.push_system("Agent deselected — back to default.".to_string());
        }
        self.active_agent_name = name;
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
        // Intercept __KB_RESULT__ from /knowledge async handler — render as KnowledgeCard
        if let Some(json_str) = text.strip_prefix("__KB_RESULT__") {
            if let Ok(response) = serde_json::from_str::<serde_json::Value>(json_str) {
                let sub_questions = crate::slash_commands::parse_sub_questions_from_json(&response);
                let answer = response.get("answer")
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string());
                let latency_ms = response.get("latency_ms").and_then(|l| l.as_u64()).unwrap_or(0);
                let query_id = response.get("query_id").and_then(|q| q.as_i64()).unwrap_or(0);
                let query = response.get("query").and_then(|q| q.as_str()).unwrap_or("").to_string();
                let intent = response.get("intent").and_then(|i| i.as_str()).unwrap_or("general").to_string();
                let from_cache = response.get("from_cache").and_then(|f| f.as_bool()).unwrap_or(false);

                // Complete the running KB: Knowledge Graph tool call card
                for msg in self.chat.messages.iter_mut().rev() {
                    if let ChatMessage::ToolCall { card } = msg {
                        if card.tool_name == "KB: Knowledge Graph" {
                            if let tui_widgets::ToolCallStatus::Running { .. } = card.status {
                                card.status = tui_widgets::ToolCallStatus::Completed {
                                    duration: self.turn_start.map(|s| s.elapsed()).unwrap_or_default(),
                                };
                                card.output = Some(format!("{} results found.", sub_questions.iter().map(|sq| sq.results.len()).sum::<usize>()));
                                break;
                            }
                        }
                    }
                }

                self.handle_ui_event(events::UiEvent::KnowledgeResult {
                    query_id, query, intent, sub_questions, answer, latency_ms, from_cache,
                });
            }
            return;
        }

        if !self.chat.messages.last().is_some_and(|m| matches!(m, ChatMessage::User { .. })) {
            self.chat.push_user(text.clone());
        }
        self.chat.start_assistant();
        self.turn_start = Some(Instant::now());
        self.is_streaming = true;
        self.status_bar.phase = AgentPhase::Thinking;
        self.chat.auto_scroll = true;

        // Inject context files (read file contents, truncate large files)
        let text = if !self.context_files.is_empty() {
            let mut ctx = String::from("[Context files:\n");
            for file_path in &self.context_files {
                match std::fs::read_to_string(file_path) {
                    Ok(content) => {
                        let truncated: String = content.chars().take(8000).collect();
                        ctx.push_str(&format!("--- {} ---\n{}\n", file_path, truncated));
                        if content.len() > 8000 {
                            ctx.push_str("... (truncated)\n");
                        }
                    }
                    Err(_) => {
                        ctx.push_str(&format!("--- {} --- (could not read)\n", file_path));
                    }
                }
            }
            ctx.push_str("]\n\n");
            format!("{ctx}{text}")
        } else {
            text
        };

        // If an agent is selected, prepend its system prompt as context
        let text = if let Some(ref agent_name) = self.active_agent_name {
            if let Some(agent_def) = self.sidebar_state.available_agents.iter()
                .find(|a| a.name == *agent_name)
            {
                if !agent_def.system_prompt.is_empty() {
                    format!(
                        "[System: You are acting as the \"{}\" agent. Follow these instructions:\n{}\n]\n\n{}",
                        agent_name, agent_def.system_prompt, text
                    )
                } else {
                    text
                }
            } else {
                text
            }
        } else {
            text
        };

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

    /// Resolve an AskUser dialog — send user's response back to the blocked worker.
    pub fn resolve_ask_user(&mut self) {
        if let Some(dialog) = self.ask_user_dialog.take() {
            let response = dialog.response();
            self.chat.push_system(format!("You answered: {response}"));
            let tx = self.action_tx.clone();
            let request_id = dialog.request_id;
            tokio::spawn(async move {
                if tx
                    .send(Action::AskUserResponse { request_id, response })
                    .await
                    .is_err()
                {
                    eprintln!("[tui] orchestrator channel closed");
                }
            });
        }
    }

    /// Resolve AskUser with "chat about this" — user wants to discuss in the main chat instead.
    pub fn resolve_ask_user_chat(&mut self) {
        if let Some(dialog) = self.ask_user_dialog.take() {
            self.chat.push_system("Switching to chat — discuss your answer below.".to_string());
            let tx = self.action_tx.clone();
            let request_id = dialog.request_id;
            let question = dialog.question;
            tokio::spawn(async move {
                let response = format!(
                    "User chose to discuss this in chat instead of selecting an option. \
                     The question was: \"{question}\". \
                     Please ask the user in your next response and wait for their reply."
                );
                if tx
                    .send(Action::AskUserResponse { request_id, response })
                    .await
                    .is_err()
                {
                    eprintln!("[tui] orchestrator channel closed");
                }
            });
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
                    self.chat.push_inline_status(format!(
                        "Worked for {time_str}"
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
            UiEvent::AskUserRequest {
                request_id,
                question,
                options,
                default,
                ..
            } => {
                self.ask_user_dialog = Some(AskUserDialog::new(
                    request_id, question, options, default,
                ));
                self.chat.push_system("OpenAnalyst is asking you a question...".to_string());
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
                // Only show agent status in chat for non-primary agents
                // (Primary is the default internal agent — no need to announce it)
                if agent_type != events::AgentType::Primary {
                    self.chat
                        .push_system(format!("[{agent_type}] {}", truncate_display(&task, 80)));
                }
            }
            UiEvent::AgentStatusChanged { agent_id, status } => {
                let is_primary = self.sidebar_state.agents.iter()
                    .find(|a| a.agent_id == agent_id)
                    .map_or(false, |a| a.agent_type == events::AgentType::Primary);
                if let Some(agent) = self.sidebar_state.agents.iter_mut().find(|a| a.agent_id == agent_id) {
                    agent.status = status.clone();
                }
                // Suppress status messages for the primary agent
                if !is_primary {
                    self.chat.push_system(format!(
                        "[Agent] Status: {status:?}"
                    ));
                }
            }
            UiEvent::AgentCompleted { agent_id, result } => {
                let is_primary = self.sidebar_state.agents.iter()
                    .find(|a| a.agent_id == agent_id)
                    .map_or(false, |a| a.agent_type == events::AgentType::Primary);
                if let Some(agent) = self.sidebar_state.agents.iter_mut().find(|a| a.agent_id == agent_id) {
                    agent.status = AgentStatus::Completed;
                }
                if !is_primary {
                    self.chat.push_system(format!(
                        "Agent completed: {}",
                        truncate_display(&result, 120)
                    ));
                }
            }
            UiEvent::AgentFailed { agent_id, error } => {
                if let Some(agent) = self.sidebar_state.agents.iter_mut().find(|a| a.agent_id == agent_id) {
                    agent.status = AgentStatus::Failed;
                }
                self.chat.finish_assistant();
                self.is_streaming = false;
                // Show clean error — strip technical details
                let err_msg = format!("{error}");
                let clean_error = if err_msg.len() > 120 {
                    format!("{}...", &err_msg[..120])
                } else {
                    err_msg
                };
                self.chat.push_system(format!("Error: {clean_error}"));
                // Update elapsed timer (skip redundant inline error — system message is sufficient)
                if let Some(start) = self.turn_start.take() {
                    let elapsed = start.elapsed();
                    self.status_bar.elapsed = elapsed;
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
            UiEvent::KnowledgeResult {
                query_id, query, intent, sub_questions, answer, latency_ms, from_cache,
            } => {
                self.status_bar.phase = AgentPhase::Idle;
                self.is_streaming = false;
                if let Some(start) = self.turn_start.take() {
                    self.status_bar.elapsed = start.elapsed();
                }

                // Build tabbed KnowledgeCard from sub-question results
                let tabs: Vec<tui_widgets::KnowledgeTab> = sub_questions.iter().map(|sq| {
                    tui_widgets::KnowledgeTab {
                        sub_question: sq.sub_question.clone(),
                        intent: sq.intent.clone(),
                        results: sq.results.iter().map(|r| {
                            tui_widgets::KbResultEntry {
                                category_label: r.category_label.clone(),
                                snippet: r.snippet.clone(),
                                score: r.score,
                                citation_label: r.citation_label.clone(),
                                graph_expanded: r.graph_expanded,
                            }
                        }).collect(),
                    }
                }).collect();

                let card = tui_widgets::KnowledgeCard {
                    query,
                    intent,
                    latency_ms,
                    tabs,
                    active_tab: 0,
                    expanded: true,
                    answer,
                    from_cache,
                    feedback_submitted: false,
                    query_id,
                };

                self.chat.push_knowledge_result(card);
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
        static CACHED_BRANCH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
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
                .unwrap_or_else(|| "No-Git".to_string())
        });

        Some(branch.clone())
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
        let hints = build_status_hints(self.is_streaming, self.scroll_mode, self.sidebar_visible, self.focus);
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
            // Shorten model name for the badge (e.g., "claude-opus-4-6" → "opus-4-6")
            let model_badge = {
                let m = &self.status_bar.model_name;
                m.strip_prefix("claude-")
                    .or_else(|| m.strip_prefix("openanalyst-"))
                    .unwrap_or(m)
                    .to_string()
            };
            let input = InputBox::default()
                .mode(input_mode)
                .permission_level(self.permission_level)
                .model_label(Some(model_badge))
                .context_tag(context_tag)
                .active_agent(self.active_agent_name.clone())
                .context_files(self.context_files.clone());
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

        // AskUser dialog overlay (if active)
        if let Some(ref dialog) = self.ask_user_dialog {
            render_ask_user_dialog(dialog, area, buf);
        }
    }
}

/// Render the AskUser dialog as a centered modal overlay.
fn render_ask_user_dialog(dialog: &AskUserDialog, area: Rect, buf: &mut Buffer) {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

    let dialog_width = area.width.min(60);
    let dialog_height = if dialog.mode == AskUserMode::Choice {
        (dialog.options.len() as u16 + 6).min(area.height.saturating_sub(4))
    } else {
        8u16.min(area.height.saturating_sub(4))
    };

    // Center the dialog
    let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // Clear the background
    Clear.render(dialog_area, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(50, 130, 255)))
        .title(Line::from(vec![
            Span::styled(" ? ", Style::default().fg(Color::Rgb(50, 130, 255)).add_modifier(Modifier::BOLD)),
            Span::styled("OpenAnalyst asks ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));

    let inner = block.inner(dialog_area);
    block.render(dialog_area, buf);

    let mut lines: Vec<Line<'_>> = Vec::new();

    // Question text (wrapped)
    lines.push(Line::from(Span::styled(
        &dialog.question,
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    match dialog.mode {
        AskUserMode::Choice => {
            // Multiple choice options
            for (i, option) in dialog.options.iter().enumerate() {
                let is_selected = i == dialog.selected_index;
                let prefix = if is_selected { " > " } else { "   " };
                let style = if is_selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Indexed(252))
                };
                lines.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("{}) {option}", i + 1), style),
                ]));
            }
            lines.push(Line::from(""));
            // Action bar: Type · Chat about this · Navigate · Select
            lines.push(Line::from(vec![
                Span::styled(" t", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(" Type  ", Style::default().fg(Color::Indexed(240))),
                Span::styled("c", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(" Chat about this  ", Style::default().fg(Color::Indexed(240))),
                Span::styled("↑↓", Style::default().fg(Color::Cyan)),
                Span::styled(" navigate  ", Style::default().fg(Color::Indexed(240))),
                Span::styled("Enter", Style::default().fg(Color::Cyan)),
                Span::styled(" select", Style::default().fg(Color::Indexed(240))),
            ]));
        }
        AskUserMode::Type => {
            // Free-text input
            let display_text = if dialog.text_input.is_empty() {
                if let Some(ref default) = dialog.default {
                    format!("{default}▍")
                } else {
                    "▍".to_string()
                }
            } else {
                format!("{}▍", dialog.text_input)
            };
            lines.push(Line::from(vec![
                Span::styled(" > ", Style::default().fg(Color::Cyan)),
                Span::styled(display_text, Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(""));
            let hint = if !dialog.options.is_empty() {
                " Type your answer · Enter submit · Esc back to options"
            } else {
                " Type your answer · Enter submit"
            };
            lines.push(Line::from(Span::styled(
                hint,
                Style::default().fg(Color::Indexed(240)),
            )));
        }
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    paragraph.render(inner, buf);
}

/// Build the right-aligned keybinding hints for the status bar.
fn build_status_hints(is_streaming: bool, scroll_mode: bool, sidebar_visible: bool, focus: events::PanelId) -> String {
    let mut hints = Vec::new();
    if focus == events::PanelId::Sidebar {
        hints.push("Esc:input");
        hints.push("Tab:section");
        hints.push("j/k:nav");
    } else if scroll_mode {
        hints.push("Esc:input");
        hints.push("j/k:scroll");
        hints.push("Enter:expand");
    } else {
        hints.push("Esc:scroll");
        if sidebar_visible {
            hints.push("Tab:sidebar");
        }
    }
    if is_streaming {
        hints.push("Ctrl+C:cancel");
    } else {
        hints.push("Ctrl+C:quit");
    }
    hints.push("Ctrl+B:bg");
    hints.push("Ctrl+P:mode");
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
