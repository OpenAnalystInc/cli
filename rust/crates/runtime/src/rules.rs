//! Rules loader — path-specific instruction files from .openanalyst/rules/.
//!
//! Rules are markdown files that provide context-specific instructions.
//! They can optionally include a `paths` frontmatter field with glob patterns
//! to scope when the rule is loaded into context.
//!
//! ## Format
//!
//! ```markdown
//! ---
//! paths:
//!   - "src/api/**/*.rs"
//!   - "src/**/*.test.rs"
//! ---
//!
//! # Rule content here
//! All API endpoints must include input validation...
//! ```
//!
//! Rules without a `paths` field are always loaded (global rules).
//! Rules with `paths` are only loaded when matching files are accessed.

use std::path::{Path, PathBuf};

use crate::config_paths::{managed_config_home, user_config_home};

/// A loaded rule definition.
#[derive(Debug, Clone)]
pub struct RuleDefinition {
    /// Rule name (derived from filename).
    pub name: String,
    /// Glob patterns that scope this rule. Empty = always active.
    pub paths: Vec<String>,
    /// The rule content (markdown instructions).
    pub content: String,
    /// Source file path.
    pub source: PathBuf,
    /// Configuration level (managed, user, or project).
    pub scope: RuleScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleScope {
    Managed,
    Project,
    User,
}

impl RuleDefinition {
    /// Returns true if this rule has no path restrictions (always active).
    #[must_use]
    pub fn is_global(&self) -> bool {
        self.paths.is_empty()
    }

    /// Check if a file path matches any of this rule's glob patterns.
    #[must_use]
    pub fn matches_path(&self, file_path: &str) -> bool {
        if self.paths.is_empty() {
            return true;
        }
        for pattern in &self.paths {
            if glob_matches(pattern, file_path) {
                return true;
            }
        }
        false
    }
}

/// Load all rules from managed, user, and project rule directories.
/// Priority: project > user > managed (for overrides by name).
/// Managed rules cannot be overridden (they are always included).
pub fn load_rules(project_dir: &Path) -> Vec<RuleDefinition> {
    let mut rules = Vec::new();

    // Managed/global: /etc/openanalyst-cli/rules/ or equivalent
    if let Some(managed) = managed_config_home() {
        let managed_rules = managed.join("rules");
        if managed_rules.is_dir() {
            load_rules_from_dir(&managed_rules, RuleScope::Managed, &mut rules);
        }
    }

    // User-level: ~/.openanalyst/rules/*.md
    if let Some(home) = user_config_home() {
        let user_rules = home.join("rules");
        if user_rules.is_dir() {
            load_rules_from_dir(&user_rules, RuleScope::User, &mut rules);
        }
    }

    // Project-level: .openanalyst/rules/*.md
    let project_rules = project_dir.join(".openanalyst").join("rules");
    if project_rules.is_dir() {
        load_rules_from_dir(&project_rules, RuleScope::Project, &mut rules);
    }

    // Deduplicate: managed rules always kept, project overrides user
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    // Managed rules first (cannot be overridden)
    for rule in rules.iter().filter(|r| r.scope == RuleScope::Managed) {
        seen.insert(rule.name.clone());
        deduped.push(rule.clone());
    }
    // Project rules next (override user)
    for rule in rules.iter().filter(|r| r.scope == RuleScope::Project) {
        if seen.insert(rule.name.clone()) {
            deduped.push(rule.clone());
        }
    }
    // User rules last
    for rule in rules.iter().filter(|r| r.scope == RuleScope::User) {
        if seen.insert(rule.name.clone()) {
            deduped.push(rule.clone());
        }
    }

    deduped
}

/// Get all global rules (always active, no path restrictions).
pub fn global_rules(rules: &[RuleDefinition]) -> Vec<&RuleDefinition> {
    rules.iter().filter(|r| r.is_global()).collect()
}

/// Get rules matching a specific file path.
pub fn rules_for_path<'a>(rules: &'a [RuleDefinition], file_path: &str) -> Vec<&'a RuleDefinition> {
    rules.iter().filter(|r| r.matches_path(file_path)).collect()
}

/// Format rules as a help listing.
pub fn format_rules_list(rules: &[RuleDefinition]) -> String {
    if rules.is_empty() {
        return "No rules found.\n\n\
                Create rules by adding .md files to:\n\
                  .openanalyst/rules/  (project-level)\n\
                  ~/.openanalyst/rules/ (user-level)\n\n\
                Format:\n\
                ---\n\
                paths:\n\
                  - \"src/**/*.rs\"\n\
                ---\n\
                Your rule instructions here.\n\
                Rules without paths are always active.".to_string();
    }

    let global_count = rules.iter().filter(|r| r.is_global()).count();
    let scoped_count = rules.len() - global_count;
    let mut out = format!("Rules ({} total: {} global, {} path-scoped):\n\n", rules.len(), global_count, scoped_count);

    for rule in rules {
        let scope = match rule.scope {
            RuleScope::Managed => "managed",
            RuleScope::Project => "project",
            RuleScope::User => "user",
        };
        if rule.is_global() {
            out.push_str(&format!("  {} — global [{}]\n", rule.name, scope));
        } else {
            let patterns = rule.paths.join(", ");
            out.push_str(&format!("  {} — {} [{}]\n", rule.name, patterns, scope));
        }
    }
    out
}

// ── Internal ────────────────────────────────────────────────────────────────

fn load_rules_from_dir(dir: &Path, scope: RuleScope, rules: &mut Vec<RuleDefinition>) {
    load_rules_recursive(dir, dir, scope, rules, 0);
}

fn load_rules_recursive(base: &Path, dir: &Path, scope: RuleScope, rules: &mut Vec<RuleDefinition>, depth: u32) {
    if depth > 5 {
        return; // Prevent excessive nesting
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_rules_recursive(base, &path, scope, rules, depth + 1);
        } else if path.extension().map_or(false, |ext| ext == "md") {
            if let Some(rule) = parse_rule_file(&path, base, scope) {
                rules.push(rule);
            }
        }
    }
}

fn parse_rule_file(path: &Path, base_dir: &Path, scope: RuleScope) -> Option<RuleDefinition> {
    let content = std::fs::read_to_string(path).ok()?;

    // Derive name from relative path (e.g., "frontend/react" from "rules/frontend/react.md")
    let relative = path.strip_prefix(base_dir).unwrap_or(path);
    let name = relative
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/");

    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        // No frontmatter — global rule
        return Some(RuleDefinition {
            name,
            paths: Vec::new(),
            content: content.to_string(),
            source: path.to_path_buf(),
            scope,
        });
    }

    let after_first = &trimmed[3..];
    let end_marker = after_first.find("---")?;
    let frontmatter = &after_first[..end_marker].trim();
    let body = after_first[end_marker + 3..].trim().to_string();

    // Parse paths from frontmatter
    let mut paths = Vec::new();
    let mut in_paths_list = false;

    for line in frontmatter.lines() {
        let line = line.trim();
        if line.starts_with("paths:") {
            in_paths_list = true;
            // Check for inline value: paths: ["a", "b"]
            let value = line.strip_prefix("paths:").unwrap().trim();
            if value.starts_with('[') {
                // JSON-style inline array
                paths.extend(
                    value.trim_start_matches('[').trim_end_matches(']')
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|s| !s.is_empty())
                );
                in_paths_list = false;
            }
        } else if in_paths_list && line.starts_with("- ") {
            let pattern = line.strip_prefix("- ").unwrap().trim()
                .trim_matches('"').trim_matches('\'').to_string();
            if !pattern.is_empty() {
                paths.push(pattern);
            }
        } else if in_paths_list && !line.starts_with('-') && !line.is_empty() {
            in_paths_list = false;
        }
    }

    Some(RuleDefinition {
        name,
        paths,
        content: body,
        source: path.to_path_buf(),
        scope,
    })
}

/// Simple glob matching supporting *, **, and ? patterns.
fn glob_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.replace('\\', "/");
    let path = path.replace('\\', "/");
    glob_match_recursive(pattern.as_bytes(), path.as_bytes())
}

fn glob_match_recursive(pattern: &[u8], path: &[u8]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }

    // Handle ** (matches any number of path segments)
    if pattern.starts_with(b"**/") {
        let rest_pattern = &pattern[3..];
        // Try matching at current position and every subsequent /
        if glob_match_recursive(rest_pattern, path) {
            return true;
        }
        for (i, &byte) in path.iter().enumerate() {
            if byte == b'/' {
                if glob_match_recursive(rest_pattern, &path[i + 1..]) {
                    return true;
                }
            }
        }
        return false;
    }

    if pattern == b"**" {
        return true;
    }

    if path.is_empty() {
        // Pattern must be all wildcards to match empty path
        return pattern.iter().all(|&b| b == b'*');
    }

    match pattern[0] {
        b'*' => {
            // * matches any character except /
            if pattern.len() > 1 && pattern[1] == b'*' {
                // ** handled above, but catch edge cases
                return glob_match_recursive(pattern, path);
            }
            // Try matching 0 or more non-/ characters
            glob_match_recursive(&pattern[1..], path)
                || (path[0] != b'/' && glob_match_recursive(pattern, &path[1..]))
        }
        b'?' => {
            // ? matches any single character except /
            path[0] != b'/' && glob_match_recursive(&pattern[1..], &path[1..])
        }
        _ => {
            // Literal character match
            pattern[0] == path[0] && glob_match_recursive(&pattern[1..], &path[1..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_basic_patterns() {
        assert!(glob_matches("*.rs", "main.rs"));
        assert!(!glob_matches("*.rs", "src/main.rs"));
        assert!(glob_matches("**/*.rs", "src/main.rs"));
        assert!(glob_matches("**/*.rs", "src/deep/nested/main.rs"));
        assert!(glob_matches("src/**/*.rs", "src/lib.rs"));
        assert!(glob_matches("src/**/*.rs", "src/deep/lib.rs"));
        assert!(!glob_matches("src/**/*.rs", "tests/lib.rs"));
        assert!(glob_matches("src/*.{ts,tsx}", "src/app.ts") || glob_matches("src/*.ts", "src/app.ts"));
    }

    #[test]
    fn parse_rule_without_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_rule_test_global.md");
        std::fs::write(&tmp, "# Always active rule\nDo this always.").unwrap();

        let rule = parse_rule_file(&tmp, &std::env::temp_dir(), RuleScope::Project).unwrap();
        assert!(rule.is_global());
        assert!(rule.content.contains("Always active"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parse_rule_with_paths_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_rule_test_scoped.md");
        std::fs::write(&tmp, "---\npaths:\n  - \"src/api/**/*.rs\"\n  - \"src/**/*.test.rs\"\n---\n\n# API rules\nValidate inputs.").unwrap();

        let rule = parse_rule_file(&tmp, &std::env::temp_dir(), RuleScope::User).unwrap();
        assert!(!rule.is_global());
        assert_eq!(rule.paths.len(), 2);
        assert_eq!(rule.paths[0], "src/api/**/*.rs");
        assert!(rule.content.contains("Validate inputs"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn rule_path_matching() {
        let rule = RuleDefinition {
            name: "api".to_string(),
            paths: vec!["src/api/**/*.rs".to_string()],
            content: String::new(),
            source: PathBuf::new(),
            scope: RuleScope::Project,
        };
        assert!(rule.matches_path("src/api/handlers/auth.rs"));
        assert!(rule.matches_path("src/api/mod.rs"));
        assert!(!rule.matches_path("src/lib.rs"));
        assert!(!rule.matches_path("tests/api_test.rs"));
    }
}
