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
use runtime::PermissionMode;
use tokio::sync::Mutex;

use crate::registry::AgentRegistry;
use crate::router::ModelRouter;

/// Configuration for the orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub model: String,
    pub permission_mode: PermissionMode,
    pub allowed_tools: Option<BTreeSet<String>>,
    pub cwd: PathBuf,
    pub system_prompt: Vec<String>,
}

/// The main orchestrator that manages all agents and routes events.
pub struct AgentOrchestrator {
    config: OrchestratorConfig,
    registry: Arc<Mutex<AgentRegistry>>,
    router: ModelRouter,
    ui_tx: UiEventTx,
    action_rx: ActionRx,
    agent_spawn_rx: Option<AgentSpawnRx>,
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
                        Some(Action::CancelAgent(id)) => {
                            self.cancel_agent(&id).await;
                        }
                        Some(Action::UpdateModel(model)) => {
                            self.config.model = model.clone();
                            self.router = ModelRouter::from_default_model(&model);
                        }
                        Some(Action::UpdatePermissions(mode)) => {
                            if let Some(pm) = parse_permission_mode(&mode) {
                                self.config.permission_mode = pm;
                            }
                        }
                        Some(Action::Quit) | None => break,
                        Some(Action::SlashCommand(_)) => {}
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

        let _ = ui_tx
            .send(UiEvent::AgentStatusChanged {
                agent_id: agent_id.clone(),
                status: AgentStatus::Running,
            })
            .await;

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

        let handle = tokio::spawn(async move {
            let result = worker::run_agent_turn(
                agent_id_clone.clone(),
                prompt,
                effective_config,
                ui_tx.clone(),
                effective_effort,
                registry_for_worker,
            )
            .await;

            match result {
                Ok(()) => {
                    let _ = ui_tx
                        .send(UiEvent::StreamEnd {
                            agent_id: agent_id_clone.clone(),
                        })
                        .await;
                    let mut reg = registry.lock().await;
                    reg.set_status(&agent_id_clone, AgentStatus::Completed);
                }
                Err(err) => {
                    let _ = ui_tx
                        .send(UiEvent::AgentFailed {
                            agent_id: agent_id_clone.clone(),
                            error: err,
                        })
                        .await;
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

        let _ = self
            .ui_tx
            .send(UiEvent::AgentSpawned {
                agent_id: agent_id.clone(),
                parent_id: Some(req.parent_id),
                agent_type: req.agent_type.clone(),
                task: req.task.clone(),
            })
            .await;

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
            let _ = ui_tx
                .send(UiEvent::AgentStatusChanged {
                    agent_id: agent_id.clone(),
                    status: AgentStatus::Running,
                })
                .await;

            let result =
                worker::run_agent_turn(agent_id.clone(), task, config, ui_tx.clone(), effort_budget, registry_for_worker).await;

            match result {
                Ok(()) => {
                    let _ = ui_tx
                        .send(UiEvent::AgentCompleted {
                            agent_id: agent_id.clone(),
                            result: "completed".to_string(),
                        })
                        .await;
                    let mut reg = registry.lock().await;
                    reg.set_status(&agent_id, AgentStatus::Completed);
                }
                Err(err) => {
                    let _ = ui_tx
                        .send(UiEvent::AgentFailed {
                            agent_id: agent_id.clone(),
                            error: err,
                        })
                        .await;
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

    /// Cancel an agent — abort the running task.
    async fn cancel_agent(&self, agent_id: &str) {
        let mut registry = self.registry.lock().await;
        registry.abort_agent(agent_id);
        registry.set_status(agent_id, AgentStatus::Failed);
        drop(registry);
        let _ = self
            .ui_tx
            .send(UiEvent::AgentFailed {
                agent_id: agent_id.to_string(),
                error: "Cancelled by user".to_string(),
            })
            .await;
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
