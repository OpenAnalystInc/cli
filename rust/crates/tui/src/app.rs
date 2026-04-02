//! Main TUI application state and rendering.

use std::time::{Duration, Instant};

use events::{Action, ActionTx, UiEvent, UiEventRx};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use tui_widgets::status_bar::AgentPhase;
use tui_widgets::{InputBox, InputBoxState, PermissionDialog, StatusBar, ToolCallCard, ToolCallStatus};

use crate::banner::{Banner, BannerAccountInfo};
use crate::layout::compute_layout;
use crate::panels::chat::{ChatMessage, ChatPanel};

/// The main TUI application state.
pub struct App {
    // Panels
    pub chat: ChatPanel,
    pub status_bar: StatusBar,
    pub input_state: InputBoxState,

    // Modal overlays
    pub permission_dialog: Option<PermissionDialog>,

    // State
    pub scroll_mode: bool,
    pub should_quit: bool,
    pub turn_start: Option<Instant>,

    // Channels
    pub ui_rx: UiEventRx,
    pub action_tx: ActionTx,

    // Banner info
    pub banner_info: Option<BannerAccountInfo>,
    pub banner_shown: bool,

    // Spinner state
    pub spinner_state: tui_widgets::spinner::SpinnerState,
}

impl App {
    /// Create a new App.
    pub fn new(ui_rx: UiEventRx, action_tx: ActionTx) -> Self {
        Self {
            chat: ChatPanel::default(),
            status_bar: StatusBar::default(),
            input_state: InputBoxState::default(),
            permission_dialog: None,
            scroll_mode: false,
            should_quit: false,
            turn_start: None,
            ui_rx,
            action_tx,
            banner_info: None,
            banner_shown: false,
            spinner_state: tui_widgets::spinner::SpinnerState::default(),
        }
    }

    /// Set banner info and inject the banner into the chat.
    pub fn set_banner(&mut self, info: BannerAccountInfo) {
        if !self.banner_shown {
            let banner = Banner::new(info.clone());
            let lines = banner.to_lines();
            // Add banner as a system-style message at the start of chat
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

    /// Submit a prompt to the orchestrator.
    pub fn submit_prompt(&mut self, text: String) {
        self.chat.push_user(text.clone());
        self.chat.start_assistant();
        self.turn_start = Some(Instant::now());
        self.status_bar.phase = AgentPhase::Thinking;
        self.chat.auto_scroll = true;

        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(Action::SubmitPrompt(text)).await;
        });
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
                if let Some(start) = self.turn_start.take() {
                    self.status_bar.elapsed = start.elapsed();
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
                    tool_name,
                    input_preview,
                    status: ToolCallStatus::Running {
                        elapsed: Duration::ZERO,
                    },
                    output: None,
                    expanded: false,
                });
            }
            UiEvent::ToolCallEnd {
                output,
                is_error,
                duration,
                ..
            } => {
                // Search backwards for the most recent ToolCall to update
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
                agent_type, task, ..
            } => {
                self.chat
                    .push_system(format!("[{agent_type}] Agent spawned: {}", truncate_display(&task, 80)));
            }
            UiEvent::AgentStatusChanged { agent_id, status } => {
                self.chat.push_system(format!(
                    "[Agent {agent_id}] Status: {status:?}"
                ));
            }
            UiEvent::AgentCompleted { result, .. } => {
                self.chat.push_system(format!(
                    "Agent completed: {}",
                    truncate_display(&result, 120)
                ));
            }
            UiEvent::AgentFailed { error, .. } => {
                self.chat.finish_assistant();
                self.chat.push_system(format!("Error: {error}"));
                self.status_bar.phase = AgentPhase::Error;
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

    /// Render the full application.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let layout = compute_layout(area);

        // Chat panel
        self.chat.render(layout.chat, buf);

        // Status line
        self.status_bar.clone().render(layout.status, buf);

        // Input box
        let input = InputBox::default();
        input.render_with_state(layout.input, buf, &mut self.input_state);

        // Permission dialog overlay (if active)
        if let Some(dialog) = self.permission_dialog.clone() {
            dialog.render(area, buf);
        }
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
