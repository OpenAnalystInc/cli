//! Skills loader — loads markdown command files from .openanalyst/commands/.
//!
//! Compatible with Claude Code's custom commands format:
//! ```markdown
//! ---
//! name: review-pr
//! description: Review a pull request
//! ---
//! You are a code reviewer...
//! $ARGUMENTS
//! ```

use std::path::{Path, PathBuf};

/// A loaded skill definition.
#[derive(Debug, Clone)]
pub struct SkillDefinition {
    /// Command name (used as /name).
    pub name: String,
    /// Short description for help text.
    pub description: String,
    /// The system prompt template. $ARGUMENTS is replaced with user input.
    pub prompt_template: String,
    /// Source file path.
    pub source: PathBuf,
    /// Whether this is a project-level or user-level skill.
    pub scope: SkillScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillScope {
    Project,
    User,
}

/// Load all skills from both project and user command directories.
pub fn load_skills(project_dir: &Path) -> Vec<SkillDefinition> {
    let mut skills = Vec::new();

    // User-level: ~/.openanalyst/commands/*.md
    if let Some(home) = openanalyst_home() {
        let user_commands = home.join("commands");
        if user_commands.is_dir() {
            load_skills_from_dir(&user_commands, SkillScope::User, &mut skills);
        }
    }

    // Project-level: .openanalyst/commands/*.md (overrides user by name)
    let project_commands = project_dir.join(".openanalyst").join("commands");
    if project_commands.is_dir() {
        load_skills_from_dir(&project_commands, SkillScope::Project, &mut skills);
    }

    // Deduplicate: project-level overrides user-level by name
    let mut seen = std::collections::HashSet::new();
    skills.retain(|s| {
        if seen.contains(&s.name) {
            // Keep the first occurrence (project-level was added last, so reverse)
            false
        } else {
            seen.insert(s.name.clone());
            true
        }
    });

    skills
}

/// Load skills from a single directory.
fn load_skills_from_dir(dir: &Path, scope: SkillScope, skills: &mut Vec<SkillDefinition>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md") {
            if let Some(skill) = parse_skill_file(&path, scope) {
                // Project skills go to front (higher priority)
                if scope == SkillScope::Project {
                    skills.insert(0, skill);
                } else {
                    skills.push(skill);
                }
            }
        }
    }
}

/// Parse a single skill markdown file.
fn parse_skill_file(path: &Path, scope: SkillScope) -> Option<SkillDefinition> {
    let content = std::fs::read_to_string(path).ok()?;

    // Parse YAML frontmatter (between --- markers)
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        // No frontmatter — use filename as name, entire content as prompt
        let name = path.file_stem()?.to_string_lossy().to_string();
        return Some(SkillDefinition {
            name,
            description: String::new(),
            prompt_template: content,
            source: path.to_path_buf(),
            scope,
        });
    }

    let after_first = &trimmed[3..];
    let end_marker = after_first.find("---")?;
    let frontmatter = &after_first[..end_marker].trim();
    let body = after_first[end_marker + 3..].trim().to_string();

    // Parse simple YAML key: value pairs
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
                _ => {} // Ignore unknown frontmatter keys
            }
        }
    }

    Some(SkillDefinition {
        name,
        description,
        prompt_template: body,
        source: path.to_path_buf(),
        scope,
    })
}

/// Execute a skill by replacing $ARGUMENTS with the user's input.
pub fn execute_skill(skill: &SkillDefinition, arguments: &str) -> String {
    skill.prompt_template.replace("$ARGUMENTS", arguments)
}

/// Format skills as a help listing.
pub fn format_skills_list(skills: &[SkillDefinition]) -> String {
    if skills.is_empty() {
        return "No custom skills found.\n\n\
                Create skills by adding .md files to:\n\
                  .openanalyst/commands/  (project-level)\n\
                  ~/.openanalyst/commands/ (user-level)\n\n\
                Format:\n\
                ---\n\
                name: my-skill\n\
                description: What this skill does\n\
                ---\n\
                Your prompt template here.\n\
                $ARGUMENTS will be replaced with user input.".to_string();
    }

    let mut out = format!("Custom Skills ({}):\n\n", skills.len());
    for skill in skills {
        let scope = match skill.scope {
            SkillScope::Project => "project",
            SkillScope::User => "user",
        };
        let desc = if skill.description.is_empty() { "(no description)" } else { &skill.description };
        out.push_str(&format!("  /{} — {} [{}]\n", skill.name, desc, scope));
    }
    out.push_str("\nUse /<skill-name> [arguments] to run a skill.");
    out
}

fn openanalyst_home() -> Option<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skill_with_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_skill_test.md");
        std::fs::write(&tmp, "---\nname: test-skill\ndescription: A test\n---\nHello $ARGUMENTS").unwrap();

        let skill = parse_skill_file(&tmp, SkillScope::Project).unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "A test");
        assert!(skill.prompt_template.contains("$ARGUMENTS"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parse_skill_without_frontmatter() {
        let tmp = std::env::temp_dir().join("oa_bare_skill.md");
        std::fs::write(&tmp, "Just a prompt template").unwrap();

        let skill = parse_skill_file(&tmp, SkillScope::User).unwrap();
        assert_eq!(skill.name, "oa_bare_skill");
        assert!(skill.prompt_template.contains("Just a prompt"));

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn execute_replaces_arguments() {
        let skill = SkillDefinition {
            name: "test".to_string(),
            description: String::new(),
            prompt_template: "Review $ARGUMENTS carefully.".to_string(),
            source: PathBuf::new(),
            scope: SkillScope::Project,
        };
        let result = execute_skill(&skill, "the PR");
        assert_eq!(result, "Review the PR carefully.");
    }
}
