//! User-configurable keybindings for the OpenAnalyst CLI TUI.
//!
//! Users can customize keybindings by creating `.openanalyst/keybindings.json`:
//! ```json
//! {
//!   "cancel": "ctrl+c",
//!   "quit": "ctrl+q",
//!   "sidebar_toggle": "ctrl+b",
//!   "clear": "ctrl+l",
//!   "scroll_mode": "escape",
//!   "history_prev": "ctrl+up",
//!   "history_next": "ctrl+down",
//!   "focus_cycle": "tab",
//!   "submit": "enter",
//!   "newline": "shift+enter",
//!   "background": "ctrl+shift+b"
//! }
//! ```
//!
//! All keys are optional — defaults are used for any unset binding.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// All configurable actions in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Cancel,
    Quit,
    SidebarToggle,
    Clear,
    ScrollMode,
    HistoryPrev,
    HistoryNext,
    FocusCycle,
    Submit,
    Newline,
    Background,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    ScrollTop,
    ScrollBottom,
    ExpandCollapse,
}

/// A parsed key combination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    pub fn matches(&self, key: &KeyEvent) -> bool {
        key.code == self.code && key.modifiers.contains(self.modifiers)
    }
}

/// The keybinding configuration with defaults.
pub struct KeybindingConfig {
    bindings: HashMap<Action, KeyCombo>,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(Action::Cancel, parse_key("ctrl+c").unwrap());
        bindings.insert(Action::Quit, parse_key("ctrl+c").unwrap());
        bindings.insert(Action::SidebarToggle, parse_key("ctrl+b").unwrap());
        bindings.insert(Action::Clear, parse_key("ctrl+l").unwrap());
        bindings.insert(Action::ScrollMode, parse_key("escape").unwrap());
        bindings.insert(Action::HistoryPrev, parse_key("ctrl+up").unwrap());
        bindings.insert(Action::HistoryNext, parse_key("ctrl+down").unwrap());
        bindings.insert(Action::FocusCycle, parse_key("tab").unwrap());
        bindings.insert(Action::Submit, parse_key("enter").unwrap());
        bindings.insert(Action::Newline, parse_key("shift+enter").unwrap());
        bindings.insert(Action::Background, parse_key("ctrl+shift+b").unwrap());
        bindings.insert(Action::ScrollUp, parse_key("k").unwrap());
        bindings.insert(Action::ScrollDown, parse_key("j").unwrap());
        bindings.insert(Action::ScrollPageUp, parse_key("pageup").unwrap());
        bindings.insert(Action::ScrollPageDown, parse_key("pagedown").unwrap());
        bindings.insert(Action::ScrollTop, parse_key("g").unwrap());
        bindings.insert(Action::ScrollBottom, parse_key("shift+g").unwrap());
        bindings.insert(Action::ExpandCollapse, parse_key("enter").unwrap());
        Self { bindings }
    }
}

impl KeybindingConfig {
    /// Load keybindings from `.openanalyst/keybindings.json`, merging with defaults.
    pub fn load() -> Self {
        let mut config = Self::default();

        let path = std::path::Path::new(".openanalyst").join("keybindings.json");
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(user_bindings) = serde_json::from_str::<HashMap<String, String>>(&content) {
                for (action_str, key_str) in &user_bindings {
                    if let (Some(action), Some(combo)) = (
                        parse_action(action_str),
                        parse_key(key_str),
                    ) {
                        config.bindings.insert(action, combo);
                    }
                }
            }
        }

        config
    }

    /// Check if a key event matches a specific action.
    pub fn matches(&self, key: &KeyEvent, action: Action) -> bool {
        self.bindings
            .get(&action)
            .map_or(false, |combo| combo.matches(key))
    }

    /// Get the display string for a keybinding (for hints).
    pub fn display(&self, action: Action) -> String {
        self.bindings
            .get(&action)
            .map_or("?".to_string(), |combo| format_key(combo))
    }

    /// Render all keybindings as a help string.
    pub fn render_help(&self) -> String {
        let mut lines = Vec::new();
        lines.push("Keybindings (customize in .openanalyst/keybindings.json):".to_string());
        lines.push(String::new());

        let pairs: Vec<(Action, &str)> = vec![
            (Action::Cancel, "Cancel / Quit"),
            (Action::SidebarToggle, "Toggle sidebar"),
            (Action::Clear, "Clear chat"),
            (Action::ScrollMode, "Toggle scroll mode"),
            (Action::HistoryPrev, "Previous input history"),
            (Action::HistoryNext, "Next input history"),
            (Action::FocusCycle, "Cycle focus (input→chat→sidebar)"),
            (Action::Submit, "Send message"),
            (Action::Newline, "Insert newline"),
            (Action::Background, "Run in background"),
            (Action::ScrollUp, "Scroll up (in scroll mode)"),
            (Action::ScrollDown, "Scroll down (in scroll mode)"),
            (Action::ScrollPageUp, "Page up"),
            (Action::ScrollPageDown, "Page down"),
            (Action::ExpandCollapse, "Expand/collapse tool card"),
        ];

        for (action, desc) in pairs {
            let key_str = self.display(action);
            lines.push(format!("  {:<20} {}", key_str, desc));
        }

        lines.join("\n")
    }
}

/// Parse an action name from JSON.
fn parse_action(s: &str) -> Option<Action> {
    match s.to_ascii_lowercase().replace('-', "_").as_str() {
        "cancel" => Some(Action::Cancel),
        "quit" => Some(Action::Quit),
        "sidebar_toggle" | "sidebar" => Some(Action::SidebarToggle),
        "clear" => Some(Action::Clear),
        "scroll_mode" | "scroll" => Some(Action::ScrollMode),
        "history_prev" | "history_up" => Some(Action::HistoryPrev),
        "history_next" | "history_down" => Some(Action::HistoryNext),
        "focus_cycle" | "focus" | "tab" => Some(Action::FocusCycle),
        "submit" | "send" => Some(Action::Submit),
        "newline" => Some(Action::Newline),
        "background" | "bg" => Some(Action::Background),
        "scroll_up" => Some(Action::ScrollUp),
        "scroll_down" => Some(Action::ScrollDown),
        "page_up" | "scroll_page_up" => Some(Action::ScrollPageUp),
        "page_down" | "scroll_page_down" => Some(Action::ScrollPageDown),
        "scroll_top" | "top" => Some(Action::ScrollTop),
        "scroll_bottom" | "bottom" => Some(Action::ScrollBottom),
        "expand" | "expand_collapse" => Some(Action::ExpandCollapse),
        _ => None,
    }
}

/// Parse a key string like "ctrl+shift+b" into a KeyCombo.
pub fn parse_key(s: &str) -> Option<KeyCombo> {
    let parts: Vec<&str> = s.split('+').map(str::trim).collect();
    let mut modifiers = KeyModifiers::empty();
    let mut key_str = "";

    for part in &parts {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "alt" => modifiers |= KeyModifiers::ALT,
            _ => key_str = part,
        }
    }

    let code = match key_str.to_ascii_lowercase().as_str() {
        "enter" | "return" => KeyCode::Enter,
        "escape" | "esc" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "backspace" | "bs" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "space" | " " => KeyCode::Char(' '),
        s if s.len() == 1 => {
            let c = s.chars().next()?;
            if modifiers.contains(KeyModifiers::SHIFT) && c.is_ascii_alphabetic() {
                KeyCode::Char(c.to_ascii_uppercase())
            } else {
                KeyCode::Char(c)
            }
        }
        _ => return None,
    };

    Some(KeyCombo { code, modifiers })
}

/// Format a KeyCombo as a display string.
fn format_key(combo: &KeyCombo) -> String {
    let mut parts = Vec::new();
    if combo.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if combo.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift");
    }
    if combo.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt");
    }

    let key_name = match combo.code {
        KeyCode::Enter => "Enter",
        KeyCode::Esc => "Esc",
        KeyCode::Tab => "Tab",
        KeyCode::BackTab => "BackTab",
        KeyCode::Backspace => "Backspace",
        KeyCode::Delete => "Delete",
        KeyCode::Up => "Up",
        KeyCode::Down => "Down",
        KeyCode::Left => "Left",
        KeyCode::Right => "Right",
        KeyCode::PageUp => "PageUp",
        KeyCode::PageDown => "PageDown",
        KeyCode::Home => "Home",
        KeyCode::End => "End",
        KeyCode::Char(' ') => "Space",
        KeyCode::Char(c) => {
            let mut result = parts.join("+");
            if !result.is_empty() {
                result.push('+');
            }
            result.push(c.to_ascii_uppercase());
            return result;
        }
        _ => "?",
    };
    parts.push(key_name);
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_keys() {
        let combo = parse_key("ctrl+c").unwrap();
        assert_eq!(combo.code, KeyCode::Char('c'));
        assert!(combo.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn parse_shift_enter() {
        let combo = parse_key("shift+enter").unwrap();
        assert_eq!(combo.code, KeyCode::Enter);
        assert!(combo.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn parse_ctrl_shift_b() {
        let combo = parse_key("ctrl+shift+b").unwrap();
        assert_eq!(combo.code, KeyCode::Char('B'));
        assert!(combo.modifiers.contains(KeyModifiers::CONTROL));
        assert!(combo.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn default_config_has_all_bindings() {
        let config = KeybindingConfig::default();
        assert!(config.bindings.contains_key(&Action::Cancel));
        assert!(config.bindings.contains_key(&Action::Submit));
        assert!(config.bindings.contains_key(&Action::Newline));
        assert!(config.bindings.contains_key(&Action::Background));
    }

    #[test]
    fn matches_key_event() {
        let config = KeybindingConfig::default();
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(config.matches(&ctrl_c, Action::Cancel));
        assert!(!config.matches(&ctrl_c, Action::Submit));
    }

    #[test]
    fn render_help_is_nonempty() {
        let config = KeybindingConfig::default();
        let help = config.render_help();
        assert!(help.contains("Cancel"));
        assert!(help.contains("Ctrl+c"));
    }
}
