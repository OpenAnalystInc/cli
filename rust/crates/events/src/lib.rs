//! Shared event types for the OpenAnalyst TUI frontend and backend orchestrator.
//!
//! This crate defines the message protocol that flows through `tokio::sync::mpsc` channels,
//! connecting the async Ratatui event loop with the blocking `ConversationRuntime` workers.

use std::time::Duration;

use serde::{Deserialize, Serialize};

// ── Agent identification ──

/// Unique identifier for an agent instance.
pub type AgentId = String;

/// The type/role of a spawned agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    /// The primary interactive agent (always exactly one).
    Primary,
    /// Fast, read-only agent for codebase exploration.
    Explore,
    /// Read-only agent for designing implementation plans.
    Plan,
    /// General-purpose agent with full tool access.
    General,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "Primary"),
            Self::Explore => write!(f, "Explore"),
            Self::Plan => write!(f, "Plan"),
            Self::General => write!(f, "General"),
        }
    }
}

/// Lifecycle status of an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

// ── Diff information for tool call cards ──

/// A single line in a diff hunk.
#[derive(Debug, Clone)]
pub enum DiffLine {
    /// Unchanged context line.
    Context(String),
    /// Added line (shown in green).
    Added(String),
    /// Removed line (shown in red).
    Removed(String),
}

/// A contiguous diff hunk with line numbers and changes.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// Starting line number in the new file.
    pub new_start: usize,
    /// Starting line number in the old file.
    pub old_start: usize,
    /// Lines in this hunk.
    pub lines: Vec<DiffLine>,
}

/// Structured diff information for Edit/Write tool calls.
#[derive(Debug, Clone)]
pub struct DiffInfo {
    /// File path that was modified.
    pub file_path: String,
    /// Total number of lines added.
    pub added: usize,
    /// Total number of lines removed.
    pub removed: usize,
    /// Diff hunks with context.
    pub hunks: Vec<DiffHunk>,
}

// ── Panel identification ──

/// Identifies a focusable panel in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Chat,
    Input,
    Sidebar,
    AgentPanel,
}

// ── UI-bound events (backend → TUI) ──

/// Events emitted by the orchestrator/agent workers and consumed by the TUI render loop.
#[derive(Debug, Clone)]
pub enum UiEvent {
    // ── Streaming ──
    /// A chunk of assistant text arrived.
    StreamDelta {
        agent_id: AgentId,
        text: String,
    },
    /// The assistant finished streaming for this turn.
    StreamEnd {
        agent_id: AgentId,
    },

    // ── Tool calls ──
    /// A tool execution is starting.
    ToolCallStart {
        agent_id: AgentId,
        call_id: String,
        tool_name: String,
        input_preview: String,
    },
    /// A tool execution completed.
    ToolCallEnd {
        agent_id: AgentId,
        call_id: String,
        output: String,
        is_error: bool,
        duration: Duration,
        /// Structured diff info for Edit/Write tools (renders as rich diff in TUI).
        diff: Option<DiffInfo>,
    },

    // ── Permissions ──
    /// The backend needs the user to approve a tool invocation.
    PermissionRequest {
        request_id: String,
        agent_id: AgentId,
        tool_name: String,
        input: String,
        required_mode: String,
    },

    // ── Agent lifecycle ──
    /// A new agent was spawned.
    AgentSpawned {
        agent_id: AgentId,
        parent_id: Option<AgentId>,
        agent_type: AgentType,
        task: String,
    },
    /// An agent's status changed.
    AgentStatusChanged {
        agent_id: AgentId,
        status: AgentStatus,
    },
    /// An agent completed successfully.
    AgentCompleted {
        agent_id: AgentId,
        result: String,
    },
    /// An agent failed with an error.
    AgentFailed {
        agent_id: AgentId,
        error: String,
    },

    // ── Usage ──
    /// Token usage update from a streaming response.
    UsageUpdate {
        agent_id: AgentId,
        input_tokens: u32,
        output_tokens: u32,
    },

    // ── Knowledge Base ──
    /// Knowledge base query completed with structured agentic results.
    KnowledgeResult {
        query_id: i64,
        query: String,
        intent: String,
        sub_questions: Vec<SubQuestionResult>,
        answer: Option<String>,
        latency_ms: u64,
        from_cache: bool,
    },

    // ── AskUser ──
    /// The agent wants to ask the user a question via modal dialog.
    AskUserRequest {
        request_id: String,
        agent_id: AgentId,
        question: String,
        options: Option<Vec<String>>,
        default: Option<String>,
    },

    // ── Animation ──
    /// Periodic tick for spinner animations and elapsed time updates.
    Tick,
}

/// A sub-question result from the agentic RAG pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubQuestionResult {
    pub sub_question: String,
    pub intent: String,
    pub results: Vec<KbChunkResult>,
}

/// A single chunk result from the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbChunkResult {
    pub chunk_id: String,
    pub text: String,
    pub snippet: String,
    pub score: f64,
    /// Abstracted category label — e.g., "Ads Strategy", never raw course name.
    pub category_label: String,
    pub content_type: String,
    /// Citation label — e.g., "[Ads Strategy #1]".
    pub citation_label: String,
    pub has_timestamps: bool,
    pub graph_expanded: bool,
}

// ── User actions (TUI → backend) ──

/// Actions sent from the TUI to the backend orchestrator.
#[derive(Debug, Clone)]
pub enum Action {
    /// User submitted a prompt with optional effort/model overrides.
    SubmitPrompt {
        text: String,
        effort_budget: Option<u32>,
        model_override: Option<String>,
    },
    /// User responded to a permission request.
    PermissionResponse {
        request_id: String,
        allow: bool,
    },
    /// User requested cancellation of an agent.
    CancelAgent(AgentId),
    /// User issued a slash command.
    SlashCommand(String),
    /// User changed the default model — update orchestrator config + router.
    UpdateModel(String),
    /// User changed the permission mode.
    UpdatePermissions(String),
    /// MOE dispatch — multiple chained commands to run as parallel agents.
    MoeDispatch {
        /// Raw command strings (e.g., ["/bughunter src/", "/commit", "/pr"])
        commands: Vec<String>,
    },
    /// Mid-task skill injection — run a slash command while agents are working.
    InjectSkill(String),
    /// Voice transcription completed — place text in input box for review.
    VoiceTranscribed { text: String },
    /// User submitted feedback for a knowledge query.
    KnowledgeFeedback {
        query_id: i64,
        rating: String,  // "positive" | "negative" | "corrected"
        comment: String,
        correction: String,
    },
    /// User responded to an AskUser question.
    AskUserResponse {
        request_id: String,
        response: String,
    },
    /// User requested to quit.
    Quit,
}

// ── Agent spawn request (tool → orchestrator) ──

/// Request to spawn a new sub-agent, sent from the Agent tool to the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnRequest {
    /// Type of agent to spawn.
    pub agent_type: AgentType,
    /// The task/prompt for the agent.
    pub task: String,
    /// Parent agent that requested the spawn.
    pub parent_id: AgentId,
    /// Model override (if any).
    pub model: Option<String>,
}

// ── Channel type aliases ──

/// Sender for UI events (backend → TUI).
pub type UiEventTx = tokio::sync::mpsc::Sender<UiEvent>;
/// Receiver for UI events (backend → TUI).
pub type UiEventRx = tokio::sync::mpsc::Receiver<UiEvent>;

/// Sender for user actions (TUI → backend).
pub type ActionTx = tokio::sync::mpsc::Sender<Action>;
/// Receiver for user actions (TUI → backend).
pub type ActionRx = tokio::sync::mpsc::Receiver<Action>;

/// Sender for agent spawn requests (tool → orchestrator).
pub type AgentSpawnTx = tokio::sync::mpsc::Sender<AgentSpawnRequest>;
/// Receiver for agent spawn requests (tool → orchestrator).
pub type AgentSpawnRx = tokio::sync::mpsc::Receiver<AgentSpawnRequest>;

/// Sender for permission decision responses (TUI → blocked worker thread).
pub type PermissionResponseTx = tokio::sync::oneshot::Sender<bool>;
/// Receiver for permission decision responses (TUI → blocked worker thread).
pub type PermissionResponseRx = tokio::sync::oneshot::Receiver<bool>;

/// Sender for AskUser responses (TUI → blocked worker thread).
pub type AskUserResponseTx = tokio::sync::oneshot::Sender<String>;
/// Receiver for AskUser responses (TUI → blocked worker thread).
pub type AskUserResponseRx = tokio::sync::oneshot::Receiver<String>;
