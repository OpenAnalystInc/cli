//! Output styles loader — .openanalyst/output-styles/*.md
//!
//! Custom output formatting definitions that control how the assistant
//! structures and formats its responses.
//!
//! ## Format
//!
//! ```markdown
//! ---
//! name: concise
//! description: Short, direct responses without unnecessary explanation
//! ---
//!
//! Respond concisely. Lead with the answer, not reasoning.
//! Use bullet points for multiple items.
//! Skip filler words and preamble.
//! ```

use std::path::{Path, PathBuf};

use crate::config_paths::{managed_config_home, user_config_home};

/// A loaded output style definition.
#[derive(Debug, Clone)]
pub struct OutputStyleDefinition {
    /// Style name (used as identifier).
    pub name: String,
    /// Description of when/how to use this style.
    pub description: String,
    /// The formatting instructions injected into the system prompt.
    pub instructions: String,
    /// Source file path.
    pub source: PathBuf,
    /// Scope (project or user level).
    pub scope: OutputStyleScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStyleScope {
    Managed,
    Project,
    User,
}

/// Load all output styles from managed, user, and project directories.
pub fn load_output_styles(project_dir: &Path) -> Vec<OutputStyleDefinition> {
    let mut styles = Vec::new();

    // Managed/global level
    if let Some(managed) = managed_config_home() {
        let managed_styles = managed.join("output-styles");
        if managed_styles.is_dir() {
            load_styles_from_dir(&managed_styles, OutputStyleScope::Managed, &mut styles);
        }
    }

    // User-level: ~/.openanalyst/output-styles/*.md
    if let Some(home) = user_config_home() {
        let user_styles = home.join("output-styles");
        if user_styles.is_dir() {
            load_styles_from_dir(&user_styles, OutputStyleScope::User, &mut styles);
        }
    }

    // Project-level: .openanalyst/output-styles/*.md
    let project_styles = project_dir.join(".openanalyst").join("output-styles");
    if project_styles.is_dir() {
        load_styles_from_dir(&project_styles, OutputStyleScope::Project, &mut styles);
    }

    // Deduplicate: managed always kept, project overrides user
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for style in styles.iter().filter(|s| s.scope == OutputStyleScope::Managed) {
        seen.insert(style.name.clone());
        deduped.push(style.clone());
    }
    for style in styles.iter().filter(|s| s.scope == OutputStyleScope::Project) {
        if seen.insert(style.name.clone()) {
            deduped.push(style.clone());
        }
    }
    for style in styles.iter().filter(|s| s.scope == OutputStyleScope::User) {
        if seen.insert(style.name.clone()) {
            deduped.push(style.clone());
        }
    }

    deduped
}

/// Find an output style by name.
pub fn find_output_style<'a>(styles: &'a [OutputStyleDefinition], name: &str) -> Option<&'a OutputStyleDefinition> {
    styles.iter().find(|s| s.name == name)
}

/// Format output styles as a help listing.
pub fn format_output_styles_list(styles: &[OutputStyleDefinition]) -> String {
    if styles.is_empty() {
        return "No output styles found.\n\n\
                Create output styles by adding .md files to:\n\
                  .openanalyst/output-styles/  (project-level)\n\
                  ~/.openanalyst/output-styles/ (user-level)\n\n\
                Format:\n\
                ---\n\
                name: concise\n\
                description: Short, direct responses\n\
                ---\n\
                Your formatting instructions here.".to_string();
    }

    let mut out = format!("Output Styles ({}):\n\n", styles.len());
    for style in styles {
        let scope = match style.scope {
            OutputStyleScope::Managed => "managed",
            OutputStyleScope::Project => "project",
            OutputStyleScope::User => "user",
        };
        let desc = if style.description.is_empty() { "(no description)" } else { &style.description };
        out.push_str(&format!("  {} — {} [{}]\n", style.name, desc, scope));
    }
    out
}

// ── Internal ────────────────────────────────────────────────────────────────

fn load_styles_from_dir(dir: &Path, scope: OutputStyleScope, styles: &mut Vec<OutputStyleDefinition>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md") {
            if let Some(style) = parse_style_file(&path, scope) {
                styles.push(style);
            }
        }
    }
}

fn parse_style_file(path: &Path, scope: OutputStyleScope) -> Option<OutputStyleDefinition> {
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    let file_name = path.file_stem()?.to_string_lossy().to_string();

    if !trimmed.starts_with("---") {
        return Some(OutputStyleDefinition {
            name: file_name,
            description: String::new(),
            instructions: content,
            source: path.to_path_buf(),
            scope,
        });
    }

    let after_first = &trimmed[3..];
    let end_marker = after_first.find("---")?;
    let frontmatter = after_first[..end_marker].trim();
    let body = after_first[end_marker + 3..].trim().to_string();

    let mut name = file_name;
    let mut description = String::new();

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "name" => name = value.to_string(),
                "description" | "desc" => description = value.to_string(),
                _ => {}
            }
        }
    }

    Some(OutputStyleDefinition {
        name,
        description,
        instructions: body,
        source: path.to_path_buf(),
        scope,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_style_with_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_style_concise.md");
        std::fs::write(&tmp, "---\nname: concise\ndescription: Short responses\n---\nBe brief. No filler.").unwrap();

        let style = parse_style_file(&tmp, OutputStyleScope::Project).unwrap();
        assert_eq!(style.name, "concise");
        assert_eq!(style.description, "Short responses");
        assert!(style.instructions.contains("Be brief"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parse_style_without_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_style_verbose.md");
        std::fs::write(&tmp, "Explain everything in detail with examples.").unwrap();

        let style = parse_style_file(&tmp, OutputStyleScope::User).unwrap();
        assert_eq!(style.name, "oa_style_verbose");
        assert!(style.instructions.contains("Explain everything"));

        let _ = std::fs::remove_file(&tmp);
    }
}
