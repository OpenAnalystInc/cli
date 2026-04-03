//! Policy engine — declarative TOML-based access control for tools.
//!
//! Declarative TOML-based access control for tools with:
//! - Priority-based rule matching (higher priority wins)
//! - Wildcard tool name matching (`*`, `mcp_*`, `mcp_server_*`)
//! - Approval modes (Default, AutoEdit, Yolo, Plan)
//! - Shell command argument pattern matching
//! - Per-rule deny messages
//! - Tiered priority bands (default, extension, workspace, user, admin)

use regex::Regex;

// ── Decision & Approval Mode ─────────────────────────────────────────────────

/// Policy decision for a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Tool execution is permitted.
    Allow,
    /// Tool execution is blocked.
    Deny,
    /// Ask the user for approval (becomes Deny in non-interactive mode).
    AskUser,
}

/// Approval mode that filters which rules apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApprovalMode {
    /// Normal operation — asks user for unknown tools.
    Default,
    /// Permissive — auto-allows read-only + safe write operations.
    AutoEdit,
    /// Fully autonomous — allows everything except certain meta-operations.
    Yolo,
    /// Planning only — only read-only tools allowed.
    Plan,
}

impl ApprovalMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "default" | "prompt" | "ask" => Some(Self::Default),
            "autoedit" | "auto_edit" | "auto-edit" | "workspace" | "workspace-write" => Some(Self::AutoEdit),
            "yolo" | "full" | "danger-full-access" | "allow" | "allow-all" => Some(Self::Yolo),
            "plan" | "read-only" | "readonly" => Some(Self::Plan),
            _ => None,
        }
    }
}

// ── Priority Tiers ───────────────────────────────────────────────────────────

/// Priority tier bands for layered policy resolution.
/// Rules within each tier use `tier_base + (rule_priority / 1000)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyTier {
    /// Built-in defaults (1.000–1.999).
    Default = 1,
    /// Extension-contributed (2.000–2.999).
    Extension = 2,
    /// Workspace-local .openanalyst/policy.toml (3.000–3.999).
    Workspace = 3,
    /// User home ~/.openanalyst/policy.toml (4.000–4.999).
    User = 4,
    /// System admin (5.000–5.999).
    Admin = 5,
}

impl PolicyTier {
    fn base_priority(self) -> f64 {
        self as u8 as f64
    }
}

// ── Policy Rule ──────────────────────────────────────────────────────────────

/// A single policy rule.
#[derive(Debug, Clone)]
pub struct PolicyRule {
    /// Tool name to match. `*` for all, `mcp_*` for all MCP, `mcp_server_*` for specific server.
    pub tool_name: String,
    /// Decision when this rule matches.
    pub decision: PolicyDecision,
    /// Priority (higher wins). Computed from tier + rule priority.
    pub priority: f64,
    /// Only apply in these approval modes (None = all modes).
    pub modes: Option<Vec<ApprovalMode>>,
    /// Only for interactive (Some(true)) or non-interactive (Some(false)) contexts.
    pub interactive: Option<bool>,
    /// Optional regex pattern for tool arguments (JSON-serialized).
    pub args_pattern: Option<Regex>,
    /// MCP server name context (for FQN tools like `mcp_server_tool`).
    pub mcp_name: Option<String>,
    /// Allow shell redirections when decision is Allow.
    pub allow_redirection: bool,
    /// Custom denial message.
    pub deny_message: Option<String>,
    /// Source identifier (for debugging).
    pub source: String,
}

// ── Policy Engine ────────────────────────────────────────────────────────────

/// The policy engine configuration.
pub struct PolicyEngine {
    /// Rules sorted by priority (descending — highest first).
    rules: Vec<PolicyRule>,
    /// Default decision when no rule matches.
    pub default_decision: PolicyDecision,
    /// Whether running in non-interactive mode.
    pub non_interactive: bool,
    /// Current approval mode.
    pub approval_mode: ApprovalMode,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            default_decision: PolicyDecision::AskUser,
            non_interactive: false,
            approval_mode: ApprovalMode::Default,
        }
    }
}

impl PolicyEngine {
    /// Create with built-in default rules.
    pub fn with_defaults() -> Self {
        let mut engine = Self::default();
        engine.add_default_rules();
        engine
    }

    /// Check if a tool call is allowed.
    ///
    /// `tool_name` is the tool name (e.g., "read_file", "mcp_github_list_repos").
    /// `args_json` is the JSON-serialized arguments (for pattern matching).
    /// `is_interactive` indicates whether user can be prompted.
    pub fn check(
        &self,
        tool_name: &str,
        args_json: Option<&str>,
        is_interactive: bool,
    ) -> PolicyCheckResult {
        // Find first matching rule
        for rule in &self.rules {
            if !self.rule_matches(rule, tool_name, args_json, is_interactive) {
                continue;
            }

            let decision = self.finalize_decision(rule.decision);
            return PolicyCheckResult {
                decision,
                matched_rule: Some(rule.source.clone()),
                deny_message: rule.deny_message.clone(),
                allow_redirection: rule.allow_redirection,
            };
        }

        // No rule matched — use default
        let decision = self.finalize_decision(self.default_decision);
        PolicyCheckResult {
            decision,
            matched_rule: None,
            deny_message: None,
            allow_redirection: false,
        }
    }

    /// Add a rule and re-sort by priority.
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
        self.rules.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// Remove all rules from a specific source.
    pub fn remove_rules_by_source(&mut self, source: &str) {
        self.rules.retain(|r| r.source != source);
    }

    /// Remove all rules for a specific tool.
    pub fn remove_rules_for_tool(&mut self, tool_name: &str) {
        self.rules.retain(|r| r.tool_name != tool_name);
    }

    /// Get all rules (for debugging/display).
    pub fn rules(&self) -> &[PolicyRule] {
        &self.rules
    }

    /// Set the approval mode.
    pub fn set_approval_mode(&mut self, mode: ApprovalMode) {
        self.approval_mode = mode;
    }

    // ── Internal ─────────────────────────────────────────────────────────

    fn rule_matches(
        &self,
        rule: &PolicyRule,
        tool_name: &str,
        args_json: Option<&str>,
        is_interactive: bool,
    ) -> bool {
        // Check approval mode filter
        if let Some(ref modes) = rule.modes {
            if !modes.contains(&self.approval_mode) {
                return false;
            }
        }

        // Check interactive filter
        if let Some(rule_interactive) = rule.interactive {
            if rule_interactive != is_interactive {
                return false;
            }
        }

        // Check MCP server context
        if let Some(ref mcp_name) = rule.mcp_name {
            let server = extract_mcp_server(tool_name);
            if mcp_name != "*" && server.as_deref() != Some(mcp_name.as_str()) {
                return false;
            }
        }

        // Check tool name match
        if !tool_name_matches(&rule.tool_name, tool_name) {
            return false;
        }

        // Check args pattern
        if let Some(ref pattern) = rule.args_pattern {
            if let Some(args) = args_json {
                if !pattern.is_match(args) {
                    return false;
                }
            } else {
                return false; // Pattern requires args but none provided
            }
        }

        true
    }

    fn finalize_decision(&self, decision: PolicyDecision) -> PolicyDecision {
        // In Yolo mode, override to Allow (except for meta-operations)
        if self.approval_mode == ApprovalMode::Yolo && decision == PolicyDecision::AskUser {
            return PolicyDecision::Allow;
        }

        // In non-interactive mode, AskUser becomes Deny
        if self.non_interactive && decision == PolicyDecision::AskUser {
            return PolicyDecision::Deny;
        }

        decision
    }

    fn add_default_rules(&mut self) {
        let tier = PolicyTier::Default;

        // Read-only tools: always allow
        for tool in &["read_file", "glob_search", "grep_search", "Read", "Glob", "Grep", "ToolSearch"] {
            self.rules.push(PolicyRule {
                tool_name: tool.to_string(),
                decision: PolicyDecision::Allow,
                priority: tier.base_priority() + 0.050,
                modes: None,
                interactive: None,
                args_pattern: None,
                mcp_name: None,
                allow_redirection: false,
                deny_message: None,
                source: "builtin:read-only".to_string(),
            });
        }

        // Write tools: ask user in default mode, allow in auto-edit/yolo
        for tool in &["write_file", "edit_file", "Write", "Edit"] {
            self.rules.push(PolicyRule {
                tool_name: tool.to_string(),
                decision: PolicyDecision::AskUser,
                priority: tier.base_priority() + 0.010,
                modes: Some(vec![ApprovalMode::Default]),
                interactive: Some(true),
                args_pattern: None,
                mcp_name: None,
                allow_redirection: false,
                deny_message: None,
                source: "builtin:write".to_string(),
            });
            self.rules.push(PolicyRule {
                tool_name: tool.to_string(),
                decision: PolicyDecision::Allow,
                priority: tier.base_priority() + 0.015,
                modes: Some(vec![ApprovalMode::AutoEdit, ApprovalMode::Yolo]),
                interactive: None,
                args_pattern: None,
                mcp_name: None,
                allow_redirection: false,
                deny_message: None,
                source: "builtin:write-auto".to_string(),
            });
        }

        // Bash/shell: ask in default, allow in yolo
        for tool in &["bash", "Bash", "PowerShell"] {
            self.rules.push(PolicyRule {
                tool_name: tool.to_string(),
                decision: PolicyDecision::AskUser,
                priority: tier.base_priority() + 0.010,
                modes: Some(vec![ApprovalMode::Default, ApprovalMode::AutoEdit]),
                interactive: Some(true),
                args_pattern: None,
                mcp_name: None,
                allow_redirection: false,
                deny_message: None,
                source: "builtin:bash".to_string(),
            });
            self.rules.push(PolicyRule {
                tool_name: tool.to_string(),
                decision: PolicyDecision::Allow,
                priority: tier.base_priority() + 0.015,
                modes: Some(vec![ApprovalMode::Yolo]),
                interactive: None,
                args_pattern: None,
                mcp_name: None,
                allow_redirection: true,
                deny_message: None,
                source: "builtin:bash-yolo".to_string(),
            });
        }

        // Plan mode: deny everything except read-only
        self.rules.push(PolicyRule {
            tool_name: "*".to_string(),
            decision: PolicyDecision::Deny,
            priority: tier.base_priority() + 0.060,
            modes: Some(vec![ApprovalMode::Plan]),
            interactive: None,
            args_pattern: None,
            mcp_name: None,
            allow_redirection: false,
            deny_message: Some("Plan mode: only read-only tools allowed".to_string()),
            source: "builtin:plan-deny".to_string(),
        });

        // Plan mode: selectively allow read-only
        for tool in &["read_file", "glob_search", "grep_search", "Read", "Glob", "Grep", "ToolSearch", "WebFetch", "WebSearch"] {
            self.rules.push(PolicyRule {
                tool_name: tool.to_string(),
                decision: PolicyDecision::Allow,
                priority: tier.base_priority() + 0.070,
                modes: Some(vec![ApprovalMode::Plan]),
                interactive: None,
                args_pattern: None,
                mcp_name: None,
                allow_redirection: false,
                deny_message: None,
                source: "builtin:plan-allow-read".to_string(),
            });
        }

        // Sort all rules
        self.rules.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

/// Result of a policy check.
#[derive(Debug, Clone)]
pub struct PolicyCheckResult {
    pub decision: PolicyDecision,
    pub matched_rule: Option<String>,
    pub deny_message: Option<String>,
    pub allow_redirection: bool,
}

// ── TOML Loading ─────────────────────────────────────────────────────────────

/// Load policy rules from a TOML file.
///
/// Format:
/// ```toml
/// [[rule]]
/// toolName = "read_file"
/// decision = "allow"
/// priority = 50
///
/// [[rule]]
/// toolName = "bash"
/// decision = "ask_user"
/// priority = 10
/// modes = ["default"]
/// commandPrefix = "git"
/// ```
pub fn load_policy_toml(content: &str, tier: PolicyTier, source: &str) -> Result<Vec<PolicyRule>, String> {
    let table: toml::Value = content.parse().map_err(|e| format!("Invalid TOML: {e}"))?;
    let mut rules = Vec::new();

    let Some(rule_array) = table.get("rule").and_then(|v| v.as_array()) else {
        return Ok(rules); // No rules defined
    };

    for entry in rule_array {
        let tool_names = match entry.get("toolName") {
            Some(toml::Value::String(s)) => vec![s.clone()],
            Some(toml::Value::Array(arr)) => {
                arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
            }
            _ => continue,
        };

        let decision = match entry.get("decision").and_then(|v| v.as_str()) {
            Some("allow") => PolicyDecision::Allow,
            Some("deny") => PolicyDecision::Deny,
            Some("ask_user") | Some("ask") => PolicyDecision::AskUser,
            _ => continue,
        };

        let raw_priority = entry.get("priority").and_then(|v| v.as_integer()).unwrap_or(0);
        let priority = tier.base_priority() + (raw_priority as f64 / 1000.0);

        let modes = entry.get("modes").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().and_then(ApprovalMode::from_str))
                .collect()
        });

        let interactive = entry.get("interactive").and_then(|v| v.as_bool());

        let mcp_name = entry.get("mcpName").and_then(|v| v.as_str()).map(String::from);

        let allow_redirection = entry.get("allowRedirection").and_then(|v| v.as_bool()).unwrap_or(false);

        let deny_message = entry.get("denyMessage").and_then(|v| v.as_str()).map(String::from);

        // Build args pattern from argsPattern, commandPrefix, or commandRegex
        let args_pattern = build_args_pattern(entry);

        for tool in &tool_names {
            rules.push(PolicyRule {
                tool_name: tool.clone(),
                decision,
                priority,
                modes: modes.clone(),
                interactive,
                args_pattern: args_pattern.clone(),
                mcp_name: mcp_name.clone(),
                allow_redirection,
                deny_message: deny_message.clone(),
                source: source.to_string(),
            });
        }
    }

    rules.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
    Ok(rules)
}

fn build_args_pattern(entry: &toml::Value) -> Option<Regex> {
    // Direct argsPattern
    if let Some(pattern) = entry.get("argsPattern").and_then(|v| v.as_str()) {
        if pattern.len() > 2048 {
            return None; // Safety: prevent ReDoS
        }
        return Regex::new(pattern).ok();
    }

    // commandPrefix → regex matching "command":"<prefix>
    if let Some(prefix) = entry.get("commandPrefix").and_then(|v| v.as_str()) {
        let escaped = regex::escape(prefix);
        return Regex::new(&format!(r#""command":"{escaped}"#)).ok();
    }

    // commandRegex → regex matching "command":"<regex>
    if let Some(cmd_regex) = entry.get("commandRegex").and_then(|v| v.as_str()) {
        if cmd_regex.len() > 2048 {
            return None;
        }
        return Regex::new(&format!(r#""command":"{cmd_regex}"#)).ok();
    }

    None
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Check if a rule's tool_name pattern matches the actual tool name.
fn tool_name_matches(pattern: &str, tool_name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern == "mcp_*" {
        return tool_name.starts_with("mcp_");
    }
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        return tool_name.starts_with(prefix);
    }
    pattern == tool_name
}

/// Extract the MCP server name from a fully-qualified tool name.
/// Format: `mcp_<server>_<tool>` → `<server>`
fn extract_mcp_server(tool_name: &str) -> Option<String> {
    let stripped = tool_name.strip_prefix("mcp_")?;
    let underscore = stripped.find('_')?;
    Some(stripped[..underscore].to_string())
}

/// Load policy rules from `.openanalyst/policy.toml` in the workspace.
pub fn load_workspace_policy(workspace: &std::path::Path) -> Vec<PolicyRule> {
    let path = workspace.join(".openanalyst").join("policy.toml");
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            load_policy_toml(&content, PolicyTier::Workspace, &format!("workspace:{}", path.display()))
                .unwrap_or_default()
        }
        Err(_) => Vec::new(),
    }
}

/// Load policy rules from `~/.openanalyst/policy.toml` (user-global).
pub fn load_user_policy() -> Vec<PolicyRule> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    if home.is_empty() {
        return Vec::new();
    }
    let path = std::path::Path::new(&home).join(".openanalyst").join("policy.toml");
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            load_policy_toml(&content, PolicyTier::User, &format!("user:{}", path.display()))
                .unwrap_or_default()
        }
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_engine_allows_read() {
        let engine = PolicyEngine::with_defaults();
        let result = engine.check("read_file", None, true);
        assert_eq!(result.decision, PolicyDecision::Allow);
    }

    #[test]
    fn default_engine_asks_for_bash() {
        let engine = PolicyEngine::with_defaults();
        let result = engine.check("bash", None, true);
        assert_eq!(result.decision, PolicyDecision::AskUser);
    }

    #[test]
    fn yolo_mode_allows_everything() {
        let mut engine = PolicyEngine::with_defaults();
        engine.set_approval_mode(ApprovalMode::Yolo);
        let result = engine.check("bash", None, true);
        assert_eq!(result.decision, PolicyDecision::Allow);
    }

    #[test]
    fn plan_mode_denies_write() {
        let mut engine = PolicyEngine::with_defaults();
        engine.set_approval_mode(ApprovalMode::Plan);
        let result = engine.check("write_file", None, true);
        assert_eq!(result.decision, PolicyDecision::Deny);
    }

    #[test]
    fn plan_mode_allows_read() {
        let mut engine = PolicyEngine::with_defaults();
        engine.set_approval_mode(ApprovalMode::Plan);
        let result = engine.check("read_file", None, true);
        assert_eq!(result.decision, PolicyDecision::Allow);
    }

    #[test]
    fn non_interactive_denies_ask_user() {
        let mut engine = PolicyEngine::with_defaults();
        engine.non_interactive = true;
        let result = engine.check("bash", None, false);
        assert_eq!(result.decision, PolicyDecision::Deny);
    }

    #[test]
    fn wildcard_matching() {
        assert!(tool_name_matches("*", "anything"));
        assert!(tool_name_matches("mcp_*", "mcp_github_list"));
        assert!(!tool_name_matches("mcp_*", "read_file"));
        assert!(tool_name_matches("mcp_github_*", "mcp_github_list_repos"));
        assert!(!tool_name_matches("mcp_github_*", "mcp_slack_send"));
    }

    #[test]
    fn extract_mcp_server_name() {
        assert_eq!(extract_mcp_server("mcp_github_list"), Some("github".to_string()));
        assert_eq!(extract_mcp_server("mcp_slack_send_message"), Some("slack".to_string()));
        assert_eq!(extract_mcp_server("read_file"), None);
    }

    #[test]
    fn load_toml_basic() {
        let toml = r#"
[[rule]]
toolName = "read_file"
decision = "allow"
priority = 50

[[rule]]
toolName = "bash"
decision = "deny"
priority = 100
denyMessage = "Bash is disabled"
"#;
        let rules = load_policy_toml(toml, PolicyTier::Workspace, "test").unwrap();
        assert_eq!(rules.len(), 2);
        // Higher priority first
        assert_eq!(rules[0].tool_name, "bash");
        assert_eq!(rules[0].decision, PolicyDecision::Deny);
        assert_eq!(rules[1].tool_name, "read_file");
    }

    #[test]
    fn load_toml_with_command_prefix() {
        let toml = r#"
[[rule]]
toolName = "bash"
decision = "allow"
priority = 50
commandPrefix = "git"
"#;
        let rules = load_policy_toml(toml, PolicyTier::User, "test").unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].args_pattern.is_some());
        let pattern = rules[0].args_pattern.as_ref().unwrap();
        assert!(pattern.is_match(r#"{"command":"git status"}"#));
        assert!(!pattern.is_match(r#"{"command":"rm -rf /"}"#));
    }

    #[test]
    fn custom_rule_overrides_default() {
        let mut engine = PolicyEngine::with_defaults();
        engine.add_rule(PolicyRule {
            tool_name: "read_file".to_string(),
            decision: PolicyDecision::Deny,
            priority: 5.0, // Higher than default tier 1
            modes: None,
            interactive: None,
            args_pattern: None,
            mcp_name: None,
            allow_redirection: false,
            deny_message: Some("Custom deny".to_string()),
            source: "custom".to_string(),
        });
        let result = engine.check("read_file", None, true);
        assert_eq!(result.decision, PolicyDecision::Deny);
        assert_eq!(result.deny_message.as_deref(), Some("Custom deny"));
    }
}
