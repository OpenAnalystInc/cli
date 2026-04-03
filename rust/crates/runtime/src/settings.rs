//! Settings.json — user and project level configuration.
//!
//! Two-level config: user (`~/.openanalyst/settings.json`) is base,
//! project (`.openanalyst/settings.json`) overrides field-by-field.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// OpenAnalyst CLI settings loaded from settings.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Default model for new sessions.
    #[serde(default)]
    pub model: Option<String>,

    /// Default permission mode.
    #[serde(default)]
    pub permissions: Option<String>,

    /// UI theme.
    #[serde(default)]
    pub theme: Option<String>,

    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerEntry>,

    /// Hook configurations.
    #[serde(default)]
    pub hooks: HookSettings,

    /// Skill settings.
    #[serde(default)]
    pub skills: SkillSettings,

    /// Plugin settings.
    #[serde(default)]
    pub plugins: PluginSettings,

    /// Maximum turns per agent before forced stop.
    #[serde(default)]
    pub max_turns: Option<u32>,

    /// Auto-compact sessions when they get large.
    #[serde(default = "default_true")]
    pub auto_compact: bool,

    /// Custom system prompt additions.
    #[serde(default)]
    pub system_prompt: Option<String>,
}

fn default_true() -> bool { true }

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: None,
            permissions: None,
            theme: None,
            mcp_servers: HashMap::new(),
            hooks: HookSettings::default(),
            skills: SkillSettings::default(),
            plugins: PluginSettings::default(),
            max_turns: None,
            auto_compact: true,
            system_prompt: None,
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

/// Hook settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSettings {
    #[serde(default)]
    pub pre_tool_use: Vec<HookEntry>,
    #[serde(default)]
    pub post_tool_use: Vec<HookEntry>,
}

/// A single hook entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEntry {
    pub command: String,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
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
        model: project.model.or(user.model),
        permissions: project.permissions.or(user.permissions),
        theme: project.theme.or(user.theme),
        mcp_servers: {
            let mut merged = user.mcp_servers;
            merged.extend(project.mcp_servers);
            merged
        },
        hooks: if !project.hooks.pre_tool_use.is_empty() || !project.hooks.post_tool_use.is_empty() {
            project.hooks
        } else {
            user.hooks
        },
        skills: project.skills,
        plugins: PluginSettings {
            enabled: if project.plugins.enabled.is_empty() {
                user.plugins.enabled
            } else {
                project.plugins.enabled
            },
        },
        max_turns: project.max_turns.or(user.max_turns),
        auto_compact: project.auto_compact,
        system_prompt: project.system_prompt.or(user.system_prompt),
    }
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
        "systemPrompt" | "system_prompt" => {
            settings.system_prompt = Some(value.to_string());
            Ok(format!("systemPrompt = {}", &value[..value.len().min(50)]))
        }
        _ => Err(format!("Unknown setting: {key}\nAvailable: model, permissions, theme, maxTurns, autoCompact, systemPrompt")),
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
