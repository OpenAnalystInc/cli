//! Settings.json — user and project level configuration.
//!
//! Two-level config: user (`~/.openanalyst/settings.json`) is base,
//! project (`.openanalyst/settings.json`) overrides field-by-field.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// OpenAnalyst CLI settings loaded from settings.json.
///
/// Three-level config: managed (enforced) > project > user.
/// Settings follow the same structure as Claude Code's settings.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    // ── Basic ────────────────────────────────────────────────────────────────

    /// Default permission mode: "default", "plan", "acceptEdits", "auto", "dontAsk", "bypassPermissions".
    #[serde(default)]
    pub default_mode: Option<String>,

    /// Default model for new sessions.
    #[serde(default)]
    pub model: Option<String>,

    /// Effort level: "low", "medium", "high", "max".
    #[serde(default)]
    pub effort: Option<String>,

    /// Default permission mode (legacy alias for defaultMode).
    #[serde(default)]
    pub permissions: Option<String>,

    /// UI theme.
    #[serde(default)]
    pub theme: Option<String>,

    /// Custom system prompt additions.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Maximum turns per agent before forced stop.
    #[serde(default)]
    pub max_turns: Option<u32>,

    /// Auto-compact sessions when they get large.
    #[serde(default = "default_true")]
    pub auto_compact: bool,

    /// Include Co-Authored-By in commits.
    #[serde(default = "default_true")]
    pub include_co_authored_by: bool,

    /// Enable auto memory system.
    #[serde(default = "default_true")]
    pub auto_memory_enabled: bool,

    /// Custom auto memory directory path.
    #[serde(default)]
    pub auto_memory_directory: Option<String>,

    // ── Environment & Paths ──────────────────────────────────────────────────

    /// Environment variables to set for all tool executions.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Additional directories to include in context.
    #[serde(default)]
    pub additional_directories: Vec<String>,

    /// Glob patterns to exclude OPENANALYST.md files.
    #[serde(default)]
    pub openanalyst_md_excludes: Vec<String>,

    /// Glob patterns for file suggestions.
    #[serde(default)]
    pub file_suggestion_patterns: Vec<String>,

    /// Glob patterns to exclude from context.
    #[serde(default)]
    pub exclude_from_context: Vec<String>,

    // ── Permissions ──────────────────────────────────────────────────────────

    /// Permission rules: allow, ask, deny.
    #[serde(default)]
    pub permission_rules: PermissionRulesSettings,

    /// Auto mode configuration (environment context, soft denies).
    #[serde(default)]
    pub auto_mode: Option<AutoModeSettings>,

    // ── MCP & Plugins ────────────────────────────────────────────────────────

    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerEntry>,

    /// Allowed MCP servers (allowlist).
    #[serde(default)]
    pub allowed_mcp_servers: Vec<String>,

    /// Denied MCP servers (denylist).
    #[serde(default)]
    pub denied_mcp_servers: Vec<String>,

    /// Plugin settings.
    #[serde(default)]
    pub plugins: PluginSettings,

    /// Enabled plugins map.
    #[serde(default)]
    pub enabled_plugins: HashMap<String, bool>,

    // ── Hooks ────────────────────────────────────────────────────────────────

    /// Hook configurations.
    #[serde(default)]
    pub hooks: HookSettings,

    /// Disable all hooks.
    #[serde(default)]
    pub disable_all_hooks: bool,

    // ── Skills ───────────────────────────────────────────────────────────────

    /// Skill settings.
    #[serde(default)]
    pub skills: SkillSettings,

    // ── Sandbox ──────────────────────────────────────────────────────────────

    /// Sandbox configuration for filesystem and network isolation.
    #[serde(default)]
    pub sandbox: Option<SandboxSettings>,

    // ── Model & Capability ───────────────────────────────────────────────────

    /// Model aliases (e.g., {"fast": "openanalyst-mini"}).
    #[serde(default)]
    pub model_aliases: HashMap<String, String>,

    /// Extended thinking configuration.
    #[serde(default)]
    pub extended_thinking: Option<ExtendedThinkingSettings>,

    /// Allow model selection in UI.
    #[serde(default = "default_true")]
    pub allow_model_selection: bool,

    /// Force a specific model (overrides user selection).
    #[serde(default)]
    pub force_model: Option<String>,

    /// Fallback model when primary is unavailable.
    #[serde(default)]
    pub fallback_model: Option<String>,

    // ── Subagents ────────────────────────────────────────────────────────────

    /// Subagent configuration: disable specific built-in subagents.
    #[serde(default)]
    pub subagents: SubagentSettings,

    // ── Terminal & Output ────────────────────────────────────────────────────

    /// Terminal theme: "auto", "dark", "light".
    #[serde(default)]
    pub terminal_theme: Option<String>,

    /// Vi mode enabled.
    #[serde(default)]
    pub vi_mode_enabled: bool,

    /// Enable fast mode (same model, faster output).
    #[serde(default)]
    pub fast_mode_enabled: bool,

    // ── Status Line ──────────────────────────────────────────────────────────

    /// Status line configuration.
    #[serde(default)]
    pub status_line: Option<StatusLineSettings>,

    // ── Notification ─────────────────────────────────────────────────────────

    /// Notification hooks (permission_prompt, idle_prompt).
    #[serde(default)]
    pub notification_hooks: HashMap<String, String>,
}

/// Permission rules configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionRulesSettings {
    /// Tools/patterns to auto-allow.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Tools/patterns requiring confirmation.
    #[serde(default)]
    pub ask: Vec<String>,
    /// Tools/patterns to deny.
    #[serde(default)]
    pub deny: Vec<String>,
}

/// Auto mode configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoModeSettings {
    /// Environment context strings.
    #[serde(default)]
    pub environment: Vec<String>,
    /// Actions safe to auto-allow.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Actions to soft-deny (warn but don't block).
    #[serde(default)]
    pub soft_deny: Vec<String>,
}

/// Sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub filesystem: Option<SandboxFilesystemSettings>,
    #[serde(default)]
    pub network: Option<SandboxNetworkSettings>,
    #[serde(default)]
    pub auto_allow_bash_if_sandboxed: bool,
}

/// Sandbox filesystem settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxFilesystemSettings {
    #[serde(default)]
    pub allow_read: Vec<String>,
    #[serde(default)]
    pub deny_read: Vec<String>,
    #[serde(default)]
    pub allow_write: Vec<String>,
    #[serde(default)]
    pub deny_write: Vec<String>,
}

/// Sandbox network settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxNetworkSettings {
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub denied_domains: Vec<String>,
}

/// Extended thinking configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedThinkingSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub budget_tokens: Option<u32>,
}

/// Subagent configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentSettings {
    /// List of built-in subagent types to disable.
    #[serde(default)]
    pub disabled_subagents: Vec<String>,
}

/// Status line configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusLineSettings {
    /// Type: "command" or "disabled".
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    /// Command to execute for status line.
    #[serde(default)]
    pub command: Option<String>,
}

fn default_true() -> bool { true }

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_mode: None,
            model: None,
            effort: None,
            permissions: None,
            theme: None,
            system_prompt: None,
            max_turns: None,
            auto_compact: true,
            include_co_authored_by: true,
            auto_memory_enabled: true,
            auto_memory_directory: None,
            env: HashMap::new(),
            additional_directories: Vec::new(),
            openanalyst_md_excludes: Vec::new(),
            file_suggestion_patterns: Vec::new(),
            exclude_from_context: Vec::new(),
            permission_rules: PermissionRulesSettings::default(),
            auto_mode: None,
            mcp_servers: HashMap::new(),
            allowed_mcp_servers: Vec::new(),
            denied_mcp_servers: Vec::new(),
            plugins: PluginSettings::default(),
            enabled_plugins: HashMap::new(),
            hooks: HookSettings::default(),
            disable_all_hooks: false,
            skills: SkillSettings::default(),
            sandbox: None,
            model_aliases: HashMap::new(),
            extended_thinking: None,
            allow_model_selection: true,
            force_model: None,
            fallback_model: None,
            subagents: SubagentSettings::default(),
            terminal_theme: None,
            vi_mode_enabled: false,
            fast_mode_enabled: false,
            status_line: None,
            notification_hooks: HashMap::new(),
        }
    }
}

/// MCP server entry in settings.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Transport type (stdio, sse, http).
    #[serde(default = "default_stdio")]
    pub transport: String,
}

fn default_stdio() -> String { "stdio".to_string() }

/// Hook settings — all hook event types matching Claude Code.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HookSettings {
    #[serde(default)]
    pub session_start: Vec<HookMatcher>,
    #[serde(default)]
    pub session_end: Vec<HookMatcher>,
    #[serde(default)]
    pub instructions_loaded: Vec<HookMatcher>,
    #[serde(default)]
    pub user_prompt_submit: Vec<HookMatcher>,
    #[serde(default)]
    pub pre_tool_use: Vec<HookMatcher>,
    #[serde(default)]
    pub post_tool_use: Vec<HookMatcher>,
    #[serde(default)]
    pub post_tool_use_failure: Vec<HookMatcher>,
    #[serde(default)]
    pub permission_request: Vec<HookMatcher>,
    #[serde(default)]
    pub permission_denied: Vec<HookMatcher>,
    #[serde(default)]
    pub notification: Vec<HookMatcher>,
    #[serde(default)]
    pub stop: Vec<HookMatcher>,
    #[serde(default)]
    pub stop_failure: Vec<HookMatcher>,
    #[serde(default)]
    pub config_change: Vec<HookMatcher>,
    #[serde(default)]
    pub cwd_changed: Vec<HookMatcher>,
    #[serde(default)]
    pub file_changed: Vec<HookMatcher>,
    #[serde(default)]
    pub subagent_start: Vec<HookMatcher>,
    #[serde(default)]
    pub subagent_stop: Vec<HookMatcher>,
    #[serde(default)]
    pub task_created: Vec<HookMatcher>,
    #[serde(default)]
    pub task_completed: Vec<HookMatcher>,
    #[serde(default)]
    pub teammate_idle: Vec<HookMatcher>,
    #[serde(default)]
    pub pre_compact: Vec<HookMatcher>,
    #[serde(default)]
    pub post_compact: Vec<HookMatcher>,
    #[serde(default)]
    pub worktree_create: Vec<HookMatcher>,
    #[serde(default)]
    pub worktree_remove: Vec<HookMatcher>,
    #[serde(default)]
    pub elicitation: Vec<HookMatcher>,
    #[serde(default)]
    pub elicitation_result: Vec<HookMatcher>,
}

impl HookSettings {
    /// Returns true if any hook event has entries configured.
    pub fn has_entries(&self) -> bool {
        !self.session_start.is_empty()
            || !self.session_end.is_empty()
            || !self.instructions_loaded.is_empty()
            || !self.user_prompt_submit.is_empty()
            || !self.pre_tool_use.is_empty()
            || !self.post_tool_use.is_empty()
            || !self.post_tool_use_failure.is_empty()
            || !self.permission_request.is_empty()
            || !self.permission_denied.is_empty()
            || !self.notification.is_empty()
            || !self.stop.is_empty()
            || !self.stop_failure.is_empty()
            || !self.config_change.is_empty()
            || !self.cwd_changed.is_empty()
            || !self.file_changed.is_empty()
            || !self.subagent_start.is_empty()
            || !self.subagent_stop.is_empty()
            || !self.task_created.is_empty()
            || !self.task_completed.is_empty()
            || !self.teammate_idle.is_empty()
            || !self.pre_compact.is_empty()
            || !self.post_compact.is_empty()
            || !self.worktree_create.is_empty()
            || !self.worktree_remove.is_empty()
            || !self.elicitation.is_empty()
            || !self.elicitation_result.is_empty()
    }
}

/// A hook matcher — matches tool names/patterns and runs hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMatcher {
    /// Tool name or pattern to match (e.g., "Bash", "Edit|Write").
    #[serde(default)]
    pub matcher: Option<String>,
    /// Additional condition (e.g., "Bash(git push *)").
    #[serde(rename = "if", default)]
    pub condition: Option<String>,
    /// The hooks to run when matched.
    #[serde(default)]
    pub hooks: Vec<HookEntry>,
}

/// A single hook action entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEntry {
    /// Hook type: "command", "http", "prompt", "agent".
    #[serde(rename = "type")]
    pub kind: String,
    /// Shell command to execute (for type=command).
    #[serde(default)]
    pub command: Option<String>,
    /// URL for HTTP hooks (for type=http).
    #[serde(default)]
    pub url: Option<String>,
    /// HTTP headers (for type=http).
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Environment variables allowed in expansion.
    #[serde(default)]
    pub allowed_env_vars: Vec<String>,
    /// Prompt text (for type=prompt or type=agent).
    #[serde(default)]
    pub prompt: Option<String>,
    /// Model override for prompt/agent hooks.
    #[serde(default)]
    pub model: Option<String>,
    /// Timeout in seconds.
    #[serde(default)]
    pub timeout: Option<u32>,
}

/// Skill settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for SkillSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Plugin settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginSettings {
    #[serde(default)]
    pub enabled: Vec<String>,
}

// ── Loading ──────────────────────────────────────────────────────────────────

/// Load settings from both user and project levels, merging them.
/// Project settings override user settings field-by-field.
pub fn load_settings(project_dir: &Path) -> Settings {
    let user_settings = load_user_settings();
    let project_settings = load_project_settings(project_dir);
    merge_settings(user_settings, project_settings)
}

/// Load user-level settings from `~/.openanalyst/settings.json`.
pub fn load_user_settings() -> Settings {
    let home = std::env::var("OPENANALYST_CONFIG_HOME")
        .ok()
        .or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.openanalyst")))
        .or_else(|| std::env::var("USERPROFILE").ok().map(|h| format!("{h}/.openanalyst")))
        .unwrap_or_default();

    if home.is_empty() {
        return Settings::default();
    }

    let path = PathBuf::from(&home).join("settings.json");
    load_settings_file(&path)
}

/// Load project-level settings from `.openanalyst/settings.json`.
pub fn load_project_settings(project_dir: &Path) -> Settings {
    let path = project_dir.join(".openanalyst").join("settings.json");
    load_settings_file(&path)
}

fn load_settings_file(path: &Path) -> Settings {
    if !path.exists() {
        return Settings::default();
    }
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Merge two settings: project overrides user (field-by-field).
fn merge_settings(user: Settings, project: Settings) -> Settings {
    Settings {
        default_mode: project.default_mode.or(user.default_mode),
        model: project.model.or(user.model),
        effort: project.effort.or(user.effort),
        permissions: project.permissions.or(user.permissions),
        theme: project.theme.or(user.theme),
        system_prompt: project.system_prompt.or(user.system_prompt),
        max_turns: project.max_turns.or(user.max_turns),
        auto_compact: project.auto_compact,
        include_co_authored_by: project.include_co_authored_by,
        auto_memory_enabled: project.auto_memory_enabled,
        auto_memory_directory: project.auto_memory_directory.or(user.auto_memory_directory),
        env: {
            let mut merged = user.env;
            merged.extend(project.env);
            merged
        },
        additional_directories: merge_vecs(user.additional_directories, project.additional_directories),
        openanalyst_md_excludes: merge_vecs(user.openanalyst_md_excludes, project.openanalyst_md_excludes),
        file_suggestion_patterns: if project.file_suggestion_patterns.is_empty() {
            user.file_suggestion_patterns
        } else {
            project.file_suggestion_patterns
        },
        exclude_from_context: merge_vecs(user.exclude_from_context, project.exclude_from_context),
        permission_rules: PermissionRulesSettings {
            allow: merge_vecs(user.permission_rules.allow, project.permission_rules.allow),
            ask: merge_vecs(user.permission_rules.ask, project.permission_rules.ask),
            deny: merge_vecs(user.permission_rules.deny, project.permission_rules.deny),
        },
        auto_mode: project.auto_mode.or(user.auto_mode),
        mcp_servers: {
            let mut merged = user.mcp_servers;
            merged.extend(project.mcp_servers);
            merged
        },
        allowed_mcp_servers: merge_vecs(user.allowed_mcp_servers, project.allowed_mcp_servers),
        denied_mcp_servers: merge_vecs(user.denied_mcp_servers, project.denied_mcp_servers),
        plugins: PluginSettings {
            enabled: if project.plugins.enabled.is_empty() {
                user.plugins.enabled
            } else {
                project.plugins.enabled
            },
        },
        enabled_plugins: {
            let mut merged = user.enabled_plugins;
            merged.extend(project.enabled_plugins);
            merged
        },
        hooks: if project.hooks.has_entries() {
            project.hooks
        } else {
            user.hooks
        },
        disable_all_hooks: project.disable_all_hooks,
        skills: project.skills,
        sandbox: project.sandbox.or(user.sandbox),
        model_aliases: {
            let mut merged = user.model_aliases;
            merged.extend(project.model_aliases);
            merged
        },
        extended_thinking: project.extended_thinking.or(user.extended_thinking),
        allow_model_selection: project.allow_model_selection,
        force_model: project.force_model.or(user.force_model),
        fallback_model: project.fallback_model.or(user.fallback_model),
        subagents: if project.subagents.disabled_subagents.is_empty() {
            user.subagents
        } else {
            project.subagents
        },
        terminal_theme: project.terminal_theme.or(user.terminal_theme),
        vi_mode_enabled: project.vi_mode_enabled,
        fast_mode_enabled: project.fast_mode_enabled,
        status_line: project.status_line.or(user.status_line),
        notification_hooks: {
            let mut merged = user.notification_hooks;
            merged.extend(project.notification_hooks);
            merged
        },
    }
}

/// Merge two Vecs, deduplicating entries.
fn merge_vecs(base: Vec<String>, overlay: Vec<String>) -> Vec<String> {
    let mut merged = base;
    for item in overlay {
        if !merged.contains(&item) {
            merged.push(item);
        }
    }
    merged
}

/// Save settings to the project-level file.
pub fn save_project_settings(project_dir: &Path, settings: &Settings) -> Result<(), String> {
    let dir = project_dir.join(".openanalyst");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create .openanalyst/: {e}"))?;
    let path = dir.join("settings.json");
    let json = serde_json::to_string_pretty(settings).map_err(|e| format!("Serialize error: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

/// Update a single setting by key path (e.g., "model", "maxTurns").
pub fn update_setting(settings: &mut Settings, key: &str, value: &str) -> Result<String, String> {
    match key {
        "model" => {
            settings.model = Some(value.to_string());
            Ok(format!("model = {value}"))
        }
        "defaultMode" | "default_mode" => {
            settings.default_mode = Some(value.to_string());
            Ok(format!("defaultMode = {value}"))
        }
        "effort" => {
            settings.effort = Some(value.to_string());
            Ok(format!("effort = {value}"))
        }
        "permissions" => {
            settings.permissions = Some(value.to_string());
            Ok(format!("permissions = {value}"))
        }
        "theme" => {
            settings.theme = Some(value.to_string());
            Ok(format!("theme = {value}"))
        }
        "maxTurns" | "max_turns" => {
            let n: u32 = value.parse().map_err(|_| "Invalid number".to_string())?;
            settings.max_turns = Some(n);
            Ok(format!("maxTurns = {n}"))
        }
        "autoCompact" | "auto_compact" => {
            let b: bool = value.parse().map_err(|_| "Expected true/false".to_string())?;
            settings.auto_compact = b;
            Ok(format!("autoCompact = {b}"))
        }
        "includeCoAuthoredBy" => {
            let b: bool = value.parse().map_err(|_| "Expected true/false".to_string())?;
            settings.include_co_authored_by = b;
            Ok(format!("includeCoAuthoredBy = {b}"))
        }
        "autoMemoryEnabled" => {
            let b: bool = value.parse().map_err(|_| "Expected true/false".to_string())?;
            settings.auto_memory_enabled = b;
            Ok(format!("autoMemoryEnabled = {b}"))
        }
        "systemPrompt" | "system_prompt" => {
            settings.system_prompt = Some(value.to_string());
            Ok(format!("systemPrompt = {}", &value[..value.len().min(50)]))
        }
        "viModeEnabled" | "vi_mode" => {
            let b: bool = value.parse().map_err(|_| "Expected true/false".to_string())?;
            settings.vi_mode_enabled = b;
            Ok(format!("viModeEnabled = {b}"))
        }
        "fastModeEnabled" | "fast_mode" => {
            let b: bool = value.parse().map_err(|_| "Expected true/false".to_string())?;
            settings.fast_mode_enabled = b;
            Ok(format!("fastModeEnabled = {b}"))
        }
        "allowModelSelection" => {
            let b: bool = value.parse().map_err(|_| "Expected true/false".to_string())?;
            settings.allow_model_selection = b;
            Ok(format!("allowModelSelection = {b}"))
        }
        "forceModel" => {
            settings.force_model = Some(value.to_string());
            Ok(format!("forceModel = {value}"))
        }
        "fallbackModel" => {
            settings.fallback_model = Some(value.to_string());
            Ok(format!("fallbackModel = {value}"))
        }
        "terminalTheme" => {
            settings.terminal_theme = Some(value.to_string());
            Ok(format!("terminalTheme = {value}"))
        }
        "disableAllHooks" => {
            let b: bool = value.parse().map_err(|_| "Expected true/false".to_string())?;
            settings.disable_all_hooks = b;
            Ok(format!("disableAllHooks = {b}"))
        }
        _ => Err(format!(
            "Unknown setting: {key}\n\
             Available: model, defaultMode, effort, permissions, theme, maxTurns, autoCompact,\n\
             includeCoAuthoredBy, autoMemoryEnabled, systemPrompt, viModeEnabled, fastModeEnabled,\n\
             allowModelSelection, forceModel, fallbackModel, terminalTheme, disableAllHooks"
        )),
    }
}

// ── .mcp.json Loading ────────────────────────────────────────────────────────

/// Load MCP servers from `.openanalyst/.mcp.json` (project) and `~/.openanalyst/.mcp.json` (user).
pub fn load_mcp_json(project_dir: &Path) -> HashMap<String, McpServerEntry> {
    let mut servers = HashMap::new();

    // User-level .mcp.json
    let home = std::env::var("OPENANALYST_CONFIG_HOME")
        .ok()
        .or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.openanalyst")))
        .or_else(|| std::env::var("USERPROFILE").ok().map(|h| format!("{h}/.openanalyst")))
        .unwrap_or_default();
    if !home.is_empty() {
        let user_mcp = PathBuf::from(&home).join(".mcp.json");
        if let Some(user_servers) = load_mcp_json_file(&user_mcp) {
            servers.extend(user_servers);
        }
    }

    // Project-level .mcp.json (overrides user)
    let project_mcp = project_dir.join(".openanalyst").join(".mcp.json");
    if let Some(project_servers) = load_mcp_json_file(&project_mcp) {
        servers.extend(project_servers);
    }

    servers
}

fn load_mcp_json_file(path: &Path) -> Option<HashMap<String, McpServerEntry>> {
    let content = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let servers_obj = v.get("mcpServers")?.as_object()?;

    let mut result = HashMap::new();
    for (name, config) in servers_obj {
        let command = config.get("command").and_then(|c| c.as_str())?.to_string();
        let args: Vec<String> = config.get("args")
            .and_then(|a| a.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let env: HashMap<String, String> = config.get("env")
            .and_then(|e| e.as_object())
            .map(|obj| obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
            .unwrap_or_default();

        result.insert(name.clone(), McpServerEntry {
            command,
            args,
            env,
            transport: "stdio".to_string(),
        });
    }

    Some(result)
}

/// Save an MCP server to the project-level .mcp.json.
pub fn save_mcp_server(project_dir: &Path, name: &str, entry: &McpServerEntry) -> Result<(), String> {
    let dir = project_dir.join(".openanalyst");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir error: {e}"))?;
    let path = dir.join(".mcp.json");

    let mut existing: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let servers = existing.as_object_mut()
        .and_then(|obj| obj.entry("mcpServers").or_insert(serde_json::json!({})).as_object_mut());

    if let Some(servers) = servers {
        servers.insert(name.to_string(), serde_json::json!({
            "command": entry.command,
            "args": entry.args,
        }));
    }

    let json = serde_json::to_string_pretty(&existing).map_err(|e| format!("Serialize: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {e}"))?;
    Ok(())
}

/// Remove an MCP server from the project-level .mcp.json.
pub fn remove_mcp_server(project_dir: &Path, name: &str) -> Result<(), String> {
    let path = project_dir.join(".openanalyst").join(".mcp.json");
    if !path.exists() {
        return Err("No .mcp.json found".to_string());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("Read: {e}"))?;
    let mut v: serde_json::Value = serde_json::from_str(&content).map_err(|e| format!("Parse: {e}"))?;

    if let Some(servers) = v.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
        servers.remove(name);
    }

    let json = serde_json::to_string_pretty(&v).map_err(|e| format!("Serialize: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings() {
        let s = Settings::default();
        assert!(s.model.is_none());
        assert!(s.auto_compact);
        assert!(s.skills.enabled);
    }

    #[test]
    fn merge_project_overrides_user() {
        let user = Settings { model: Some("sonnet".to_string()), ..Default::default() };
        let project = Settings { model: Some("opus".to_string()), ..Default::default() };
        let merged = merge_settings(user, project);
        assert_eq!(merged.model.as_deref(), Some("opus"));
    }

    #[test]
    fn merge_falls_back_to_user() {
        let user = Settings { model: Some("sonnet".to_string()), ..Default::default() };
        let project = Settings::default();
        let merged = merge_settings(user, project);
        assert_eq!(merged.model.as_deref(), Some("sonnet"));
    }

    #[test]
    fn update_setting_works() {
        let mut s = Settings::default();
        assert!(update_setting(&mut s, "model", "opus").is_ok());
        assert_eq!(s.model.as_deref(), Some("opus"));
        assert!(update_setting(&mut s, "maxTurns", "50").is_ok());
        assert_eq!(s.max_turns, Some(50));
        assert!(update_setting(&mut s, "unknown", "x").is_err());
    }
}
