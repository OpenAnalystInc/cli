//! Autonomous agent — Karpathy-style loop: think→act→observe→verify→repeat.
//!
//! Inspired by Andrej Karpathy's agent philosophy:
//! - Simple loop, good model, basic tools
//! - Let the model do its own planning (no elaborate external planners)
//! - Verifiable success criteria (does it compile? do tests pass?)
//! - Retry on failure, ask human only when genuinely stuck
//!
//! The autonomous agent runs without user interaction until:
//! 1. Success criteria pass (if provided)
//! 2. Max turns reached
//! 3. The model signals completion
//! 4. User cancels (Ctrl+C)

/// Configuration for an autonomous agent run.
#[derive(Debug, Clone)]
pub struct AutonomousConfig {
    /// The task to accomplish.
    pub task: String,
    /// Optional high-level goal description.
    pub goal: Option<String>,
    /// Optional shell command(s) to verify success (e.g., "cargo test").
    /// Multiple criteria separated by " && ".
    pub criteria: Option<String>,
    /// Optional cron schedule for recurring runs.
    pub schedule: Option<String>,
    /// Maximum number of agent turns before stopping.
    pub max_turns: u32,
}

impl Default for AutonomousConfig {
    fn default() -> Self {
        Self {
            task: String::new(),
            goal: None,
            criteria: None,
            schedule: None,
            max_turns: 30,
        }
    }
}

impl AutonomousConfig {
    /// Build the system prompt injection for the autonomous agent.
    ///
    /// This tells the model how to behave autonomously:
    /// - Work independently, don't ask for confirmation
    /// - Use tools freely
    /// - Check your work
    /// - Stop when the task is done
    #[must_use]
    pub fn build_autonomous_prompt(&self) -> String {
        let mut prompt = String::from(
            "You are an autonomous coding agent running in OpenAnalyst CLI. \
             Work independently to complete the task without asking for user confirmation. \
             Use tools freely: read files, write code, run tests, search the codebase.\n\n"
        );

        prompt.push_str(&format!("## Task\n{}\n\n", self.task));

        if let Some(ref goal) = self.goal {
            prompt.push_str(&format!("## Goal\n{goal}\n\n"));
        }

        if let Some(ref criteria) = self.criteria {
            prompt.push_str(&format!(
                "## Success Criteria\n\
                 After each iteration, verify your work by running: `{criteria}`\n\
                 If the check passes, you're done. If it fails, analyze the error and fix it.\n\n"
            ));
        }

        prompt.push_str(
            "## Rules\n\
             1. Work autonomously — do NOT ask for permission or confirmation\n\
             2. Read code before modifying it\n\
             3. After making changes, verify they work (run tests, compile, etc.)\n\
             4. If something fails, analyze the error and try a different approach\n\
             5. When the task is complete, say DONE and summarize what you did\n\
             6. Stay focused on the task — don't add unnecessary features\n"
        );

        prompt
    }

    /// Build a progress summary for display in the TUI.
    #[must_use]
    pub fn status_summary(&self) -> String {
        let mut parts = vec![format!("Task: {}", truncate(&self.task, 50))];
        if let Some(ref goal) = self.goal {
            parts.push(format!("Goal: {}", truncate(goal, 40)));
        }
        if let Some(ref criteria) = self.criteria {
            parts.push(format!("Criteria: {criteria}"));
        }
        parts.push(format!("Max turns: {}", self.max_turns));
        parts.join("\n")
    }
}

/// Check if success criteria pass by running the shell command.
pub fn check_criteria(criteria: &str) -> CriteriaResult {
    let output = std::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
        .args(if cfg!(windows) { vec!["/C", criteria] } else { vec!["-c", criteria] })
        .output();

    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout).to_string();
            let stderr = String::from_utf8_lossy(&result.stderr).to_string();
            if result.status.success() {
                CriteriaResult::Pass {
                    output: stdout,
                }
            } else {
                CriteriaResult::Fail {
                    exit_code: result.status.code().unwrap_or(-1),
                    stdout,
                    stderr,
                }
            }
        }
        Err(e) => CriteriaResult::Error {
            message: e.to_string(),
        },
    }
}

/// Result of checking success criteria.
#[derive(Debug, Clone)]
pub enum CriteriaResult {
    Pass { output: String },
    Fail { exit_code: i32, stdout: String, stderr: String },
    Error { message: String },
}

impl CriteriaResult {
    #[must_use]
    pub const fn passed(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }

    /// Format as feedback for the model to see.
    #[must_use]
    pub fn as_feedback(&self) -> String {
        match self {
            Self::Pass { output } => {
                let preview = truncate(output, 500);
                format!("✓ Criteria PASSED.\n{preview}")
            }
            Self::Fail { exit_code, stdout, stderr } => {
                let out = truncate(stdout, 300);
                let err = truncate(stderr, 300);
                format!("✗ Criteria FAILED (exit code {exit_code}).\nstdout:\n{out}\nstderr:\n{err}")
            }
            Self::Error { message } => {
                format!("⚠ Criteria check error: {message}")
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max - 3).collect();
        format!("{t}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_autonomous_prompt() {
        let config = AutonomousConfig {
            task: "fix the login bug".to_string(),
            goal: Some("all auth tests pass".to_string()),
            criteria: Some("cargo test --lib auth".to_string()),
            schedule: None,
            max_turns: 10,
        };
        let prompt = config.build_autonomous_prompt();
        assert!(prompt.contains("fix the login bug"));
        assert!(prompt.contains("cargo test --lib auth"));
        assert!(prompt.contains("autonomous"));
    }

    #[test]
    fn criteria_check_passes_for_true() {
        let result = check_criteria(if cfg!(windows) { "echo ok" } else { "true" });
        assert!(result.passed());
    }

    #[test]
    fn criteria_check_fails_for_false() {
        let result = check_criteria(if cfg!(windows) { "exit /b 1" } else { "false" });
        assert!(!result.passed());
    }
}
