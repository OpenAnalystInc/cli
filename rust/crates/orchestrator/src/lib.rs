//! Multi-agent orchestrator for the OpenAnalyst TUI.
//!
//! Manages agent lifecycle, bridges the sync `ConversationRuntime` to the async TUI
//! via channel-based `ApiClient`, `ToolExecutor`, and `PermissionPrompter` implementations.
//!
//! Smart model routing: automatically selects the optimal model per agent type —
//! cheap/fast models for exploration, balanced for planning, capable for coding.

pub mod agent;
pub mod autonomous;
pub mod context;
pub mod knowledge;
pub mod loop_detection;
pub mod moe;
pub mod registry;
pub mod router;
pub mod worker;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use events::{
    Action, AgentSpawnRequest, AgentSpawnRx, AgentStatus, AgentType,
    UiEvent, UiEventTx, ActionRx,
};
use runtime::{CompactionConfig, PermissionMode, Session};
use tokio::sync::Mutex;

use crate::registry::AgentRegistry;
use crate::router::{ActionCategory, ModelRouter, ModelTier};

/// Configuration for the orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub model: String,
    pub permission_mode: PermissionMode,
    pub allowed_tools: Option<BTreeSet<String>>,
    pub cwd: PathBuf,
    pub system_prompt: Vec<String>,
    /// Maximum turns per agent before forced stop (loop detection safety net).
    pub max_turns: Option<u32>,
}

/// The main orchestrator that manages all agents and routes events.
pub struct AgentOrchestrator {
    config: OrchestratorConfig,
    registry: Arc<Mutex<AgentRegistry>>,
    router: ModelRouter,
    ui_tx: UiEventTx,
    action_rx: ActionRx,
    agent_spawn_rx: Option<AgentSpawnRx>,
    /// Persistent session shared across all turns — enables multi-turn conversation memory.
    session: Arc<Mutex<Session>>,
}

impl AgentOrchestrator {
    /// Create a new orchestrator with smart model routing.
    #[must_use]
    pub fn new(
        config: OrchestratorConfig,
        ui_tx: UiEventTx,
        action_rx: ActionRx,
        agent_spawn_rx: Option<AgentSpawnRx>,
    ) -> Self {
        let router = ModelRouter::from_default_model(&config.model);
        Self {
            config,
            registry: Arc::new(Mutex::new(AgentRegistry::new())),
            router,
            ui_tx,
            action_rx,
            agent_spawn_rx,
            session: Arc::new(Mutex::new(Session::default())),
        }
    }

    /// Run the orchestrator event loop.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                action = self.action_rx.recv() => {
                    match action {
                        Some(Action::SubmitPrompt { text, effort_budget, model_override }) => {
                            self.submit_to_primary(text, effort_budget, model_override).await;
                        }
                        Some(Action::PermissionResponse { request_id, allow }) => {
                            self.resolve_permission(&request_id, allow).await;
                        }
                        Some(Action::CancelAgent { agent_id }) => {
                            let id = agent_id.unwrap_or_default();
                            self.cancel_agent(&id).await;
                        }
                        Some(Action::UpdateModel { model }) => {
                            self.config.model = model.clone();
                            self.router = ModelRouter::from_default_model(&model);
                        }
                        Some(Action::UpdatePermissions { mode }) => {
                            if let Some(pm) = parse_permission_mode(&mode) {
                                self.config.permission_mode = pm;
                            }
                        }
                        Some(Action::MoeDispatch { commands }) => {
                            self.handle_moe_dispatch(commands).await;
                        }
                        Some(Action::InjectSkill { command }) => {
                            // Mid-task skill injection: spawn a new agent for this command
                            self.handle_skill_injection(command).await;
                        }
                        Some(Action::VoiceTranscribed { .. }) => {
                            // Handled directly in the TUI event loop, not here
                        }
                        Some(Action::KnowledgeFeedback { .. }) => {
                            // Handled in the TUI slash command layer, not here
                        }
                        Some(Action::AskUserResponse { request_id, response }) => {
                            self.resolve_ask_user(&request_id, response).await;
                        }
                        Some(Action::Quit) | None => break,
                        Some(Action::SlashCommand { command }) => {
                            self.handle_slash_command(command).await;
                        }
                        Some(Action::ToggleContextFile { path, action }) => {
                            let _ = self.ui_tx.send(UiEvent::SystemMessage {
                                content: format!("Context file {action}: {path}"),
                                level: "info".to_string(),
                            }).await;
                        }
                        Some(Action::ChangeRouting { category, tier }) => {
                            if let (Some(cat), Some(t)) = (
                                ActionCategory::from_str_opt(&category),
                                ModelTier::from_str_opt(&tier),
                            ) {
                                self.router.table.set_tier(cat, t);
                                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                                    content: format!("Routing updated: {category} -> {tier}"),
                                    level: "info".to_string(),
                                }).await;
                            } else {
                                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                                    content: format!("Invalid routing: category={category}, tier={tier}. Use explore/research/code/write + fast/balanced/capable."),
                                    level: "warning".to_string(),
                                }).await;
                            }
                        }
                        Some(Action::ClearChat { .. }) => {
                            {
                                let mut guard = self.session.lock().await;
                                *guard = Session::default();
                            }
                            let _ = self.ui_tx.send(UiEvent::SystemMessage {
                                content: "Chat cleared.".to_string(),
                                level: "info".to_string(),
                            }).await;
                        }
                    }
                }
                spawn_req = async {
                    if let Some(rx) = &mut self.agent_spawn_rx {
                        rx.recv().await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    if let Some(req) = spawn_req {
                        self.handle_spawn_request(req).await;
                    }
                }
            }
        }
    }

    /// Submit a prompt to the primary agent with smart model routing.
    async fn submit_to_primary(
        &self,
        prompt: String,
        effort_budget: Option<u32>,
        model_override: Option<String>,
    ) {
        let agent_id = {
            let mut registry = self.registry.lock().await;
            if let Some(id) = registry.primary_agent_id() {
                id.clone()
            } else {
                registry.create_agent(AgentType::Primary, "primary".to_string(), None)
            }
        };

        let ui_tx = self.ui_tx.clone();
        let registry = self.registry.clone();

        if ui_tx
            .send(UiEvent::AgentStatusChanged {
                agent_id: agent_id.clone(),
                status: AgentStatus::Running,
            })
            .await
            .is_err()
        {
            eprintln!("[orchestrator] TUI channel closed — event dropped");
        }

        // Smart model selection:
        // 1. User override takes priority
        // 2. Otherwise, router picks based on task complexity
        let mut effective_config = self.config.clone();
        let effective_effort;
        if let Some(model) = model_override {
            effective_config.model = model;
            effective_effort = effort_budget;
        } else {
            // Smart routing: classify prompt → pick model + effort
            let route = self.router.route_prompt(&prompt);
            effective_config.model = route.model;
            effective_effort = effort_budget.or(Some(route.effort_budget));
        }

        let agent_id_clone = agent_id.clone();
        let registry_for_handle = self.registry.clone();
        let registry_for_worker = self.registry.clone();
        let session = self.session.clone();

        let handle = tokio::spawn(async move {
            let result = worker::run_agent_turn(
                agent_id_clone.clone(),
                prompt,
                effective_config,
                ui_tx.clone(),
                effective_effort,
                registry_for_worker,
                session,
            )
            .await;

            match result {
                Ok(()) => {
                    if ui_tx
                        .send(UiEvent::StreamEnd {
                            agent_id: agent_id_clone.clone(),
                        })
                        .await
                        .is_err()
                    {
                        eprintln!("[orchestrator] TUI channel closed — event dropped");
                    }
                    let mut reg = registry.lock().await;
                    reg.set_status(&agent_id_clone, AgentStatus::Completed);
                }
                Err(err) => {
                    if ui_tx
                        .send(UiEvent::AgentFailed {
                            agent_id: agent_id_clone.clone(),
                            error: err,
                        })
                        .await
                        .is_err()
                    {
                        eprintln!("[orchestrator] TUI channel closed — event dropped");
                    }
                    let mut reg = registry.lock().await;
                    reg.set_status(&agent_id_clone, AgentStatus::Failed);
                }
            }
        });

        let mut reg = registry_for_handle.lock().await;
        reg.set_handle(&agent_id, handle);
    }

    /// Handle a spawn request from the Agent tool — routes model by agent type.
    async fn handle_spawn_request(&self, req: AgentSpawnRequest) {
        let agent_id = {
            let mut registry = self.registry.lock().await;
            registry.create_agent(req.agent_type.clone(), req.task.clone(), Some(req.parent_id.clone()))
        };

        if self
            .ui_tx
            .send(UiEvent::AgentSpawned {
                agent_id: agent_id.clone(),
                parent_id: Some(req.parent_id),
                agent_type: req.agent_type.clone(),
                task: req.task.clone(),
            })
            .await
            .is_err()
        {
            eprintln!("[orchestrator] TUI channel closed — event dropped");
        }

        let ui_tx = self.ui_tx.clone();
        let registry = self.registry.clone();
        let task = req.task;

        // Smart model routing for sub-agents:
        // 1. Explicit model in spawn request takes priority
        // 2. Otherwise, router picks by agent type
        let mut config = self.config.clone();
        let effort_budget;
        if let Some(model) = req.model {
            config.model = model;
            effort_budget = None;
        } else {
            let route = self.router.route_agent_task(&req.agent_type, &task);
            config.model = route.model;
            effort_budget = Some(route.effort_budget);
        }

        let agent_id_for_handle = agent_id.clone();
        let registry_for_worker = self.registry.clone();
        let handle = tokio::spawn(async move {
            if ui_tx
                .send(UiEvent::AgentStatusChanged {
                    agent_id: agent_id.clone(),
                    status: AgentStatus::Running,
                })
                .await
                .is_err()
            {
                eprintln!("[orchestrator] TUI channel closed — event dropped");
            }

            // Sub-agents get a fresh session (no persistence needed — they're one-shot tasks)
            let sub_session = Arc::new(Mutex::new(Session::default()));
            let result =
                worker::run_agent_turn(agent_id.clone(), task, config, ui_tx.clone(), effort_budget, registry_for_worker, sub_session).await;

            match result {
                Ok(()) => {
                    if ui_tx
                        .send(UiEvent::AgentCompleted {
                            agent_id: agent_id.clone(),
                            result: "completed".to_string(),
                        })
                        .await
                        .is_err()
                    {
                        eprintln!("[orchestrator] TUI channel closed — event dropped");
                    }
                    let mut reg = registry.lock().await;
                    reg.set_status(&agent_id, AgentStatus::Completed);
                }
                Err(err) => {
                    if ui_tx
                        .send(UiEvent::AgentFailed {
                            agent_id: agent_id.clone(),
                            error: err,
                        })
                        .await
                        .is_err()
                    {
                        eprintln!("[orchestrator] TUI channel closed — event dropped");
                    }
                    let mut reg = registry.lock().await;
                    reg.set_status(&agent_id, AgentStatus::Failed);
                }
            }
        });

        // Store handle for cancellation
        let mut reg = self.registry.lock().await;
        reg.set_handle(&agent_id_for_handle, handle);
    }

    /// Resolve a permission prompt by notifying the blocked worker thread.
    async fn resolve_permission(&self, request_id: &str, allow: bool) {
        let mut registry = self.registry.lock().await;
        registry.resolve_permission(request_id, allow);
    }

    async fn resolve_ask_user(&self, request_id: &str, response: String) {
        let mut registry = self.registry.lock().await;
        registry.resolve_ask_user(request_id, response);
    }

    /// Cancel an agent — abort the running task.
    async fn cancel_agent(&self, agent_id: &str) {
        let mut registry = self.registry.lock().await;
        registry.abort_agent(agent_id);
        registry.set_status(agent_id, AgentStatus::Failed);
        drop(registry);
        if self
            .ui_tx
            .send(UiEvent::AgentFailed {
                agent_id: agent_id.to_string(),
                error: "Cancelled by user".to_string(),
            })
            .await
            .is_err()
        {
            eprintln!("[orchestrator] TUI channel closed — event dropped");
        }
    }

    /// Handle MOE dispatch — parse chained commands, build execution plan, spawn agents.
    async fn handle_moe_dispatch(&self, raw_commands: Vec<String>) {
        use moe::{parse_command_chain, build_execution_plan, command_to_prompt, ChainParseResult};

        // Re-join commands for parsing
        let input = raw_commands.join(" ");
        let chain = parse_command_chain(&input);

        let commands = match chain {
            ChainParseResult::Single(text) => {
                // Fell through — just submit as regular prompt
                self.submit_to_primary(text, None, None).await;
                return;
            }
            ChainParseResult::Sequential(cmds) => cmds,
            ChainParseResult::MoeDispatch(cmds) => cmds,
        };

        let plan = build_execution_plan(commands);
        let total = plan.commands.len();

        // Announce MOE dispatch
        if self.ui_tx.send(UiEvent::StreamDelta {
            agent_id: "moe".to_string(),
            text: format!("\n[MOE] Dispatching {total} agents across {} waves\n", plan.waves.len()),
        }).await.is_err() {
            eprintln!("[orchestrator] TUI channel closed — event dropped");
        }

        // Execute waves sequentially (agents within each wave run in parallel)
        for (wave_idx, wave) in plan.waves.iter().enumerate() {
            if self.ui_tx.send(UiEvent::StreamDelta {
                agent_id: "moe".to_string(),
                text: format!("[MOE] Wave {}/{} — {} agent(s)\n", wave_idx + 1, plan.waves.len(), wave.len()),
            }).await.is_err() {
                eprintln!("[orchestrator] TUI channel closed — event dropped");
            }

            let mut handles = Vec::new();

            for &cmd_idx in wave {
                let cmd = &plan.commands[cmd_idx];
                let prompt = command_to_prompt(cmd);

                // Create agent for this command
                let agent_id = {
                    let mut registry = self.registry.lock().await;
                    registry.create_agent(
                        cmd.agent_type.clone(),
                        format!("/{}: {}", cmd.name, cmd.args),
                        None,
                    )
                };

                if self.ui_tx.send(UiEvent::AgentSpawned {
                    agent_id: agent_id.clone(),
                    parent_id: None,
                    agent_type: cmd.agent_type.clone(),
                    task: format!("/{} {}", cmd.name, cmd.args),
                }).await.is_err() {
                    eprintln!("[orchestrator] TUI channel closed — event dropped");
                }

                // Route to optimal model via the routing table
                let route = self.router.route_prompt(&prompt);
                let mut config = self.config.clone();
                config.model = route.model;

                let ui_tx = self.ui_tx.clone();
                let registry = self.registry.clone();
                let effort = Some(route.effort_budget);

                let handle = tokio::spawn(async move {
                    if ui_tx.send(UiEvent::AgentStatusChanged {
                        agent_id: agent_id.clone(),
                        status: AgentStatus::Running,
                    }).await.is_err() {
                        eprintln!("[orchestrator] TUI channel closed — event dropped");
                    }

                    // MOE sub-agents get fresh sessions (one-shot parallel tasks)
                    let moe_session = Arc::new(Mutex::new(Session::default()));
                    let result = worker::run_agent_turn(
                        agent_id.clone(),
                        prompt,
                        config,
                        ui_tx.clone(),
                        effort,
                        registry.clone(),
                        moe_session,
                    ).await;

                    match result {
                        Ok(()) => {
                            if ui_tx.send(UiEvent::AgentCompleted {
                                agent_id: agent_id.clone(),
                                result: "completed".to_string(),
                            }).await.is_err() {
                                eprintln!("[orchestrator] TUI channel closed — event dropped");
                            }
                            let mut reg = registry.lock().await;
                            reg.set_status(&agent_id, AgentStatus::Completed);
                        }
                        Err(err) => {
                            if ui_tx.send(UiEvent::AgentFailed {
                                agent_id: agent_id.clone(),
                                error: err,
                            }).await.is_err() {
                                eprintln!("[orchestrator] TUI channel closed — event dropped");
                            }
                            let mut reg = registry.lock().await;
                            reg.set_status(&agent_id, AgentStatus::Failed);
                        }
                    }
                });

                handles.push(handle);
            }

            // Wait for all agents in this wave to complete
            for handle in handles {
                if let Err(e) = handle.await {
                    eprintln!("[moe] Agent task panicked: {e}");
                }
            }
        }

        // Signal MOE completion
        if self.ui_tx.send(UiEvent::StreamDelta {
            agent_id: "moe".to_string(),
            text: format!("\n[MOE] All {total} agents completed.\n"),
        }).await.is_err() {
            eprintln!("[orchestrator] TUI channel closed — event dropped");
        }
        if self.ui_tx.send(UiEvent::StreamEnd {
            agent_id: "moe".to_string(),
        }).await.is_err() {
            eprintln!("[orchestrator] TUI channel closed — event dropped");
        }
    }

    /// Handle mid-task skill injection — spawn a new agent for a command while others are running.
    async fn handle_skill_injection(&self, command: String) {
        use moe::command_to_prompt;

        // Provide a function that parses a single command (exposed for this purpose)
        let trimmed = command.trim();
        let stripped = trimmed.strip_prefix('/').unwrap_or(trimmed);
        let mut parts = stripped.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("").to_string();
        let args = parts.next().unwrap_or("").trim().to_string();

        let (category, agent_type) = moe::classify_command_pub(&name);

        let cmd = moe::ChainedCommand {
            raw: trimmed.to_string(),
            name: name.clone(),
            args,
            category,
            agent_type: agent_type.clone(),
            depends_on: None,
        };

        let prompt = command_to_prompt(&cmd);

        let agent_id = {
            let mut registry = self.registry.lock().await;
            registry.create_agent(agent_type.clone(), format!("[injected] /{name}"), None)
        };

        if self.ui_tx.send(UiEvent::AgentSpawned {
            agent_id: agent_id.clone(),
            parent_id: None,
            agent_type,
            task: format!("[skill injection] /{name}"),
        }).await.is_err() {
            eprintln!("[orchestrator] TUI channel closed — event dropped");
        }

        let route = self.router.route_prompt(&prompt);
        let mut config = self.config.clone();
        config.model = route.model;

        let ui_tx = self.ui_tx.clone();
        let registry = self.registry.clone();
        let agent_id_for_handle = agent_id.clone();

        let handle = tokio::spawn(async move {
            if ui_tx.send(UiEvent::AgentStatusChanged {
                agent_id: agent_id.clone(),
                status: AgentStatus::Running,
            }).await.is_err() {
                eprintln!("[orchestrator] TUI channel closed — event dropped");
            }

            // Skill injection agents get fresh sessions (one-shot tasks)
            let skill_session = Arc::new(Mutex::new(Session::default()));
            let result = worker::run_agent_turn(
                agent_id.clone(), prompt, config, ui_tx.clone(),
                Some(route.effort_budget), registry.clone(), skill_session,
            ).await;

            match result {
                Ok(()) => {
                    if ui_tx.send(UiEvent::AgentCompleted {
                        agent_id: agent_id.clone(),
                        result: "completed".to_string(),
                    }).await.is_err() {
                        eprintln!("[orchestrator] TUI channel closed — event dropped");
                    }
                    let mut reg = registry.lock().await;
                    reg.set_status(&agent_id, AgentStatus::Completed);
                }
                Err(err) => {
                    if ui_tx.send(UiEvent::AgentFailed {
                        agent_id: agent_id.clone(),
                        error: err,
                    }).await.is_err() {
                        eprintln!("[orchestrator] TUI channel closed — event dropped");
                    }
                    let mut reg = registry.lock().await;
                    reg.set_status(&agent_id, AgentStatus::Failed);
                }
            }
        });

        let mut reg = self.registry.lock().await;
        reg.set_handle(&agent_id_for_handle, handle);
    }

    /// Handle a slash command from the TUI — routes to local handling or the commands crate.
    async fn handle_slash_command(&mut self, command: String) {
        let cmd_str = if command.starts_with('/') {
            command.clone()
        } else {
            format!("/{command}")
        };

        let trimmed = cmd_str.trim_start_matches('/');
        let (cmd_name, cmd_args) = match trimmed.split_once(char::is_whitespace) {
            Some((name, args)) => (name, Some(args.trim())),
            None => (trimmed, None),
        };

        match cmd_name {
            "clear" => {
                {
                    let mut guard = self.session.lock().await;
                    *guard = Session::default();
                }
                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                    content: "Chat cleared.".to_string(),
                    level: "info".to_string(),
                }).await;
            }
            "model" => {
                if let Some(model) = cmd_args {
                    self.config.model = model.to_string();
                    self.router = ModelRouter::from_default_model(model);
                    let _ = self.ui_tx.send(UiEvent::SystemMessage {
                        content: format!("Model changed to: {model}"),
                        level: "info".to_string(),
                    }).await;
                    let _ = self.ui_tx.send(UiEvent::ModelInfo {
                        name: model.to_string(),
                        provider: String::new(),
                    }).await;
                } else {
                    let _ = self.ui_tx.send(UiEvent::SystemMessage {
                        content: format!("Current model: {}", self.config.model),
                        level: "info".to_string(),
                    }).await;
                }
            }
            "help" => {
                let help_text = commands::render_slash_command_help();
                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                    content: help_text,
                    level: "info".to_string(),
                }).await;
            }
            "version" => {
                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                    content: format!("OpenAnalyst CLI v{}", env!("CARGO_PKG_VERSION")),
                    level: "info".to_string(),
                }).await;
            }
            "compact" => {
                let msg = {
                    let guard = self.session.lock().await;
                    let before = guard.messages.len();
                    let compact_config = CompactionConfig::default();
                    let result = runtime::compact_session(&guard, compact_config);
                    if result.removed_message_count == 0 {
                        "Compaction skipped: session is below the compaction threshold.".to_string()
                    } else {
                        // Apply the compacted session
                        drop(guard);
                        let mut guard = self.session.lock().await;
                        let after_count = result.compacted_session.messages.len();
                        *guard = result.compacted_session;
                        format!("Session compacted: {before} -> {after_count} messages")
                    }
                };
                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                    content: msg,
                    level: "info".to_string(),
                }).await;
            }
            "status" => {
                let guard = self.session.lock().await;
                let msg_count = guard.messages.len();
                let estimated_tokens = runtime::estimate_session_tokens(&guard);
                drop(guard);
                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                    content: format!(
                        "Session: {} messages, ~{} tokens\nModel: {}\nPermission mode: {:?}",
                        msg_count, estimated_tokens, self.config.model, self.config.permission_mode
                    ),
                    level: "info".to_string(),
                }).await;
            }
            "cost" => {
                let guard = self.session.lock().await;
                let estimated_tokens = runtime::estimate_session_tokens(&guard);
                drop(guard);
                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                    content: format!("Session tokens (estimated): {estimated_tokens}"),
                    level: "info".to_string(),
                }).await;
            }
            "permissions" => {
                if let Some(mode_str) = cmd_args {
                    if let Some(pm) = parse_permission_mode(mode_str) {
                        self.config.permission_mode = pm;
                        let _ = self.ui_tx.send(UiEvent::SystemMessage {
                            content: format!("Permission mode changed to: {mode_str}"),
                            level: "info".to_string(),
                        }).await;
                    } else {
                        let _ = self.ui_tx.send(UiEvent::SystemMessage {
                            content: format!("Unknown permission mode: {mode_str}. Use: read-only, workspace-write, danger-full-access, prompt, allow."),
                            level: "warning".to_string(),
                        }).await;
                    }
                } else {
                    let _ = self.ui_tx.send(UiEvent::SystemMessage {
                        content: format!("Current permission mode: {:?}", self.config.permission_mode),
                        level: "info".to_string(),
                    }).await;
                }
            }
            "route" => {
                let table = self.router.render_table();
                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                    content: table,
                    level: "info".to_string(),
                }).await;
            }
            "effort" => {
                if let Some(args) = cmd_args {
                    // Parse: "/effort <level>" (global) or "/effort <category> <level>"
                    let parts: Vec<&str> = args.split_whitespace().collect();
                    match parts.len() {
                        1 => {
                            if let Some(effort) = crate::router::EffortLevel::from_str_opt(parts[0]) {
                                self.router.table.set_effort_all(effort);
                                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                                    content: format!("Effort set to {} for all categories.", effort),
                                    level: "info".to_string(),
                                }).await;
                            } else {
                                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                                    content: format!("Unknown effort level: {}. Use: low, medium, high, max.", parts[0]),
                                    level: "warning".to_string(),
                                }).await;
                            }
                        }
                        2 => {
                            if let (Some(cat), Some(effort)) = (
                                ActionCategory::from_str_opt(parts[0]),
                                crate::router::EffortLevel::from_str_opt(parts[1]),
                            ) {
                                self.router.table.set_effort(cat, effort);
                                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                                    content: format!("Effort for {} set to {}.", cat, effort),
                                    level: "info".to_string(),
                                }).await;
                            } else {
                                let _ = self.ui_tx.send(UiEvent::SystemMessage {
                                    content: "Usage: /effort <level> or /effort <category> <level>".to_string(),
                                    level: "warning".to_string(),
                                }).await;
                            }
                        }
                        _ => {
                            let _ = self.ui_tx.send(UiEvent::SystemMessage {
                                content: "Usage: /effort <level> or /effort <category> <level>".to_string(),
                                level: "warning".to_string(),
                            }).await;
                        }
                    }
                } else {
                    let table = self.router.render_table();
                    let _ = self.ui_tx.send(UiEvent::SystemMessage {
                        content: table,
                        level: "info".to_string(),
                    }).await;
                }
            }
            _ => {
                // For all other commands, try the commands crate
                let session_guard = self.session.lock().await;
                let compact_config = CompactionConfig::default();
                if let Some(result) = commands::handle_slash_command(&cmd_str, &session_guard, compact_config) {
                    // If the command returned a modified session, apply it
                    let new_session = result.session;
                    drop(session_guard);
                    {
                        let mut guard = self.session.lock().await;
                        *guard = new_session;
                    }
                    let _ = self.ui_tx.send(UiEvent::SystemMessage {
                        content: result.message,
                        level: "info".to_string(),
                    }).await;
                } else {
                    drop(session_guard);
                    let _ = self.ui_tx.send(UiEvent::SystemMessage {
                        content: format!("Unknown command: /{cmd_name}. Type /help for available commands."),
                        level: "warning".to_string(),
                    }).await;
                }
            }
        }
    }
}

/// Parse a permission mode string into `PermissionMode`.
fn parse_permission_mode(mode: &str) -> Option<PermissionMode> {
    match mode.to_ascii_lowercase().as_str() {
        "read-only" | "readonly" | "ro" => Some(PermissionMode::ReadOnly),
        "workspace" | "workspace-write" | "ws" => Some(PermissionMode::WorkspaceWrite),
        "full" | "danger-full-access" | "yolo" => Some(PermissionMode::DangerFullAccess),
        "prompt" | "ask" | "default" => Some(PermissionMode::Prompt),
        "allow" | "allow-all" => Some(PermissionMode::Allow),
        _ => None,
    }
}
