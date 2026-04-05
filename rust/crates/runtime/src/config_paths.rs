//! Shared configuration path resolution for all three config levels:
//!
//! 1. **Managed/Global** — Organization-wide enforced settings (cannot be overridden)
//!    - Windows: `C:\Program Files\OpenAnalystCLI\`
//!    - macOS: `/Library/Application Support/OpenAnalystCLI/`
//!    - Linux/WSL: `/etc/openanalyst-cli/`
//!
//! 2. **User** — Personal settings (all projects)
//!    - `~/.openanalyst/`
//!    - Override: `OPENANALYST_CONFIG_HOME` env var
//!
//! 3. **Project** — Repository-specific settings
//!    - `.openanalyst/` (in project root)

use std::path::PathBuf;

/// Returns the managed/global configuration directory (organization-wide).
/// This is read-only from the CLI's perspective — set by IT/admin.
#[must_use]
pub fn managed_config_home() -> Option<PathBuf> {
    // Check environment override first
    if let Ok(val) = std::env::var("OPENANALYST_MANAGED_CONFIG") {
        let p = PathBuf::from(val);
        if p.is_dir() {
            return Some(p);
        }
    }

    #[cfg(target_os = "windows")]
    {
        let path = PathBuf::from(r"C:\Program Files\OpenAnalystCLI");
        if path.is_dir() {
            return Some(path);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let path = PathBuf::from("/Library/Application Support/OpenAnalystCLI");
        if path.is_dir() {
            return Some(path);
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let path = PathBuf::from("/etc/openanalyst-cli");
        if path.is_dir() {
            return Some(path);
        }
    }

    None
}

/// Returns the user-level configuration directory (`~/.openanalyst/`).
#[must_use]
pub fn user_config_home() -> Option<PathBuf> {
    std::env::var("OPENANALYST_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok()
                .map(|h| PathBuf::from(h).join(".openanalyst"))
        })
}

/// Returns the project-level configuration directory (`.openanalyst/`).
#[must_use]
pub fn project_config_dir(project_dir: &std::path::Path) -> PathBuf {
    project_dir.join(".openanalyst")
}

/// Configuration scope — managed > user > project for enforcement;
/// project > user > managed for user customization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigLevel {
    /// Organization-wide enforced settings (highest enforcement priority).
    Managed,
    /// Personal user-level settings.
    User,
    /// Repository/project-level settings (highest customization priority).
    Project,
}

impl ConfigLevel {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::User => "user",
            Self::Project => "project",
        }
    }
}
