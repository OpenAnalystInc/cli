//! IDE detection — detects the user's IDE/editor from environment variables.
//!
//! Detects which IDE/editor the user is running in based on environment variables
//! and process ancestry. Used for context-aware behavior and extension integration.

use std::env;

/// Known IDE identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ide {
    VsCode,
    VsCodeInsiders,
    Cursor,
    Windsurf,
    Void,
    Positron,
    Zed,
    Replit,
    CloudShell,
    IntelliJIdea,
    WebStorm,
    PyCharm,
    GoLand,
    RustRover,
    Rider,
    PhpStorm,
    CLion,
    DataGrip,
    AndroidStudio,
    Fleet,
    Xcode,
    Neovim,
    Vim,
    Emacs,
    SublimeText,
    Terminal,
    Unknown,
}

impl Ide {
    /// Human-readable display name.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::VsCode => "VS Code",
            Self::VsCodeInsiders => "VS Code Insiders",
            Self::Cursor => "Cursor",
            Self::Windsurf => "Windsurf",
            Self::Void => "Void",
            Self::Positron => "Positron",
            Self::Zed => "Zed",
            Self::Replit => "Replit",
            Self::CloudShell => "Cloud Shell",
            Self::IntelliJIdea => "IntelliJ IDEA",
            Self::WebStorm => "WebStorm",
            Self::PyCharm => "PyCharm",
            Self::GoLand => "GoLand",
            Self::RustRover => "RustRover",
            Self::Rider => "Rider",
            Self::PhpStorm => "PhpStorm",
            Self::CLion => "CLion",
            Self::DataGrip => "DataGrip",
            Self::AndroidStudio => "Android Studio",
            Self::Fleet => "Fleet",
            Self::Xcode => "Xcode",
            Self::Neovim => "Neovim",
            Self::Vim => "Vim",
            Self::Emacs => "Emacs",
            Self::SublimeText => "Sublime Text",
            Self::Terminal => "Terminal",
            Self::Unknown => "Unknown",
        }
    }

    /// Whether this IDE supports LSP integration.
    pub fn supports_lsp(self) -> bool {
        matches!(self,
            Self::VsCode | Self::VsCodeInsiders | Self::Cursor | Self::Windsurf |
            Self::Void | Self::Positron | Self::Zed | Self::Neovim |
            Self::IntelliJIdea | Self::WebStorm | Self::PyCharm | Self::GoLand |
            Self::RustRover | Self::Rider | Self::PhpStorm | Self::CLion |
            Self::DataGrip | Self::AndroidStudio | Self::Fleet |
            Self::SublimeText | Self::Emacs
        )
    }

    /// Whether this IDE has an integrated terminal.
    pub fn has_terminal(self) -> bool {
        matches!(self,
            Self::VsCode | Self::VsCodeInsiders | Self::Cursor | Self::Windsurf |
            Self::Void | Self::Positron | Self::Zed | Self::Replit |
            Self::IntelliJIdea | Self::WebStorm | Self::PyCharm | Self::GoLand |
            Self::RustRover | Self::Rider | Self::PhpStorm | Self::CLion |
            Self::DataGrip | Self::AndroidStudio | Self::Fleet
        )
    }
}

/// Detect the current IDE from environment variables.
///
/// Priority order: specific env vars first, then generic
/// terminal detection.
pub fn detect_ide() -> Ide {
    // VS Code family — detected via TERM_PROGRAM or VSCODE_*
    if env::var("TERM_PROGRAM").ok().as_deref() == Some("vscode") {
        if env::var("CURSOR_TRACE_ID").is_ok() || env::var("CURSOR_SESSION_ID").is_ok() {
            return Ide::Cursor;
        }
        if env::var("WINDSURF_SESSION_ID").is_ok() || env::var("CODEIUM_SESSION_ID").is_ok() {
            return Ide::Windsurf;
        }
        if env::var("VOID_SESSION_ID").is_ok() {
            return Ide::Void;
        }
        if env::var("POSITRON_SESSION_ID").is_ok() || env::var("POSITRON_VERSION").is_ok() {
            return Ide::Positron;
        }
        if env::var("VSCODE_CLI_VERSION").ok().map_or(false, |v| v.contains("insider")) {
            return Ide::VsCodeInsiders;
        }
        return Ide::VsCode;
    }

    // Zed
    if env::var("ZED_TERM").is_ok() || env::var("TERM_PROGRAM").ok().as_deref() == Some("zed") {
        return Ide::Zed;
    }

    // Replit
    if env::var("REPL_ID").is_ok() || env::var("REPLIT_ENVIRONMENT").is_ok() {
        return Ide::Replit;
    }

    // Google Cloud Shell
    if env::var("CLOUD_SHELL").is_ok() || env::var("GOOGLE_CLOUD_SHELL").is_ok() {
        return Ide::CloudShell;
    }

    // JetBrains family — detected via TERMINAL_EMULATOR or IDEA_INITIAL_DIRECTORY
    if env::var("TERMINAL_EMULATOR").ok().as_deref() == Some("JetBrains-JediTerm")
        || env::var("IDEA_INITIAL_DIRECTORY").is_ok()
    {
        // Try to distinguish JetBrains product
        let idea_paths = env::var("IDEA_VM_OPTIONS")
            .or_else(|_| env::var("IDE_VM_OPTIONS"))
            .unwrap_or_default();
        let lower = idea_paths.to_ascii_lowercase();

        if lower.contains("webstorm") { return Ide::WebStorm; }
        if lower.contains("pycharm") { return Ide::PyCharm; }
        if lower.contains("goland") { return Ide::GoLand; }
        if lower.contains("rustrover") { return Ide::RustRover; }
        if lower.contains("rider") { return Ide::Rider; }
        if lower.contains("phpstorm") { return Ide::PhpStorm; }
        if lower.contains("clion") { return Ide::CLion; }
        if lower.contains("datagrip") { return Ide::DataGrip; }
        if lower.contains("androidstudio") || lower.contains("android-studio") { return Ide::AndroidStudio; }
        if lower.contains("fleet") { return Ide::Fleet; }
        return Ide::IntelliJIdea; // Default JetBrains product
    }

    // Neovim / Vim
    if env::var("NVIM").is_ok() || env::var("NVIM_LISTEN_ADDRESS").is_ok() {
        return Ide::Neovim;
    }
    if env::var("VIM").is_ok() || env::var("VIMRUNTIME").is_ok() {
        return Ide::Vim;
    }

    // Emacs
    if env::var("INSIDE_EMACS").is_ok() || env::var("EMACS").is_ok() {
        return Ide::Emacs;
    }

    // Xcode
    if env::var("__XCODE_BUILT_PRODUCTS_DIR_PATHS").is_ok() {
        return Ide::Xcode;
    }

    // Generic terminal detection
    if let Ok(term) = env::var("TERM_PROGRAM") {
        let lower = term.to_ascii_lowercase();
        if lower.contains("sublime") { return Ide::SublimeText; }
    }

    // If we're in a terminal at all
    if env::var("TERM").is_ok() || env::var("WT_SESSION").is_ok() {
        return Ide::Terminal;
    }

    Ide::Unknown
}

/// Get a summary of the detected IDE for prompt context.
pub fn ide_context_string() -> String {
    let ide = detect_ide();
    if ide == Ide::Unknown || ide == Ide::Terminal {
        String::new()
    } else {
        format!("User's IDE: {}", ide.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_something() {
        // In a test environment, should return Terminal or Unknown
        let ide = detect_ide();
        assert!(ide == Ide::Terminal || ide == Ide::Unknown || ide == Ide::VsCode);
    }

    #[test]
    fn display_names_are_nonempty() {
        let ides = [Ide::VsCode, Ide::Cursor, Ide::Zed, Ide::IntelliJIdea, Ide::Terminal];
        for ide in ides {
            assert!(!ide.display_name().is_empty());
        }
    }

    #[test]
    fn vscode_supports_lsp() {
        assert!(Ide::VsCode.supports_lsp());
        assert!(Ide::Cursor.supports_lsp());
        assert!(!Ide::Terminal.supports_lsp());
    }
}
