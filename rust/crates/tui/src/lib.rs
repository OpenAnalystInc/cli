//! TUI crate for OpenAnalyst CLI.
//!
//! The ratatui-based TUI has been archived to `_archived/ratatui-tui/`.
//! This crate now only provides:
//! - `headless` — JSON-RPC bridge for the Ink TUI
//! - `BannerAccountInfo` — account info struct used by the CLI entry point

pub mod headless;

/// Account info for the startup banner display.
///
/// This struct is used by the CLI entry point to pass account info
/// to both the headless bridge and legacy TUI codepaths.
/// Kept here (rather than in a shared crate) to minimize churn.
#[derive(Debug, Clone, Default)]
pub struct BannerAccountInfo {
    pub display_name: String,
    pub model_display: String,
    pub provider_name: String,
    pub user_email: Option<String>,
    pub credits: Option<String>,
    pub organization: Option<String>,
    pub cwd: String,
    pub version: String,
}
