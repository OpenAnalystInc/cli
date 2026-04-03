//! Folder trust system — workspace trust zone discovery and management.
//!
//! Determines whether the current workspace is "trusted" — meaning user has
//! explicitly allowed hooks, skills, and plugins to run from this directory.
//!
//! Trust is established by:
//! 1. Presence of `.openanalyst/trust.json` in the workspace root
//! 2. User's global trust list in `~/.openanalyst/trusted_folders.json`
//! 3. Explicit `--trust` CLI flag
//!
//! Untrusted folders: hooks and skills are disabled, plugins require confirmation.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ── Types ────────────────────────────────────────────────────────────────────

/// Trust level for a workspace folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    /// Fully trusted — hooks, skills, plugins all active.
    Trusted,
    /// Not explicitly trusted — hooks/skills disabled, plugins require confirmation.
    Untrusted,
    /// Explicitly blocked — nothing runs.
    Blocked,
}

/// Result of trust discovery.
#[derive(Debug, Clone)]
pub struct TrustInfo {
    pub level: TrustLevel,
    pub reason: String,
    pub workspace_root: PathBuf,
}

// ── Trust Discovery ──────────────────────────────────────────────────────────

/// Discover the trust level for the given workspace directory.
pub fn discover_trust(workspace: &Path) -> TrustInfo {
    let workspace = workspace.to_path_buf();

    // 1. Check local trust marker: .openanalyst/trust.json
    let local_trust = workspace.join(".openanalyst").join("trust.json");
    if local_trust.exists() {
        if let Ok(content) = std::fs::read_to_string(&local_trust) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                if parsed.get("trusted") == Some(&serde_json::Value::Bool(true)) {
                    return TrustInfo {
                        level: TrustLevel::Trusted,
                        reason: "Local .openanalyst/trust.json".to_string(),
                        workspace_root: workspace,
                    };
                }
                if parsed.get("blocked") == Some(&serde_json::Value::Bool(true)) {
                    return TrustInfo {
                        level: TrustLevel::Blocked,
                        reason: "Explicitly blocked in .openanalyst/trust.json".to_string(),
                        workspace_root: workspace,
                    };
                }
            }
        }
    }

    // 2. Check user's global trust list
    if let Some(home_dir) = home_config_dir() {
        let global_trust = home_dir.join("trusted_folders.json");
        if global_trust.exists() {
            if let Ok(content) = std::fs::read_to_string(&global_trust) {
                if let Ok(folders) = serde_json::from_str::<Vec<String>>(&content) {
                    let trusted_set: HashSet<String> = folders.into_iter().collect();
                    let ws_str = workspace.to_string_lossy().to_string();
                    // Check exact match and parent matches
                    if trusted_set.contains(&ws_str) {
                        return TrustInfo {
                            level: TrustLevel::Trusted,
                            reason: "Listed in global trusted_folders.json".to_string(),
                            workspace_root: workspace,
                        };
                    }
                    // Check if any trusted folder is a parent
                    for trusted in &trusted_set {
                        if ws_str.starts_with(trusted) {
                            return TrustInfo {
                                level: TrustLevel::Trusted,
                                reason: format!("Child of trusted folder: {trusted}"),
                                workspace_root: workspace,
                            };
                        }
                    }
                }
            }
        }
    }

    // 3. Check if it's the user's home directory (always trusted)
    if let Some(home) = get_home_dir() {
        if workspace == home {
            return TrustInfo {
                level: TrustLevel::Trusted,
                reason: "Home directory".to_string(),
                workspace_root: workspace,
            };
        }
    }

    // 4. Default: untrusted
    TrustInfo {
        level: TrustLevel::Untrusted,
        reason: "No trust marker found. Run /init or add to trusted_folders.json".to_string(),
        workspace_root: workspace,
    }
}

/// Add a folder to the user's global trust list.
pub fn trust_folder(workspace: &Path) -> Result<(), String> {
    let config_dir = home_config_dir().ok_or("Cannot find home config directory")?;
    let _ = std::fs::create_dir_all(&config_dir);
    let global_trust = config_dir.join("trusted_folders.json");

    let mut folders: Vec<String> = if global_trust.exists() {
        let content = std::fs::read_to_string(&global_trust)
            .map_err(|e| format!("Failed to read trusted_folders.json: {e}"))?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    let ws_str = workspace.to_string_lossy().to_string();
    if !folders.contains(&ws_str) {
        folders.push(ws_str);
        let json = serde_json::to_string_pretty(&folders)
            .map_err(|e| format!("Failed to serialize: {e}"))?;
        std::fs::write(&global_trust, json)
            .map_err(|e| format!("Failed to write trusted_folders.json: {e}"))?;
    }

    // Also create local trust marker
    let local_dir = workspace.join(".openanalyst");
    let _ = std::fs::create_dir_all(&local_dir);
    let _ = std::fs::write(
        local_dir.join("trust.json"),
        r#"{"trusted": true}"#,
    );

    Ok(())
}

/// Remove a folder from the user's global trust list.
pub fn untrust_folder(workspace: &Path) -> Result<(), String> {
    let config_dir = home_config_dir().ok_or("Cannot find home config directory")?;
    let global_trust = config_dir.join("trusted_folders.json");

    if !global_trust.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&global_trust)
        .map_err(|e| format!("Failed to read: {e}"))?;
    let mut folders: Vec<String> = serde_json::from_str(&content).unwrap_or_default();

    let ws_str = workspace.to_string_lossy().to_string();
    folders.retain(|f| f != &ws_str);

    let json = serde_json::to_string_pretty(&folders)
        .map_err(|e| format!("Failed to serialize: {e}"))?;
    std::fs::write(&global_trust, json)
        .map_err(|e| format!("Failed to write: {e}"))?;

    // Remove local trust marker
    let local_trust = workspace.join(".openanalyst").join("trust.json");
    let _ = std::fs::remove_file(local_trust);

    Ok(())
}

/// Get the user's home directory.
fn get_home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

/// Get the OpenAnalyst config directory under user's home.
fn home_config_dir() -> Option<PathBuf> {
    std::env::var("OPENANALYST_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            get_home_dir().map(|h| h.join(".openanalyst"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn untrusted_by_default() {
        let tmp = std::env::temp_dir().join("oa_trust_test_default");
        let _ = fs::create_dir_all(&tmp);
        let info = discover_trust(&tmp);
        assert_eq!(info.level, TrustLevel::Untrusted);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn trusted_with_local_marker() {
        let tmp = std::env::temp_dir().join("oa_trust_test_local");
        let oa_dir = tmp.join(".openanalyst");
        let _ = fs::create_dir_all(&oa_dir);
        fs::write(oa_dir.join("trust.json"), r#"{"trusted": true}"#).unwrap();

        let info = discover_trust(&tmp);
        assert_eq!(info.level, TrustLevel::Trusted);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn blocked_with_local_marker() {
        let tmp = std::env::temp_dir().join("oa_trust_test_blocked");
        let oa_dir = tmp.join(".openanalyst");
        let _ = fs::create_dir_all(&oa_dir);
        fs::write(oa_dir.join("trust.json"), r#"{"blocked": true}"#).unwrap();

        let info = discover_trust(&tmp);
        assert_eq!(info.level, TrustLevel::Blocked);
        let _ = fs::remove_dir_all(&tmp);
    }
}
