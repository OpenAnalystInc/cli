//! Skills & commands loader — loads skill definitions and custom slash commands.
//!
//! ## Skill format (`.openanalyst/skills/<name>/SKILL.md`)
//!
//! Multi-file skills live in subdirectories under `skills/`:
//! ```
//! .openanalyst/skills/deploy/
//! ├── SKILL.md          # Required: main skill definition
//! ├── reference.md      # Optional: supporting docs (available via ${OPENANALYST_SKILL_DIR})
//! └── scripts/
//!     └── validate.sh   # Optional: helper scripts
//! ```
//!
//! SKILL.md frontmatter:
//! ```yaml
//! ---
//! name: deploy
//! description: Deploy the application to staging or production
//! argument-hint: "[environment] [--dry-run]"
//! disable-model-invocation: false
//! user-invocable: true
//! allowed-tools: "Read Grep Bash(npm *)"
//! model: "openanalyst-beta"
//! effort: "medium"
//! context: "fork"
//! agent: "Explore"
//! paths:
//!   - "src/**/*.ts"
//! shell: "bash"
//! ---
//!
//! Skill instructions here...
//! $ARGUMENTS for user input
//! $0, $1 for positional args
//! ```
//!
//! ## Command format (`.openanalyst/commands/<name>.md`)
//!
//! Simple single-file slash commands:
//! ```yaml
//! ---
//! name: review-pr
//! description: Review a pull request
//! ---
//! You are a code reviewer... $ARGUMENTS
//! ```

use std::path::{Path, PathBuf};

use crate::config_paths::{managed_config_home, user_config_home};

/// A loaded skill definition with full Claude Code-compatible frontmatter.
#[derive(Debug, Clone)]
pub struct SkillDefinition {
    /// Command name (used as /name). Lowercase, hyphens/numbers, max 64 chars.
    pub name: String,
    /// Short description for help text and auto-invocation matching.
    pub description: String,
    /// Hint shown in help for expected arguments (e.g., "[filename] [format]").
    pub argument_hint: Option<String>,
    /// The system prompt template. $ARGUMENTS is replaced with user input.
    pub prompt_template: String,
    /// Source file path (SKILL.md or command .md file).
    pub source: PathBuf,
    /// Directory containing the skill (for multi-file skills with SKILL.md).
    pub skill_dir: Option<PathBuf>,
    /// Whether this is a project-level or user-level skill.
    pub scope: SkillScope,
    /// Skill kind — multi-file SKILL.md or simple command .md.
    pub kind: SkillKind,
    /// If true, only the user can invoke this skill (not the model).
    pub disable_model_invocation: bool,
    /// If true, the user can invoke this skill as a slash command.
    pub user_invocable: bool,
    /// Space-separated tool permissions (e.g., "Read Grep Bash(npm *)").
    pub allowed_tools: Option<String>,
    /// Model override for this skill.
    pub model: Option<String>,
    /// Effort level: "low", "medium", "high", "max".
    pub effort: Option<String>,
    /// Execution context: "fork" to run in isolated subagent.
    pub context: Option<String>,
    /// Which subagent to use when context is "fork".
    pub agent: Option<String>,
    /// Path glob patterns — skill is relevant when matching files are accessed.
    pub paths: Vec<String>,
    /// Shell to use for shell injections: "bash", "powershell".
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillScope {
    Managed,
    Project,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillKind {
    /// Multi-file skill from skills/<name>/SKILL.md
    Skill,
    /// Simple single-file command from commands/<name>.md
    Command,
}

/// Load all skills and commands from managed, user, and project directories.
/// Priority: managed (always kept) > project (overrides user) > user.
pub fn load_skills(project_dir: &Path) -> Vec<SkillDefinition> {
    let mut skills = Vec::new();

    // Managed/global level
    if let Some(managed) = managed_config_home() {
        load_commands_from_dir(&managed.join("commands"), SkillScope::Managed, &mut skills);
        load_skills_from_dir(&managed.join("skills"), SkillScope::Managed, &mut skills);
    }

    // User-level
    if let Some(home) = user_config_home() {
        load_commands_from_dir(&home.join("commands"), SkillScope::User, &mut skills);
        load_skills_from_dir(&home.join("skills"), SkillScope::User, &mut skills);
    }

    // Project-level (overrides user by name)
    let oa_dir = project_dir.join(".openanalyst");
    load_commands_from_dir(&oa_dir.join("commands"), SkillScope::Project, &mut skills);
    load_skills_from_dir(&oa_dir.join("skills"), SkillScope::Project, &mut skills);

    // Deduplicate: managed always kept, project overrides user
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for skill in skills.iter().filter(|s| s.scope == SkillScope::Managed) {
        seen.insert(skill.name.clone());
        deduped.push(skill.clone());
    }
    for skill in skills.iter().filter(|s| s.scope == SkillScope::Project) {
        if seen.insert(skill.name.clone()) {
            deduped.push(skill.clone());
        }
    }
    for skill in skills.iter().filter(|s| s.scope == SkillScope::User) {
        if seen.insert(skill.name.clone()) {
            deduped.push(skill.clone());
        }
    }

    deduped
}

/// Execute a skill by replacing template variables with user input.
pub fn execute_skill(skill: &SkillDefinition, arguments: &str) -> String {
    let mut result = skill.prompt_template.replace("$ARGUMENTS", arguments);

    // Replace positional arguments ($0, $1, $2, ...)
    let parts: Vec<&str> = arguments.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        result = result.replace(&format!("${}", i), part);
    }

    // Replace ${OPENANALYST_SKILL_DIR} with the skill directory path
    if let Some(dir) = &skill.skill_dir {
        result = result.replace("${OPENANALYST_SKILL_DIR}", &dir.display().to_string());
    }

    // Replace ${OPENANALYST_SESSION_ID} placeholder (caller should replace with actual ID)
    // Left as-is here; the orchestrator handles it.

    result
}

/// Format skills as a help listing.
pub fn format_skills_list(skills: &[SkillDefinition]) -> String {
    if skills.is_empty() {
        return "No custom skills or commands found.\n\n\
                Create skills by adding SKILL.md files to:\n\
                  .openanalyst/skills/<name>/SKILL.md   (project-level)\n\
                  ~/.openanalyst/skills/<name>/SKILL.md  (user-level)\n\n\
                Or create simple slash commands:\n\
                  .openanalyst/commands/<name>.md   (project-level)\n\
                  ~/.openanalyst/commands/<name>.md  (user-level)\n\n\
                SKILL.md format:\n\
                ---\n\
                name: my-skill\n\
                description: What this skill does\n\
                argument-hint: \"[arg1] [arg2]\"\n\
                user-invocable: true\n\
                allowed-tools: \"Read Grep Bash\"\n\
                ---\n\
                Your prompt template here.\n\
                $ARGUMENTS will be replaced with user input.".to_string();
    }

    let skill_count = skills.iter().filter(|s| s.kind == SkillKind::Skill).count();
    let cmd_count = skills.iter().filter(|s| s.kind == SkillKind::Command).count();
    let mut out = format!("Skills & Commands ({} skills, {} commands):\n\n", skill_count, cmd_count);

    if skill_count > 0 {
        out.push_str("  Skills:\n");
        for skill in skills.iter().filter(|s| s.kind == SkillKind::Skill) {
            let scope = scope_label(skill.scope);
            let desc = if skill.description.is_empty() { "(no description)" } else { &skill.description };
            let hint = skill.argument_hint.as_deref().unwrap_or("");
            let invocable = if skill.user_invocable { "" } else { " [model-only]" };
            if hint.is_empty() {
                out.push_str(&format!("    /{} — {}{} [{}]\n", skill.name, desc, invocable, scope));
            } else {
                out.push_str(&format!("    /{} {} — {}{} [{}]\n", skill.name, hint, desc, invocable, scope));
            }
        }
        out.push('\n');
    }

    if cmd_count > 0 {
        out.push_str("  Commands:\n");
        for skill in skills.iter().filter(|s| s.kind == SkillKind::Command) {
            let scope = scope_label(skill.scope);
            let desc = if skill.description.is_empty() { "(no description)" } else { &skill.description };
            out.push_str(&format!("    /{} — {} [{}]\n", skill.name, desc, scope));
        }
        out.push('\n');
    }

    out.push_str("Use /<name> [arguments] to invoke.");
    out
}

fn scope_label(scope: SkillScope) -> &'static str {
    match scope {
        SkillScope::Managed => "managed",
        SkillScope::Project => "project",
        SkillScope::User => "user",
    }
}

// ── Commands loading (simple .md files) ─────────────────────────────────────

fn load_commands_from_dir(dir: &Path, scope: SkillScope, skills: &mut Vec<SkillDefinition>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md") {
            if let Some(skill) = parse_command_file(&path, scope) {
                skills.push(skill);
            }
        }
    }
}

fn parse_command_file(path: &Path, scope: SkillScope) -> Option<SkillDefinition> {
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();

    if !trimmed.starts_with("---") {
        let name = path.file_stem()?.to_string_lossy().to_string();
        return Some(SkillDefinition {
            name,
            description: String::new(),
            argument_hint: None,
            prompt_template: content,
            source: path.to_path_buf(),
            skill_dir: None,
            scope,
            kind: SkillKind::Command,
            disable_model_invocation: false,
            user_invocable: true,
            allowed_tools: None,
            model: None,
            effort: None,
            context: None,
            agent: None,
            paths: Vec::new(),
            shell: None,
        });
    }

    let after_first = &trimmed[3..];
    let end_marker = after_first.find("---")?;
    let frontmatter = after_first[..end_marker].trim();
    let body = after_first[end_marker + 3..].trim().to_string();

    let mut name = path.file_stem()?.to_string_lossy().to_string();
    let mut description = String::new();

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "name" => name = value.to_string(),
                "description" | "desc" | "summary" => description = value.to_string(),
                _ => {}
            }
        }
    }

    Some(SkillDefinition {
        name,
        description,
        argument_hint: None,
        prompt_template: body,
        source: path.to_path_buf(),
        skill_dir: None,
        scope,
        kind: SkillKind::Command,
        disable_model_invocation: false,
        user_invocable: true,
        allowed_tools: None,
        model: None,
        effort: None,
        context: None,
        agent: None,
        paths: Vec::new(),
        shell: None,
    })
}

// ── Skills loading (SKILL.md in subdirectories) ─────────────────────────────

fn load_skills_from_dir(dir: &Path, scope: SkillScope, skills: &mut Vec<SkillDefinition>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Multi-file skill: skills/<name>/SKILL.md
            let skill_md = path.join("SKILL.md");
            if skill_md.is_file() {
                if let Some(skill) = parse_skill_md(&skill_md, &path, scope) {
                    skills.push(skill);
                }
            }
        } else if path.extension().map_or(false, |ext| ext == "md") {
            // Single-file skill: skills/<name>.md (fallback, simpler format)
            if let Some(skill) = parse_single_skill_file(&path, scope) {
                skills.push(skill);
            }
        }
    }
}

fn parse_skill_md(skill_md: &Path, skill_dir: &Path, scope: SkillScope) -> Option<SkillDefinition> {
    let content = std::fs::read_to_string(skill_md).ok()?;
    let trimmed = content.trim();

    // Default name from directory name
    let dir_name = skill_dir.file_name()?.to_string_lossy().to_string();

    if !trimmed.starts_with("---") {
        return Some(SkillDefinition {
            name: dir_name,
            description: String::new(),
            argument_hint: None,
            prompt_template: content,
            source: skill_md.to_path_buf(),
            skill_dir: Some(skill_dir.to_path_buf()),
            scope,
            kind: SkillKind::Skill,
            disable_model_invocation: false,
            user_invocable: true,
            allowed_tools: None,
            model: None,
            effort: None,
            context: None,
            agent: None,
            paths: Vec::new(),
            shell: None,
        });
    }

    let after_first = &trimmed[3..];
    let end_marker = after_first.find("---")?;
    let frontmatter = after_first[..end_marker].trim();
    let body = after_first[end_marker + 3..].trim().to_string();

    let mut def = SkillDefinition {
        name: dir_name,
        description: String::new(),
        argument_hint: None,
        prompt_template: body,
        source: skill_md.to_path_buf(),
        skill_dir: Some(skill_dir.to_path_buf()),
        scope,
        kind: SkillKind::Skill,
        disable_model_invocation: false,
        user_invocable: true,
        allowed_tools: None,
        model: None,
        effort: None,
        context: None,
        agent: None,
        paths: Vec::new(),
        shell: None,
    };

    let mut in_paths_list = false;

    for line in frontmatter.lines() {
        let line = line.trim();

        // YAML list continuation for paths
        if in_paths_list {
            if line.starts_with("- ") {
                let val = line.strip_prefix("- ").unwrap().trim()
                    .trim_matches('"').trim_matches('\'').to_string();
                if !val.is_empty() {
                    def.paths.push(val);
                }
                continue;
            }
            if !line.starts_with('-') && !line.is_empty() {
                in_paths_list = false;
            }
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "name" => def.name = value.to_string(),
                "description" | "desc" | "summary" => def.description = value.to_string(),
                "argument-hint" | "argumentHint" => def.argument_hint = Some(value.to_string()),
                "disable-model-invocation" | "disableModelInvocation" => {
                    def.disable_model_invocation = value == "true";
                }
                "user-invocable" | "userInvocable" => {
                    def.user_invocable = value != "false";
                }
                "allowed-tools" | "allowedTools" => def.allowed_tools = some_nonempty(value),
                "model" => def.model = some_nonempty(value),
                "effort" => def.effort = some_nonempty(value),
                "context" => def.context = some_nonempty(value),
                "agent" => def.agent = some_nonempty(value),
                "shell" => def.shell = some_nonempty(value),
                "paths" => {
                    if value.is_empty() {
                        in_paths_list = true;
                    } else if value.starts_with('[') {
                        // Inline JSON array
                        def.paths.extend(
                            value.trim_start_matches('[').trim_end_matches(']')
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                .filter(|s| !s.is_empty())
                        );
                    } else {
                        def.paths.push(value.to_string());
                    }
                }
                _ => {} // Ignore unknown frontmatter keys
            }
        }
    }

    Some(def)
}

fn parse_single_skill_file(path: &Path, scope: SkillScope) -> Option<SkillDefinition> {
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();

    let name = path.file_stem()?.to_string_lossy().to_string();

    if !trimmed.starts_with("---") {
        return Some(SkillDefinition {
            name,
            description: String::new(),
            argument_hint: None,
            prompt_template: content,
            source: path.to_path_buf(),
            skill_dir: None,
            scope,
            kind: SkillKind::Skill,
            disable_model_invocation: false,
            user_invocable: true,
            allowed_tools: None,
            model: None,
            effort: None,
            context: None,
            agent: None,
            paths: Vec::new(),
            shell: None,
        });
    }

    // Reuse the SKILL.md parser by treating as if it's in a virtual directory
    let after_first = &trimmed[3..];
    let end_marker = after_first.find("---")?;
    let frontmatter = after_first[..end_marker].trim();
    let body = after_first[end_marker + 3..].trim().to_string();

    let mut def = SkillDefinition {
        name,
        description: String::new(),
        argument_hint: None,
        prompt_template: body,
        source: path.to_path_buf(),
        skill_dir: None,
        scope,
        kind: SkillKind::Skill,
        disable_model_invocation: false,
        user_invocable: true,
        allowed_tools: None,
        model: None,
        effort: None,
        context: None,
        agent: None,
        paths: Vec::new(),
        shell: None,
    };

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key {
                "name" => def.name = value.to_string(),
                "description" | "desc" | "summary" => def.description = value.to_string(),
                "argument-hint" | "argumentHint" => def.argument_hint = Some(value.to_string()),
                "allowed-tools" | "allowedTools" => def.allowed_tools = some_nonempty(value),
                "model" => def.model = some_nonempty(value),
                "effort" => def.effort = some_nonempty(value),
                "disable-model-invocation" => def.disable_model_invocation = value == "true",
                "user-invocable" => def.user_invocable = value != "false",
                _ => {}
            }
        }
    }

    Some(def)
}

fn some_nonempty(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_with_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_cmd_test.md");
        std::fs::write(&tmp, "---\nname: test-cmd\ndescription: A test command\n---\nHello $ARGUMENTS").unwrap();

        let skill = parse_command_file(&tmp, SkillScope::Project).unwrap();
        assert_eq!(skill.name, "test-cmd");
        assert_eq!(skill.description, "A test command");
        assert_eq!(skill.kind, SkillKind::Command);
        assert!(skill.prompt_template.contains("$ARGUMENTS"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parse_command_without_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_bare_cmd.md");
        std::fs::write(&tmp, "Just a prompt template").unwrap();

        let skill = parse_command_file(&tmp, SkillScope::User).unwrap();
        assert_eq!(skill.name, "oa_bare_cmd");
        assert_eq!(skill.kind, SkillKind::Command);
        assert!(skill.prompt_template.contains("Just a prompt"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parse_skill_md_full_frontmatter() {
        let dir = std::env::temp_dir().join("oa_skill_deploy_test");
        let _ = std::fs::create_dir_all(&dir);
        let skill_md = dir.join("SKILL.md");
        std::fs::write(&skill_md, "\
---
name: deploy
description: Deploy the application
argument-hint: \"[env] [--dry-run]\"
user-invocable: true
disable-model-invocation: false
allowed-tools: \"Read Grep Bash(npm *)\"
model: openanalyst-beta
effort: medium
context: fork
agent: Explore
shell: bash
paths:
  - \"src/**/*.ts\"
  - \"deploy/**\"
---

Deploy $ARGUMENTS to the target environment.
Use ${OPENANALYST_SKILL_DIR} for helper scripts.
").unwrap();

        let skill = parse_skill_md(&skill_md, &dir, SkillScope::Project).unwrap();
        assert_eq!(skill.name, "deploy");
        assert_eq!(skill.description, "Deploy the application");
        assert_eq!(skill.argument_hint.as_deref(), Some("[env] [--dry-run]"));
        assert_eq!(skill.kind, SkillKind::Skill);
        assert!(skill.user_invocable);
        assert!(!skill.disable_model_invocation);
        assert_eq!(skill.allowed_tools.as_deref(), Some("Read Grep Bash(npm *)"));
        assert_eq!(skill.model.as_deref(), Some("openanalyst-beta"));
        assert_eq!(skill.effort.as_deref(), Some("medium"));
        assert_eq!(skill.context.as_deref(), Some("fork"));
        assert_eq!(skill.agent.as_deref(), Some("Explore"));
        assert_eq!(skill.shell.as_deref(), Some("bash"));
        assert_eq!(skill.paths, vec!["src/**/*.ts", "deploy/**"]);
        assert!(skill.prompt_template.contains("Deploy $ARGUMENTS"));
        assert!(skill.skill_dir.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn execute_replaces_arguments_and_positional() {
        let skill = SkillDefinition {
            name: "test".to_string(),
            description: String::new(),
            argument_hint: None,
            prompt_template: "Deploy $0 with flag $1. Full: $ARGUMENTS".to_string(),
            source: PathBuf::new(),
            skill_dir: Some(PathBuf::from("/skills/deploy")),
            scope: SkillScope::Project,
            kind: SkillKind::Skill,
            disable_model_invocation: false,
            user_invocable: true,
            allowed_tools: None,
            model: None,
            effort: None,
            context: None,
            agent: None,
            paths: Vec::new(),
            shell: None,
        };
        let result = execute_skill(&skill, "staging --dry-run");
        assert_eq!(result, "Deploy staging with flag --dry-run. Full: staging --dry-run");
    }
}
