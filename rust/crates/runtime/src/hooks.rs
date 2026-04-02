use std::ffi::OsStr;
use std::process::Command;

use serde_json::json;

use crate::config::{RuntimeFeatureConfig, RuntimeHookConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    CwdChanged,
    FileChanged,
    SessionStart,
    SessionEnd,
    TaskCreated,
    Notification,
    Stop,
}

impl HookEvent {
    fn as_str(self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::CwdChanged => "CwdChanged",
            Self::FileChanged => "FileChanged",
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::TaskCreated => "TaskCreated",
            Self::Notification => "Notification",
            Self::Stop => "Stop",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRunResult {
    denied: bool,
    messages: Vec<String>,
}

impl HookRunResult {
    #[must_use]
    pub fn allow(messages: Vec<String>) -> Self {
        Self {
            denied: false,
            messages,
        }
    }

    #[must_use]
    pub fn is_denied(&self) -> bool {
        self.denied
    }

    #[must_use]
    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HookRunner {
    config: RuntimeHookConfig,
}

#[derive(Debug, Clone, Copy)]
struct HookCommandRequest<'a> {
    event: HookEvent,
    tool_name: &'a str,
    tool_input: &'a str,
    tool_output: Option<&'a str>,
    is_error: bool,
    payload: &'a str,
}

impl HookRunner {
    #[must_use]
    pub fn new(config: RuntimeHookConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn from_feature_config(feature_config: &RuntimeFeatureConfig) -> Self {
        Self::new(feature_config.hooks().clone())
    }

    #[must_use]
    pub fn run_pre_tool_use(&self, tool_name: &str, tool_input: &str) -> HookRunResult {
        self.run_commands(
            HookEvent::PreToolUse,
            self.config.pre_tool_use(),
            tool_name,
            tool_input,
            None,
            false,
        )
    }

    /// Run CwdChanged hooks when the working directory changes.
    pub fn run_cwd_changed(&self, old_cwd: &str, new_cwd: &str) -> HookRunResult {
        let commands = self.config.cwd_changed();
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }
        self.run_commands(
            HookEvent::CwdChanged,
            commands,
            "cwd",
            &serde_json::json!({"old_cwd": old_cwd, "new_cwd": new_cwd}).to_string(),
            None,
            false,
        )
    }

    /// Run FileChanged hooks when a file is modified externally (e.g., format-on-save).
    pub fn run_file_changed(&self, file_path: &str, tool_name: &str) -> HookRunResult {
        let commands = self.config.file_changed();
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }
        self.run_commands(
            HookEvent::FileChanged,
            commands,
            tool_name,
            &serde_json::json!({"file_path": file_path}).to_string(),
            None,
            false,
        )
    }

    /// Run SessionStart hooks when a session begins.
    pub fn run_session_start(&self, session_id: &str) -> HookRunResult {
        let commands = self.config.session_start();
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }
        self.run_commands(
            HookEvent::SessionStart,
            commands,
            "session",
            &serde_json::json!({"session_id": session_id}).to_string(),
            None,
            false,
        )
    }

    /// Run SessionEnd hooks when a session is ending.
    pub fn run_session_end(&self, reason: &str) -> HookRunResult {
        let commands = self.config.session_end();
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }
        self.run_commands(
            HookEvent::SessionEnd,
            commands,
            "session",
            &serde_json::json!({"reason": reason}).to_string(),
            None,
            false,
        )
    }

    /// Run TaskCreated hooks when a task is created.
    pub fn run_task_created(&self, task_id: &str, subject: &str) -> HookRunResult {
        let commands = self.config.task_created();
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }
        self.run_commands(
            HookEvent::TaskCreated,
            commands,
            "task",
            &serde_json::json!({"task_id": task_id, "subject": subject}).to_string(),
            None,
            false,
        )
    }

    /// Run Notification hooks when the agent wants to notify the user.
    pub fn run_notification(&self, message: &str) -> HookRunResult {
        let commands = self.config.notification();
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }
        self.run_commands(
            HookEvent::Notification,
            commands,
            "notification",
            &serde_json::json!({"message": message}).to_string(),
            None,
            false,
        )
    }

    /// Run Stop hooks when the agent is stopping execution.
    pub fn run_stop(&self, reason: &str) -> HookRunResult {
        let commands = self.config.stop();
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }
        self.run_commands(
            HookEvent::Stop,
            commands,
            "stop",
            &serde_json::json!({"reason": reason}).to_string(),
            None,
            false,
        )
    }

    #[must_use]
    pub fn run_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &str,
        tool_output: &str,
        is_error: bool,
    ) -> HookRunResult {
        self.run_commands(
            HookEvent::PostToolUse,
            self.config.post_tool_use(),
            tool_name,
            tool_input,
            Some(tool_output),
            is_error,
        )
    }

    fn run_commands(
        &self,
        event: HookEvent,
        commands: &[String],
        tool_name: &str,
        tool_input: &str,
        tool_output: Option<&str>,
        is_error: bool,
    ) -> HookRunResult {
        if commands.is_empty() {
            return HookRunResult::allow(Vec::new());
        }

        let payload = json!({
            "hook_event_name": event.as_str(),
            "tool_name": tool_name,
            "tool_input": parse_tool_input(tool_input),
            "tool_input_json": tool_input,
            "tool_output": tool_output,
            "tool_result_is_error": is_error,
        })
        .to_string();

        let mut messages = Vec::new();

        for command in commands {
            match Self::run_command(
                command,
                HookCommandRequest {
                    event,
                    tool_name,
                    tool_input,
                    tool_output,
                    is_error,
                    payload: &payload,
                },
            ) {
                HookCommandOutcome::Allow { message } => {
                    if let Some(message) = message {
                        messages.push(message);
                    }
                }
                HookCommandOutcome::Deny { message } => {
                    let message = message.unwrap_or_else(|| {
                        format!("{} hook denied tool `{tool_name}`", event.as_str())
                    });
                    messages.push(message);
                    return HookRunResult {
                        denied: true,
                        messages,
                    };
                }
                HookCommandOutcome::Warn { message } => messages.push(message),
            }
        }

        HookRunResult::allow(messages)
    }

    fn run_command(command: &str, request: HookCommandRequest<'_>) -> HookCommandOutcome {
        let mut child = shell_command(command);
        child.stdin(std::process::Stdio::piped());
        child.stdout(std::process::Stdio::piped());
        child.stderr(std::process::Stdio::piped());
        child.env("HOOK_EVENT", request.event.as_str());
        child.env("HOOK_TOOL_NAME", request.tool_name);
        child.env("HOOK_TOOL_INPUT", request.tool_input);
        child.env(
            "HOOK_TOOL_IS_ERROR",
            if request.is_error { "1" } else { "0" },
        );
        if let Some(tool_output) = request.tool_output {
            child.env("HOOK_TOOL_OUTPUT", tool_output);
        }

        match child.output_with_stdin(request.payload.as_bytes()) {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let message = (!stdout.is_empty()).then_some(stdout);
                match output.status.code() {
                    Some(0) => HookCommandOutcome::Allow { message },
                    Some(2) => HookCommandOutcome::Deny { message },
                    Some(code) => HookCommandOutcome::Warn {
                        message: format_hook_warning(
                            command,
                            code,
                            message.as_deref(),
                            stderr.as_str(),
                        ),
                    },
                    None => HookCommandOutcome::Warn {
                        message: format!(
                            "{} hook `{command}` terminated by signal while handling `{}`",
                            request.event.as_str(),
                            request.tool_name
                        ),
                    },
                }
            }
            Err(error) => HookCommandOutcome::Warn {
                message: format!(
                    "{} hook `{command}` failed to start for `{}`: {error}",
                    request.event.as_str(),
                    request.tool_name
                ),
            },
        }
    }
}

enum HookCommandOutcome {
    Allow { message: Option<String> },
    Deny { message: Option<String> },
    Warn { message: String },
}

fn parse_tool_input(tool_input: &str) -> serde_json::Value {
    serde_json::from_str(tool_input).unwrap_or_else(|_| json!({ "raw": tool_input }))
}

fn format_hook_warning(command: &str, code: i32, stdout: Option<&str>, stderr: &str) -> String {
    let mut message =
        format!("Hook `{command}` exited with status {code}; allowing tool execution to continue");
    if let Some(stdout) = stdout.filter(|stdout| !stdout.is_empty()) {
        message.push_str(": ");
        message.push_str(stdout);
    } else if !stderr.is_empty() {
        message.push_str(": ");
        message.push_str(stderr);
    }
    message
}

fn shell_command(command: &str) -> CommandWithStdin {
    let mut cmd = Command::new("sh");
    cmd.arg("-lc").arg(command);
    CommandWithStdin::new(cmd)
}

struct CommandWithStdin {
    command: Command,
}

impl CommandWithStdin {
    fn new(command: Command) -> Self {
        Self { command }
    }

    fn stdin(&mut self, cfg: std::process::Stdio) -> &mut Self {
        self.command.stdin(cfg);
        self
    }

    fn stdout(&mut self, cfg: std::process::Stdio) -> &mut Self {
        self.command.stdout(cfg);
        self
    }

    fn stderr(&mut self, cfg: std::process::Stdio) -> &mut Self {
        self.command.stderr(cfg);
        self
    }

    fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.command.env(key, value);
        self
    }

    fn output_with_stdin(&mut self, stdin: &[u8]) -> std::io::Result<std::process::Output> {
        let mut child = self.command.spawn()?;
        if let Some(mut child_stdin) = child.stdin.take() {
            use std::io::Write;
            child_stdin.write_all(stdin)?;
        }
        child.wait_with_output()
    }
}

#[cfg(test)]
mod tests {
    use super::{HookRunResult, HookRunner};
    use crate::config::{RuntimeFeatureConfig, RuntimeHookConfig};

    #[test]
    fn allows_exit_code_zero_and_captures_stdout() {
        let _guard = crate::test_env_lock();
        let runner = HookRunner::new(RuntimeHookConfig::new(
            vec![shell_snippet("printf 'pre ok'")],
            Vec::new(),
        ));

        let result = runner.run_pre_tool_use("Read", r#"{"path":"README.md"}"#);

        assert_eq!(result, HookRunResult::allow(vec!["pre ok".to_string()]));
    }

    #[test]
    fn denies_exit_code_two() {
        let _guard = crate::test_env_lock();
        let runner = HookRunner::new(RuntimeHookConfig::new(
            vec![shell_snippet("printf 'blocked by hook'; exit 2")],
            Vec::new(),
        ));

        let result = runner.run_pre_tool_use("Bash", r#"{"command":"pwd"}"#);

        assert!(result.is_denied());
        assert_eq!(result.messages(), &["blocked by hook".to_string()]);
    }

    #[test]
    fn warns_for_other_non_zero_statuses() {
        let _guard = crate::test_env_lock();
        let runner = HookRunner::from_feature_config(&RuntimeFeatureConfig::default().with_hooks(
            RuntimeHookConfig::new(
                vec![shell_snippet("printf 'warning hook'; exit 1")],
                Vec::new(),
            ),
        ));

        let result = runner.run_pre_tool_use("Edit", r#"{"file":"src/lib.rs"}"#);

        assert!(!result.is_denied());
        assert!(result
            .messages()
            .iter()
            .any(|message| message.contains("allowing tool execution to continue")));
    }

    fn shell_snippet(script: &str) -> String {
        script.to_string()
    }
}
