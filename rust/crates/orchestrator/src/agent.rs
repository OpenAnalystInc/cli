//! Managed agent state tracking.

use events::{AgentId, AgentStatus, AgentType};

/// A managed agent tracked by the orchestrator.
#[derive(Debug, Clone)]
pub struct ManagedAgent {
    pub id: AgentId,
    pub agent_type: AgentType,
    pub status: AgentStatus,
    pub parent_id: Option<AgentId>,
    pub task: String,
}

impl ManagedAgent {
    /// Create a new managed agent.
    #[must_use]
    pub fn new(
        id: AgentId,
        agent_type: AgentType,
        task: String,
        parent_id: Option<AgentId>,
    ) -> Self {
        Self {
            id,
            agent_type,
            status: AgentStatus::Pending,
            parent_id,
            task,
        }
    }
}
