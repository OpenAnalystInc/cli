//! Custom Ratatui widgets for the OpenAnalyst TUI.
//!
//! Reuses battle-tested ecosystem crates where possible:
//! - `tui-markdown` for markdown rendering (with syntax highlighting)
//! - `edtui` for vim-mode text input
//! - `tui-tree-widget` for file tree sidebar
//! - `throbber-widgets-tui` for animated spinners
//!
//! Only tool call cards, agent cards, permission dialogs, and the status bar
//! are custom widgets.

pub mod agent_card;
pub mod file_tree;
pub mod input_box;
pub mod markdown;
pub mod permission_dialog;
pub mod spinner;
pub mod status_bar;
pub mod tool_card;

pub use agent_card::AgentStatusCard;
pub use file_tree::FileTree;
pub use input_box::{InputBox, InputBoxState, InputMode};
pub use markdown::MarkdownStream;
pub use permission_dialog::PermissionDialog;
pub use spinner::OaSpinner;
pub use status_bar::StatusBar;
pub use tool_card::{ToolCallCard, ToolCallStatus};
