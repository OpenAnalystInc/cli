//! Custom agent definitions loader — .openanalyst/agents/*.md
//!
//! Defines custom subagents with specific tools, models, and system prompts.
//!
//! ## Format
//!
//! ```yaml
//! ---
//! name: code-reviewer
//! description: Reviews code for quality, security, and best practices
//! model: "openanalyst-beta"
//! tools:
//!   - Read
//!   - Glob
//!   - Grep
//! allowed-tools: "Read Glob Grep"
//! disabled: false
//! mcp-servers:
//!   - "github"
//! permissions:
//!   deny:
//!     - "Bash"
//!     - "Edit"
//! ---
//!
//! You are a code reviewer. Analyze code for...
//! ```

use std::path::{Path, PathBuf};

use crate::config_paths::{managed_config_home, user_config_home};

/// A custom agent definition loaded from .openanalyst/agents/.
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    /// Agent name (used to reference the agent).
    pub name: String,
    /// Description of when this agent should be used.
    pub description: String,
    /// Model override for this agent.
    pub model: Option<String>,
    /// List of tools this agent can use.
    pub tools: Vec<String>,
    /// Permission rule format string (e.g., "Read Glob Grep").
    pub allowed_tools: Option<String>,
    /// Whether this agent is disabled.
    pub disabled: bool,
    /// MCP servers available to this agent.
    pub mcp_servers: Vec<String>,
    /// Permission deny rules.
    pub deny_rules: Vec<String>,
    /// Permission ask rules.
    pub ask_rules: Vec<String>,
    /// Permission allow rules.
    pub allow_rules: Vec<String>,
    /// Enable persistent auto memory for this agent.
    pub memory: bool,
    /// The system prompt / instructions for this agent.
    pub system_prompt: String,
    /// Source file path.
    pub source: PathBuf,
    /// Scope (project or user level).
    pub scope: AgentScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentScope {
    Managed,
    Project,
    User,
}

/// Load all custom agent definitions from managed, user, and project directories.
pub fn load_agent_definitions(project_dir: &Path) -> Vec<AgentDefinition> {
    let mut agents = Vec::new();

    // Managed/global level
    if let Some(managed) = managed_config_home() {
        let managed_agents = managed.join("agents");
        if managed_agents.is_dir() {
            load_agents_from_dir(&managed_agents, AgentScope::Managed, &mut agents);
        }
    }

    // User-level: ~/.openanalyst/agents/*.md
    if let Some(home) = user_config_home() {
        let user_agents = home.join("agents");
        if user_agents.is_dir() {
            load_agents_from_dir(&user_agents, AgentScope::User, &mut agents);
        }
    }

    // Project-level: .openanalyst/agents/*.md
    let project_agents = project_dir.join(".openanalyst").join("agents");
    if project_agents.is_dir() {
        load_agents_from_dir(&project_agents, AgentScope::Project, &mut agents);
    }

    // Deduplicate: managed always kept, project overrides user
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for agent in agents.iter().filter(|a| a.scope == AgentScope::Managed) {
        seen.insert(agent.name.clone());
        deduped.push(agent.clone());
    }
    for agent in agents.iter().filter(|a| a.scope == AgentScope::Project) {
        if seen.insert(agent.name.clone()) {
            deduped.push(agent.clone());
        }
    }
    for agent in agents.iter().filter(|a| a.scope == AgentScope::User) {
        if seen.insert(agent.name.clone()) {
            deduped.push(agent.clone());
        }
    }

    deduped
}

/// Get only enabled (non-disabled) agent definitions.
pub fn active_agent_definitions(agents: &[AgentDefinition]) -> Vec<&AgentDefinition> {
    agents.iter().filter(|a| !a.disabled).collect()
}

/// Format agent definitions as a help listing.
pub fn format_agents_list(agents: &[AgentDefinition]) -> String {
    if agents.is_empty() {
        return "No custom agents defined.\n\n\
                Create agents by adding .md files to:\n\
                  .openanalyst/agents/  (project-level)\n\
                  ~/.openanalyst/agents/ (user-level)\n\n\
                Format:\n\
                ---\n\
                name: my-agent\n\
                description: What this agent does\n\
                model: openanalyst-beta\n\
                tools:\n\
                  - Read\n\
                  - Grep\n\
                ---\n\
                System prompt for the agent here.".to_string();
    }

    let active = agents.iter().filter(|a| !a.disabled).count();
    let disabled = agents.len() - active;
    let mut out = format!("Custom Agents ({} active", active);
    if disabled > 0 {
        out.push_str(&format!(", {} disabled", disabled));
    }
    out.push_str("):\n\n");

    for agent in agents {
        let scope = match agent.scope {
            AgentScope::Managed => "managed",
            AgentScope::Project => "project",
            AgentScope::User => "user",
        };
        let status = if agent.disabled { " (disabled)" } else { "" };
        let model = agent.model.as_deref().unwrap_or("default");
        out.push_str(&format!(
            "  {} — {}{} [{}] model={}\n",
            agent.name, agent.description, status, scope, model
        ));
        if !agent.tools.is_empty() {
            out.push_str(&format!("    tools: {}\n", agent.tools.join(", ")));
        }
    }
    out
}

// ── Internal ────────────────────────────────────────────────────────────────

fn load_agents_from_dir(dir: &Path, scope: AgentScope, agents: &mut Vec<AgentDefinition>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md") {
            if let Some(agent) = parse_agent_file(&path, scope) {
                agents.push(agent);
            }
        }
    }
}

fn parse_agent_file(path: &Path, scope: AgentScope) -> Option<AgentDefinition> {
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    let file_name = path.file_stem()?.to_string_lossy().to_string();

    if !trimmed.starts_with("---") {
        return Some(AgentDefinition {
            name: file_name,
            description: String::new(),
            model: None,
            tools: Vec::new(),
            allowed_tools: None,
            disabled: false,
            mcp_servers: Vec::new(),
            deny_rules: Vec::new(),
            ask_rules: Vec::new(),
            allow_rules: Vec::new(),
            memory: false,
            system_prompt: content,
            source: path.to_path_buf(),
            scope,
        });
    }

    let after_first = &trimmed[3..];
    let end_marker = after_first.find("---")?;
    let frontmatter = after_first[..end_marker].trim();
    let body = after_first[end_marker + 3..].trim().to_string();

    let mut def = AgentDefinition {
        name: file_name,
        description: String::new(),
        model: None,
        tools: Vec::new(),
        allowed_tools: None,
        disabled: false,
        mcp_servers: Vec::new(),
        deny_rules: Vec::new(),
        ask_rules: Vec::new(),
        allow_rules: Vec::new(),
        memory: false,
        system_prompt: body,
        source: path.to_path_buf(),
        scope,
    };

    #[derive(PartialEq)]
    enum ListContext {
        None,
        Tools,
        McpServers,
        Deny,
        Ask,
        Allow,
    }
    let mut list_ctx = ListContext::None;

    for line in frontmatter.lines() {
        let line = line.trim();

        // Handle YAML list items
        if list_ctx != ListContext::None && line.starts_with("- ") {
            let val = line.strip_prefix("- ").unwrap().trim()
                .trim_matches('"').trim_matches('\'').to_string();
            if !val.is_empty() {
                match list_ctx {
                    ListContext::Tools => def.tools.push(val),
                    ListContext::McpServers => def.mcp_servers.push(val),
                    ListContext::Deny => def.deny_rules.push(val),
                    ListContext::Ask => def.ask_rules.push(val),
                    ListContext::Allow => def.allow_rules.push(val),
                    ListContext::None => {}
                }
            }
            continue;
        }

        if !line.starts_with('-') && !line.is_empty() && list_ctx != ListContext::None {
            list_ctx = ListContext::None;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "name" => def.name = value.to_string(),
                "description" | "desc" => def.description = value.to_string(),
                "model" => def.model = some_nonempty(value),
                "allowed-tools" | "allowedTools" => def.allowed_tools = some_nonempty(value),
                "disabled" => def.disabled = value == "true",
                "memory" => def.memory = value == "true",
                "tools" => {
                    if value.is_empty() {
                        list_ctx = ListContext::Tools;
                    } else {
                        // Inline space-separated
                        def.tools.extend(value.split_whitespace().map(String::from));
                    }
                }
                "mcp-servers" | "mcpServers" => {
                    if value.is_empty() {
                        list_ctx = ListContext::McpServers;
                    }
                }
                "deny" => {
                    if value.is_empty() {
                        list_ctx = ListContext::Deny;
                    }
                }
                "ask" => {
                    if value.is_empty() {
                        list_ctx = ListContext::Ask;
                    }
                }
                "allow" => {
                    if value.is_empty() {
                        list_ctx = ListContext::Allow;
                    }
                }
                // Nested under permissions:
                _ if key == "permissions" => {
                    // permissions: is a parent key, items follow
                }
                _ => {}
            }
        }
    }

    Some(def)
}

fn some_nonempty(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_agent_with_full_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_agent_reviewer.md");
        std::fs::write(&tmp, "\
---
name: code-reviewer
description: Reviews code for quality and security
model: openanalyst-beta
disabled: false
memory: true
tools:
  - Read
  - Glob
  - Grep
mcp-servers:
  - github
deny:
  - Bash
  - Edit
---

You are a code reviewer. Analyze the code for:
1. Security vulnerabilities
2. Performance issues
3. Code style
").unwrap();

        let agent = parse_agent_file(&tmp, AgentScope::Project).unwrap();
        assert_eq!(agent.name, "code-reviewer");
        assert_eq!(agent.description, "Reviews code for quality and security");
        assert_eq!(agent.model.as_deref(), Some("openanalyst-beta"));
        assert!(!agent.disabled);
        assert!(agent.memory);
        assert_eq!(agent.tools, vec!["Read", "Glob", "Grep"]);
        assert_eq!(agent.mcp_servers, vec!["github"]);
        assert_eq!(agent.deny_rules, vec!["Bash", "Edit"]);
        assert!(agent.system_prompt.contains("code reviewer"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parse_agent_without_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_agent_simple.md");
        std::fs::write(&tmp, "You are a helpful assistant.").unwrap();

        let agent = parse_agent_file(&tmp, AgentScope::User).unwrap();
        assert_eq!(agent.name, "oa_agent_simple");
        assert!(agent.system_prompt.contains("helpful assistant"));
        assert!(!agent.disabled);
        assert!(agent.tools.is_empty());

        let _ = std::fs::remove_file(&tmp);
    }
}
