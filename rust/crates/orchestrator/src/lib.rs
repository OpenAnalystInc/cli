//! Multi-agent orchestrator for the OpenAnalyst TUI.
//!
//! Manages agent lifecycle, bridges the sync `ConversationRuntime` to the async TUI
//! via channel-based `ApiClient`, `ToolExecutor`, and `PermissionPrompter` implementations.

pub mod agent;
pub mod registry;
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
    ui_tx: UiEventTx,
    action_rx: ActionRx,
    agent_spawn_rx: Option<AgentSpawnRx>,
}

impl AgentOrchestrator {
    /// Create a new orchestrator.
    #[must_use]
    pub fn new(
        config: OrchestratorConfig,
        ui_tx: UiEventTx,
        action_rx: ActionRx,
        agent_spawn_rx: Option<AgentSpawnRx>,
    ) -> Self {
        Self {
            config,
            registry: Arc::new(Mutex::new(AgentRegistry::new())),
            ui_tx,
            action_rx,
            agent_spawn_rx,
        }
    }

    /// Run the orchestrator event loop.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                // Handle user actions from the TUI
                action = self.action_rx.recv() => {
                    match action {
                        Some(Action::SubmitPrompt(text)) => {
                            self.submit_to_primary(text).await;
                        }
                        Some(Action::PermissionResponse { request_id, allow }) => {
                            self.resolve_permission(&request_id, allow).await;
                        }
                        Some(Action::CancelAgent(id)) => {
                            self.cancel_agent(&id).await;
                        }
                        Some(Action::Quit) | None => break,
                        Some(Action::SlashCommand(_)) => {
                            // TODO: handle slash commands
                        }
                    }
                }
                // Handle agent spawn requests from the Agent tool
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

    /// Submit a prompt to the primary agent. Spawns one if it doesn't exist.
    async fn submit_to_primary(&self, prompt: String) {
        let agent_id = {
            let mut registry = self.registry.lock().await;
            if let Some(id) = registry.primary_agent_id() {
                id.clone()
            } else {
                let id = registry.create_agent(AgentType::Primary, "primary".to_string(), None);
                id
            }
        };

        let ui_tx = self.ui_tx.clone();
        let config = self.config.clone();
        let registry = self.registry.clone();

        // Notify TUI that agent is running
        let _ = ui_tx
            .send(UiEvent::AgentStatusChanged {
                agent_id: agent_id.clone(),
                status: AgentStatus::Running,
            })
            .await;

        let agent_id_clone = agent_id.clone();
        let registry_for_handle = self.registry.clone();
        let handle = tokio::spawn(async move {
            let result = worker::run_agent_turn(
                agent_id_clone.clone(),
                prompt,
                config,
                ui_tx.clone(),
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

        // Store handle so we can abort on cancel
        let mut reg = registry_for_handle.lock().await;
        reg.set_handle(&agent_id, handle);
    }

    /// Handle a spawn request from the Agent tool.
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
                agent_type: req.agent_type,
                task: req.task.clone(),
            })
            .await;

        let ui_tx = self.ui_tx.clone();
        let config = self.config.clone();
        let registry = self.registry.clone();
        let task = req.task;

        tokio::spawn(async move {
            let _ = ui_tx
                .send(UiEvent::AgentStatusChanged {
                    agent_id: agent_id.clone(),
                    status: AgentStatus::Running,
                })
                .await;

            let result =
                worker::run_agent_turn(agent_id.clone(), task, config, ui_tx.clone()).await;

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
