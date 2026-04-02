//! Smart context injection — analyzes prompts and injects relevant workspace context.
//!
//! Before sending a prompt to the API, this module:
//! 1. Detects file paths, function names, and code patterns in the prompt
//! 2. Searches the workspace for matching files
//! 3. Reads the most relevant file snippets
//! 4. Prepends them to the system prompt as context
//!
//! This saves the model from needing tool calls for basic context gathering,
//! reducing round trips and token usage.

use std::path::Path;

/// Context gathered from the workspace based on prompt analysis.
#[derive(Debug, Default)]
pub struct InjectedContext {
    /// Files and their content to inject.
    pub files: Vec<ContextFile>,
    /// Summary line for the status bar.
    pub summary: String,
}

#[derive(Debug)]
pub struct ContextFile {
    pub path: String,
    pub content: String,
    pub reason: String,
}

/// Analyze a prompt and gather relevant context from the workspace.
pub fn gather_context(prompt: &str, cwd: &Path) -> InjectedContext {
    let mut ctx = InjectedContext::default();
    let mut file_count = 0;

    // 1. Extract explicit file paths from the prompt
    for path in extract_file_paths(prompt) {
        let full_path = cwd.join(&path);
        if full_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let truncated = truncate_file(&content, 200); // max 200 lines
                ctx.files.push(ContextFile {
                    path: path.clone(),
                    content: truncated,
                    reason: "mentioned in prompt".to_string(),
                });
                file_count += 1;
                if file_count >= 5 {
                    break; // Max 5 files auto-injected
                }
            }
        }
    }

    // 2. Look for OPENANALYST.md or similar context files
    for name in &["OPENANALYST.md", ".openanalyst.md", "CLAUDE.md"] {
        let ctx_file = cwd.join(name);
        if ctx_file.is_file() && file_count < 5 {
            if let Ok(content) = std::fs::read_to_string(&ctx_file) {
                let truncated = truncate_file(&content, 100);
                ctx.files.push(ContextFile {
                    path: name.to_string(),
                    content: truncated,
                    reason: "project context file".to_string(),
                });
                file_count += 1;
            }
            break; // Only one context file
        }
    }

    // 3. If prompt mentions patterns like "the login function" or "auth module",
    //    try to find relevant files via simple grep
    if file_count == 0 {
        for keyword in extract_code_keywords(prompt) {
            if let Some((path, snippet)) = grep_for_keyword(cwd, &keyword) {
                ctx.files.push(ContextFile {
                    path,
                    content: snippet,
                    reason: format!("contains '{keyword}'"),
                });
                file_count += 1;
                if file_count >= 3 {
                    break;
                }
            }
        }
    }

    if !ctx.files.is_empty() {
        ctx.summary = format!("{} file(s) auto-injected", ctx.files.len());
    }

    ctx
}

/// Format injected context as a system prompt section.
pub fn format_context_for_system_prompt(ctx: &InjectedContext) -> String {
    if ctx.files.is_empty() {
        return String::new();
    }

    let mut out = String::from("\n\n<workspace-context>\n");
    for file in &ctx.files {
        out.push_str(&format!(
            "## {} ({})\n```\n{}\n```\n\n",
            file.path, file.reason, file.content
        ));
    }
    out.push_str("</workspace-context>");
    out
}

// ── Internal helpers ──

/// Extract file paths from a prompt (looks for path-like strings).
fn extract_file_paths(prompt: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for word in prompt.split_whitespace() {
        let clean = word.trim_matches(|c: char| c == '`' || c == '"' || c == '\'' || c == ',');
        // Looks like a file path if it contains / or \ and has an extension
        if (clean.contains('/') || clean.contains('\\')) && clean.contains('.') {
            paths.push(clean.to_string());
        }
        // Or if it ends with common extensions
        else if clean.ends_with(".rs")
            || clean.ends_with(".ts")
            || clean.ends_with(".js")
            || clean.ends_with(".py")
            || clean.ends_with(".toml")
            || clean.ends_with(".json")
            || clean.ends_with(".yaml")
            || clean.ends_with(".yml")
            || clean.ends_with(".md")
        {
            paths.push(clean.to_string());
        }
    }
    paths
}

/// Extract code keywords from natural language (function names, module names).
fn extract_code_keywords(prompt: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    let lower = prompt.to_ascii_lowercase();

    // Look for patterns like "the X function", "X module", "X component"
    let patterns = [
        " function", " method", " module", " struct", " class",
        " component", " handler", " controller", " service",
    ];

    for pattern in &patterns {
        if let Some(pos) = lower.find(pattern) {
            // Look for the word before the pattern
            let before = &prompt[..pos];
            if let Some(word) = before.split_whitespace().last() {
                let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if clean.len() >= 3 {
                    keywords.push(clean.to_string());
                }
            }
        }
    }

    // Also look for backtick-quoted identifiers
    for cap in prompt.split('`') {
        let trimmed = cap.trim();
        if trimmed.len() >= 3
            && trimmed.len() <= 60
            && !trimmed.contains(' ')
            && trimmed.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '.')
        {
            keywords.push(trimmed.to_string());
        }
    }

    keywords.truncate(5); // Max 5 keywords to search
    keywords
}

/// Simple grep for a keyword in common source directories.
fn grep_for_keyword(cwd: &Path, keyword: &str) -> Option<(String, String)> {
    let output = std::process::Command::new("grep")
        .args(["-rn", "--include=*.rs", "--include=*.ts", "--include=*.py",
               "--include=*.js", "-l", keyword])
        .current_dir(cwd)
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_file = stdout.lines().next()?;

    // Read a snippet around the match
    let full_path = cwd.join(first_file);
    let content = std::fs::read_to_string(&full_path).ok()?;

    // Find the line with the keyword and take surrounding context
    let mut snippet_lines = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if line.contains(keyword) {
            let start = i.saturating_sub(5);
            let end = (i + 15).min(content.lines().count());
            for (j, l) in content.lines().enumerate() {
                if j >= start && j < end {
                    snippet_lines.push(format!("{:>4} | {l}", j + 1));
                }
            }
            break;
        }
    }

    if snippet_lines.is_empty() {
        return None;
    }

    Some((first_file.to_string(), snippet_lines.join("\n")))
}

/// Truncate file content to max N lines.
fn truncate_file(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= max_lines {
        content.to_string()
    } else {
        let truncated: Vec<&str> = lines[..max_lines].to_vec();
        format!(
            "{}\n... ({} more lines)",
            truncated.join("\n"),
            lines.len() - max_lines
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_file_paths() {
        let paths = extract_file_paths("fix the bug in src/auth.rs and update Cargo.toml");
        assert!(paths.contains(&"src/auth.rs".to_string()));
        assert!(paths.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn extracts_code_keywords() {
        let kw = extract_code_keywords("refactor the login function and the `AuthService` class");
        assert!(kw.contains(&"login".to_string()));
        assert!(kw.contains(&"AuthService".to_string()));
    }

    #[test]
    fn truncates_long_files() {
        let content = (0..500).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let truncated = truncate_file(&content, 10);
        assert!(truncated.contains("line 0"));
        assert!(truncated.contains("490 more lines"));
    }
}
