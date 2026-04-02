//! Agent registry — tracks all managed agents and pending permission requests.

use std::collections::HashMap;

use events::{AgentId, AgentStatus, AgentType, PermissionResponseTx};
use tokio::task::JoinHandle;

use crate::agent::ManagedAgent;

/// Registry tracking all active agents and pending permission prompts.
pub struct AgentRegistry {
    agents: HashMap<AgentId, ManagedAgent>,
    /// Pending permission responses: request_id → oneshot sender.
    permission_pending: HashMap<String, PermissionResponseTx>,
    /// Task handles for spawned agents — used for cancellation.
    task_handles: HashMap<AgentId, JoinHandle<()>>,
    next_id: u64,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self {
            agents: HashMap::new(),
            permission_pending: HashMap::new(),
            task_handles: HashMap::new(),
            next_id: 0,
        }
    }
}

impl AgentRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new agent and return its ID.
    pub fn create_agent(
        &mut self,
        agent_type: AgentType,
        task: String,
        parent_id: Option<AgentId>,
    ) -> AgentId {
        self.next_id += 1;
        let id = format!("agent-{}", self.next_id);
        let agent = ManagedAgent::new(id.clone(), agent_type, task, parent_id);
        self.agents.insert(id.clone(), agent);
        id
    }

    /// Get the primary agent ID (if one exists).
    #[must_use]
    pub fn primary_agent_id(&self) -> Option<&AgentId> {
        self.agents
            .values()
            .find(|a| a.agent_type == AgentType::Primary)
            .map(|a| &a.id)
    }

    /// Update an agent's status.
    pub fn set_status(&mut self, agent_id: &str, status: AgentStatus) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.status = status;
        }
    }

    /// Register a pending permission request.
    pub fn register_permission(&mut self, request_id: String, tx: PermissionResponseTx) {
        self.permission_pending.insert(request_id, tx);
    }

    /// Resolve a pending permission request by sending the decision to the blocked worker.
    pub fn resolve_permission(&mut self, request_id: &str, allow: bool) {
        if let Some(tx) = self.permission_pending.remove(request_id) {
            // Send the decision — if the receiver was dropped, the error is harmless.
            if tx.send(allow).is_err() {
                eprintln!("[registry] Permission response dropped — worker thread may have been cancelled");
            }
        }
    }

    /// Store a task handle for an agent.
    pub fn set_handle(&mut self, agent_id: &str, handle: JoinHandle<()>) {
        self.task_handles.insert(agent_id.to_string(), handle);
    }

    /// Abort a running agent's task.
    pub fn abort_agent(&mut self, agent_id: &str) {
        if let Some(handle) = self.task_handles.remove(agent_id) {
            handle.abort();
        }
    }

    /// Get all agents.
    #[must_use]
    pub fn agents(&self) -> Vec<&ManagedAgent> {
        self.agents.values().collect()
    }

    /// Get a specific agent.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ManagedAgent> {
        self.agents.get(id)
    }
}
