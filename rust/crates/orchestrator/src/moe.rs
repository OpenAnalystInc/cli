//! Mixture of Experts (MOE) — multi-agent command chaining system.
//!
//! When a user chains 3+ slash commands (e.g., "/bughunter /commit /pr"),
//! the MOE dispatcher spawns one agent per command, routes each to the
//! optimal model tier, detects dependencies for ordering, and runs
//! independent commands in parallel.
//!
//! Commands chained with fewer than 3 run sequentially on the primary agent.
//! Mid-task skill injection allows new commands while agents are running.

use std::collections::HashMap;

use events::AgentType;

use crate::router::ActionCategory;

/// A parsed command in a chain.
#[derive(Debug, Clone)]
pub struct ChainedCommand {
    /// The raw command text (e.g., "/bughunter src/").
    pub raw: String,
    /// The command name (e.g., "bughunter").
    pub name: String,
    /// Arguments after the command name.
    pub args: String,
    /// What category of work this command does.
    pub category: ActionCategory,
    /// What agent type should handle this.
    pub agent_type: AgentType,
    /// Dependency: index of command that must complete before this one.
    pub depends_on: Option<usize>,
}

/// Result of parsing a multi-command input.
#[derive(Debug, Clone)]
pub enum ChainParseResult {
    /// Single command — handle normally.
    Single(String),
    /// Two commands — run sequentially.
    Sequential(Vec<ChainedCommand>),
    /// Three or more — spawn MOE agents.
    MoeDispatch(Vec<ChainedCommand>),
}

/// Parse an input line that may contain multiple chained slash commands.
///
/// Detection rules:
/// - Each command starts with `/` followed by a word
/// - Commands are separated by whitespace before the next `/`
/// - A single `/command with args` is treated as Single
/// - Arguments belong to the preceding command until the next `/`
#[must_use]
pub fn parse_command_chain(input: &str) -> ChainParseResult {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return ChainParseResult::Single(trimmed.to_string());
    }

    let commands = split_chained_commands(trimmed);

    match commands.len() {
        0 => ChainParseResult::Single(trimmed.to_string()),
        1 => ChainParseResult::Single(trimmed.to_string()),
        2 => ChainParseResult::Sequential(commands),
        _ => ChainParseResult::MoeDispatch(commands),
    }
}

/// Split input into individual ChainedCommands.
fn split_chained_commands(input: &str) -> Vec<ChainedCommand> {
    let mut commands = Vec::new();
    let mut current_start = None;

    // Find command boundaries: each `/word` at the start or after whitespace
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '/' && (i == 0 || chars[i - 1].is_whitespace()) {
            // Check if followed by a word character (not just a lone /)
            if i + 1 < chars.len() && chars[i + 1].is_alphanumeric() {
                if let Some(start) = current_start {
                    // Save previous command
                    let text: String = chars[start..i].iter().collect();
                    if let Some(cmd) = parse_single_command(text.trim(), commands.len()) {
                        commands.push(cmd);
                    }
                }
                current_start = Some(i);
            }
        }
        i += 1;
    }

    // Save last command
    if let Some(start) = current_start {
        let text: String = chars[start..].iter().collect();
        if let Some(cmd) = parse_single_command(text.trim(), commands.len()) {
            commands.push(cmd);
        }
    }

    // Detect dependencies
    resolve_dependencies(&mut commands);

    commands
}

/// Parse a single "/command args" into a ChainedCommand.
fn parse_single_command(text: &str, _index: usize) -> Option<ChainedCommand> {
    let stripped = text.strip_prefix('/')?;
    let mut parts = stripped.splitn(2, char::is_whitespace);
    let name = parts.next()?.to_string();
    let args = parts.next().unwrap_or("").trim().to_string();

    let (category, agent_type) = classify_command(&name);

    Some(ChainedCommand {
        raw: text.to_string(),
        name,
        args,
        category,
        agent_type,
        depends_on: None,
    })
}

/// Classify a slash command into an ActionCategory and AgentType (public for skill injection).
#[must_use]
pub fn classify_command_pub(name: &str) -> (ActionCategory, AgentType) {
    classify_command(name)
}

/// Classify a slash command into an ActionCategory and AgentType.
fn classify_command(name: &str) -> (ActionCategory, AgentType) {
    match name {
        // Explore commands — read-only, fast model
        "bughunter" | "diff" | "diff-review" | "doctor" | "status" | "cost"
        | "tokens" | "context" | "scrape" | "json" | "explore" => {
            (ActionCategory::Explore, AgentType::Explore)
        }

        // Research/Planning commands — balanced model
        "ultraplan" | "think" | "changelog" | "knowledge" => {
            (ActionCategory::Research, AgentType::Plan)
        }

        // Code commands — capable model
        "commit" | "commit-push-pr" | "branch" | "worktree" | "openanalyst" | "swarm" => {
            (ActionCategory::Code, AgentType::General)
        }

        // Write commands — balanced model
        "pr" | "issue" | "export" | "translate" | "diagram" => {
            (ActionCategory::Write, AgentType::General)
        }

        // Default: code (most commands involve code interaction)
        _ => (ActionCategory::Code, AgentType::General),
    }
}

/// Detect sequential dependencies between commands.
///
/// Rules:
/// - /commit depends on /bughunter (scan before commit)
/// - /pr depends on /commit (commit before PR)
/// - /commit-push-pr depends on everything before it
/// - Otherwise, commands are independent (can run in parallel)
fn resolve_dependencies(commands: &mut [ChainedCommand]) {
    // Build a name → index map (owned strings to avoid borrow)
    let name_index: HashMap<String, usize> = commands
        .iter()
        .enumerate()
        .map(|(i, cmd)| (cmd.name.clone(), i))
        .collect();

    // Known dependency pairs: (command, depends_on)
    let dep_rules: &[(&str, &[&str])] = &[
        ("commit", &["bughunter", "diff-review"]),
        ("commit-push-pr", &["bughunter", "diff-review", "commit"]),
        ("pr", &["commit", "commit-push-pr"]),
        ("issue", &["bughunter"]),
    ];

    // Collect assignments to avoid borrow conflict
    let mut assignments: Vec<(usize, usize)> = Vec::new();
    for (cmd_name, deps) in dep_rules {
        if let Some(&cmd_idx) = name_index.get(*cmd_name) {
            for dep in *deps {
                if let Some(&dep_idx) = name_index.get(*dep) {
                    if dep_idx < cmd_idx {
                        assignments.push((cmd_idx, dep_idx));
                        break;
                    }
                }
            }
        }
    }

    for (cmd_idx, dep_idx) in assignments {
        commands[cmd_idx].depends_on = Some(dep_idx);
    }
}

/// Build system prompt for an MOE agent handling a specific skill.
#[must_use]
pub fn build_moe_system_prompt(cmd: &ChainedCommand, total_agents: usize) -> String {
    format!(
        "You are agent {name} in a team of {total_agents} agents working in parallel.\n\
         Your role: execute the /{name} command{args}.\n\
         Focus exclusively on your assigned task. Be concise and efficient.\n\
         Do not duplicate work that other agents are handling.",
        name = cmd.name,
        args = if cmd.args.is_empty() {
            String::new()
        } else {
            format!(" with arguments: {}", cmd.args)
        },
    )
}

/// Convert a ChainedCommand into the prompt that would be sent to the AI.
/// This mirrors what the slash command handler does in TUI mode.
#[must_use]
pub fn command_to_prompt(cmd: &ChainedCommand) -> String {
    match cmd.name.as_str() {
        "bughunter" => {
            let scope = if cmd.args.is_empty() { "" } else { &cmd.args };
            format!(
                "You are a senior security and reliability engineer. Systematically inspect the codebase{} for:\n\
                 1. Logic bugs\n2. Error handling gaps\n3. Security issues\n4. Concurrency bugs\n\
                 Report each finding with file path, line number, severity, and fix.",
                if scope.is_empty() { String::new() } else { format!(" (scope: {scope})") }
            )
        }
        "commit" => "Review the current git diff. Generate a concise, descriptive commit message following conventional commits. Then stage and commit the changes.".to_string(),
        "pr" => {
            let ctx = if cmd.args.is_empty() { "the current branch changes" } else { &cmd.args };
            format!("Create a pull request for {ctx}. Write a clear title and description summarizing the changes.")
        }
        "diff-review" => {
            let file = if cmd.args.is_empty() { "" } else { &cmd.args };
            format!("Review the git diff{} and provide actionable code review feedback.", if file.is_empty() { String::new() } else { format!(" for {file}") })
        }
        "ultraplan" => {
            let task = if cmd.args.is_empty() { "the current task" } else { &cmd.args };
            format!("Create a detailed multi-step implementation plan for: {task}")
        }
        "changelog" => {
            format!("Generate a changelog from recent git commits.")
        }
        "issue" => {
            let ctx = if cmd.args.is_empty() { "a bug or feature" } else { &cmd.args };
            format!("Draft a GitHub issue for: {ctx}")
        }
        "translate" => format!("Translate: {}", cmd.args),
        "diagram" => format!("Create a Mermaid diagram for: {}", cmd.args),
        _ => {
            // Generic: just send the full command as a prompt
            if cmd.args.is_empty() {
                format!("Execute the /{} command.", cmd.name)
            } else {
                format!("Execute /{}: {}", cmd.name, cmd.args)
            }
        }
    }
}

/// Execution plan for MOE dispatch.
#[derive(Debug, Clone)]
pub struct MoePlan {
    /// Commands grouped by execution wave (parallel within wave, sequential across waves).
    pub waves: Vec<Vec<usize>>,
    /// All commands in the chain.
    pub commands: Vec<ChainedCommand>,
}

/// Build an execution plan from chained commands.
/// Groups independent commands into parallel waves, respecting dependencies.
#[must_use]
pub fn build_execution_plan(commands: Vec<ChainedCommand>) -> MoePlan {
    let n = commands.len();
    let mut completed = vec![false; n];
    let mut waves = Vec::new();

    loop {
        let mut wave = Vec::new();
        for i in 0..n {
            if completed[i] {
                continue;
            }
            // Check if dependency is satisfied
            let dep_satisfied = commands[i]
                .depends_on
                .map_or(true, |dep| completed[dep]);
            if dep_satisfied {
                wave.push(i);
            }
        }

        if wave.is_empty() {
            break; // All done or circular dependency
        }

        for &idx in &wave {
            completed[idx] = true;
        }
        waves.push(wave);
    }

    MoePlan { waves, commands }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_command_returns_single() {
        match parse_command_chain("/help") {
            ChainParseResult::Single(s) => assert_eq!(s, "/help"),
            other => panic!("expected Single, got {other:?}"),
        }
    }

    #[test]
    fn single_command_with_args() {
        match parse_command_chain("/bughunter src/") {
            ChainParseResult::Single(s) => assert_eq!(s, "/bughunter src/"),
            other => panic!("expected Single, got {other:?}"),
        }
    }

    #[test]
    fn two_commands_returns_sequential() {
        match parse_command_chain("/bughunter /commit") {
            ChainParseResult::Sequential(cmds) => {
                assert_eq!(cmds.len(), 2);
                assert_eq!(cmds[0].name, "bughunter");
                assert_eq!(cmds[1].name, "commit");
            }
            other => panic!("expected Sequential, got {other:?}"),
        }
    }

    #[test]
    fn three_commands_returns_moe() {
        match parse_command_chain("/bughunter /commit /pr") {
            ChainParseResult::MoeDispatch(cmds) => {
                assert_eq!(cmds.len(), 3);
                assert_eq!(cmds[0].name, "bughunter");
                assert_eq!(cmds[1].name, "commit");
                assert_eq!(cmds[2].name, "pr");
            }
            other => panic!("expected MoeDispatch, got {other:?}"),
        }
    }

    #[test]
    fn commands_with_args_parsed_correctly() {
        match parse_command_chain("/bughunter src/ /pr fix auth /issue bug report") {
            ChainParseResult::MoeDispatch(cmds) => {
                assert_eq!(cmds[0].name, "bughunter");
                assert_eq!(cmds[0].args, "src/");
                assert_eq!(cmds[1].name, "pr");
                assert_eq!(cmds[1].args, "fix auth");
                assert_eq!(cmds[2].name, "issue");
                assert_eq!(cmds[2].args, "bug report");
            }
            other => panic!("expected MoeDispatch, got {other:?}"),
        }
    }

    #[test]
    fn dependency_detection() {
        match parse_command_chain("/bughunter /commit /pr") {
            ChainParseResult::MoeDispatch(cmds) => {
                assert_eq!(cmds[0].depends_on, None); // bughunter: independent
                assert_eq!(cmds[1].depends_on, Some(0)); // commit depends on bughunter
                assert_eq!(cmds[2].depends_on, Some(1)); // pr depends on commit
            }
            other => panic!("expected MoeDispatch, got {other:?}"),
        }
    }

    #[test]
    fn independent_commands_no_deps() {
        match parse_command_chain("/translate hello /diagram auth flow /ultraplan refactor") {
            ChainParseResult::MoeDispatch(cmds) => {
                assert_eq!(cmds.len(), 3);
                assert!(cmds.iter().all(|c| c.depends_on.is_none()));
            }
            other => panic!("expected MoeDispatch, got {other:?}"),
        }
    }

    #[test]
    fn execution_plan_parallel_wave() {
        let input = "/translate hello /diagram auth flow /ultraplan refactor";
        if let ChainParseResult::MoeDispatch(cmds) = parse_command_chain(input) {
            let plan = build_execution_plan(cmds);
            // All independent → single wave with all 3
            assert_eq!(plan.waves.len(), 1);
            assert_eq!(plan.waves[0].len(), 3);
        }
    }

    #[test]
    fn execution_plan_sequential_waves() {
        let input = "/bughunter /commit /pr";
        if let ChainParseResult::MoeDispatch(cmds) = parse_command_chain(input) {
            let plan = build_execution_plan(cmds);
            // Sequential: bughunter → commit → pr (3 waves)
            assert_eq!(plan.waves.len(), 3);
            assert_eq!(plan.waves[0], vec![0]);
            assert_eq!(plan.waves[1], vec![1]);
            assert_eq!(plan.waves[2], vec![2]);
        }
    }

    #[test]
    fn execution_plan_mixed_parallel_sequential() {
        let input = "/bughunter /translate hello /commit /diagram flow";
        if let ChainParseResult::MoeDispatch(cmds) = parse_command_chain(input) {
            let plan = build_execution_plan(cmds);
            // Wave 1: bughunter (0) + translate (1) + diagram (3) — independent
            // Wave 2: commit (2) — depends on bughunter
            assert_eq!(plan.waves.len(), 2);
            assert!(plan.waves[0].contains(&0));
            assert!(plan.waves[0].contains(&1));
            assert!(plan.waves[0].contains(&3));
            assert_eq!(plan.waves[1], vec![2]);
        }
    }

    #[test]
    fn command_classification() {
        let (cat, _) = classify_command("bughunter");
        assert_eq!(cat, ActionCategory::Explore);

        let (cat, _) = classify_command("ultraplan");
        assert_eq!(cat, ActionCategory::Research);

        let (cat, _) = classify_command("commit");
        assert_eq!(cat, ActionCategory::Code);

        let (cat, _) = classify_command("pr");
        assert_eq!(cat, ActionCategory::Write);
    }

    #[test]
    fn non_slash_input_is_single() {
        match parse_command_chain("just a regular prompt") {
            ChainParseResult::Single(s) => assert_eq!(s, "just a regular prompt"),
            other => panic!("expected Single, got {other:?}"),
        }
    }
}
