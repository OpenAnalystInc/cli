//! Headless autonomous agent runner for OpenAnalyst CLI.
//!
//! This crate provides [`AgentRunner`], a non-interactive agent that takes a task
//! prompt and autonomously drives the LLM + tool loop to completion without user
//! interaction. It is used by the `openanalyst agent run` subcommand.

use std::collections::BTreeSet;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;

use api::{
    ContentBlockDelta, InputContentBlock, InputMessage, MessageRequest, MessageResponse,
    OutputContentBlock, StreamEvent as ApiStreamEvent, ToolChoice, ToolDefinition,
    ToolResultContentBlock,
};
use plugins::{PluginManager, PluginManagerConfig};
use runtime::{
    load_system_prompt, ApiClient, ApiRequest, AssistantEvent, ConfigLoader, ContentBlock,
    ConversationMessage, ConversationRuntime, MessageRole, PermissionMode, PermissionPolicy,
    RuntimeError, Session, TokenUsage, ToolError, ToolExecutor,
};
use tools::GlobalToolRegistry;

/// Set of allowed tool names.
pub type AllowedToolSet = BTreeSet<String>;

const DEFAULT_MODEL: &str = "openanalyst-beta";
const DEFAULT_MAX_TURNS: usize = 30;

/// Configuration for an autonomous agent run.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Model to use (e.g. "gemini-2.5-pro", "claude-opus-4-6")
    pub model: String,
    /// Maximum number of agentic turns before stopping
    pub max_turns: usize,
    /// Permission mode for tool execution
    pub permission_mode: PermissionMode,
    /// Optional set of allowed tools (None = all tools)
    pub allowed_tools: Option<AllowedToolSet>,
    /// Working directory for the agent
    pub cwd: PathBuf,
    /// Whether to print output to stdout
    pub verbose: bool,
    /// Additional system prompt context
    pub system_context: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: resolve_default_model(),
            max_turns: DEFAULT_MAX_TURNS,
            permission_mode: PermissionMode::WorkspaceWrite,
            allowed_tools: None,
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            verbose: false,
            system_context: None,
        }
    }
}

/// Result of a completed agent run.
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// Total turns the agent took
    pub turns: usize,
    /// Final text response from the agent
    pub final_text: String,
    /// All tool calls made during the run
    pub tool_calls: Vec<ToolCallRecord>,
    /// Total input tokens used
    pub input_tokens: u32,
    /// Total output tokens used
    pub output_tokens: u32,
    /// Wall clock duration
    pub duration_secs: f64,
}

/// A record of a single tool call made during the run.
#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub input: String,
    pub output: String,
    pub is_error: bool,
}

/// Headless autonomous agent runner.
pub struct AgentRunner {
    config: AgentConfig,
}

impl AgentRunner {
    #[must_use]
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    /// Execute the agent with the given task prompt. Runs the full agentic loop
    /// (LLM call -> tool execution -> LLM call -> ...) until the model stops
    /// requesting tools or `max_turns` is reached.
    ///
    /// # Errors
    ///
    /// Returns an error if the API client cannot be created, the system prompt
    /// fails to build, or the conversation loop encounters an unrecoverable error.
    pub fn run(&self, task: &str) -> Result<AgentResult, Box<dyn std::error::Error>> {
        let start = Instant::now();

        let system_prompt = self.build_system_prompt()?;
        let (feature_config, tool_registry) = self.build_plugin_state()?;

        let api_client = HeadlessApiClient::new(
            self.config.model.clone(),
            self.config.allowed_tools.clone(),
            tool_registry.clone(),
            self.config.verbose,
        )?;

        let tool_executor = HeadlessToolExecutor::new(
            self.config.allowed_tools.clone(),
            tool_registry.clone(),
            self.config.verbose,
        );

        let permission_policy = build_permission_policy(
            self.config.permission_mode,
            &tool_registry,
        );

        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
            feature_config,
        )
        .with_max_iterations(self.config.max_turns);

        // Auto-approve all permissions in agent mode (headless, no prompter)
        let summary = runtime.run_turn(task, None)?;

        let final_text = summary
            .assistant_messages
            .last()
            .map(|msg| {
                msg.blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        let tool_calls = summary
            .tool_results
            .iter()
            .flat_map(|msg| msg.blocks.iter())
            .filter_map(|block| match block {
                ContentBlock::ToolResult {
                    tool_name,
                    output,
                    is_error,
                    ..
                } => Some(ToolCallRecord {
                    tool_name: tool_name.clone(),
                    input: String::new(),
                    output: output.clone(),
                    is_error: *is_error,
                }),
                _ => None,
            })
            .collect();

        Ok(AgentResult {
            turns: summary.iterations,
            final_text,
            tool_calls,
            input_tokens: summary.usage.input_tokens,
            output_tokens: summary.usage.output_tokens,
            duration_secs: start.elapsed().as_secs_f64(),
        })
    }

    fn build_system_prompt(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let date = chrono_free_date();
        let mut prompt = load_system_prompt(
            self.config.cwd.clone(),
            date,
            env::consts::OS,
            "unknown",
        )?;
        if let Some(ctx) = &self.config.system_context {
            prompt.push(ctx.clone());
        }
        Ok(prompt)
    }

    fn build_plugin_state(
        &self,
    ) -> Result<(runtime::RuntimeFeatureConfig, GlobalToolRegistry), Box<dyn std::error::Error>>
    {
        let loader = ConfigLoader::default_for(&self.config.cwd);
        let runtime_config = loader.load()?;
        let plugin_settings = runtime_config.plugins();
        let mut plugin_config = PluginManagerConfig::new(loader.config_home().to_path_buf());
        plugin_config.enabled_plugins = plugin_settings.enabled_plugins().clone();
        let plugin_manager = PluginManager::new(plugin_config);
        let tool_registry =
            GlobalToolRegistry::with_plugin_tools(plugin_manager.aggregated_tools()?)?;
        Ok((runtime_config.feature_config().clone(), tool_registry))
    }
}

// ── Headless API Client ──

struct HeadlessApiClient {
    runtime: tokio::runtime::Runtime,
    client: api::ProviderClient,
    model: String,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    verbose: bool,
}

impl HeadlessApiClient {
    fn new(
        model: String,
        allowed_tools: Option<AllowedToolSet>,
        tool_registry: GlobalToolRegistry,
        verbose: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let client = api::ProviderClient::from_model(&model)?;
        Ok(Self {
            runtime: tokio::runtime::Runtime::new()?,
            client,
            model,
            allowed_tools,
            tool_registry,
            verbose,
        })
    }
}

impl ApiClient for HeadlessApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        let tools = filter_tool_specs(&self.tool_registry, self.allowed_tools.as_ref());
        let message_request = MessageRequest {
            model: self.model.clone(),
            max_tokens: max_tokens_for_model(&self.model),
            messages: convert_messages(&request.messages),
            system: (!request.system_prompt.is_empty())
                .then(|| request.system_prompt.join("\n\n")),
            tools: Some(tools),
            tool_choice: Some(ToolChoice::Auto),
            stream: true,
            thinking: None,
        };

        self.runtime.block_on(async {
            let mut stream = self
                .client
                .stream_message(&message_request)
                .await
                .map_err(|e| RuntimeError::new(e.to_string()))?;

            let mut events = Vec::new();
            let mut pending_tool: Option<(String, String, String)> = None;
            let mut saw_stop = false;

            while let Some(event) = stream
                .next_event()
                .await
                .map_err(|e| RuntimeError::new(e.to_string()))?
            {
                match event {
                    ApiStreamEvent::MessageStart(start) => {
                        for block in start.message.content {
                            push_output_block(block, &mut events, &mut pending_tool, self.verbose);
                        }
                    }
                    ApiStreamEvent::ContentBlockStart(start) => {
                        push_output_block(
                            start.content_block,
                            &mut events,
                            &mut pending_tool,
                            self.verbose,
                        );
                    }
                    ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                if self.verbose {
                                    let _ = write!(io::stdout(), "{text}");
                                    let _ = io::stdout().flush();
                                }
                                events.push(AssistantEvent::TextDelta(text));
                            }
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, input)) = &mut pending_tool {
                                input.push_str(&partial_json);
                            }
                        }
                        ContentBlockDelta::ThinkingDelta { .. }
                        | ContentBlockDelta::SignatureDelta { .. } => {}
                    },
                    ApiStreamEvent::ContentBlockStop(_) => {
                        if let Some((id, name, input)) = pending_tool.take() {
                            if self.verbose {
                                eprintln!("\n  [tool] {name}");
                            }
                            events.push(AssistantEvent::ToolUse { id, name, input });
                        }
                    }
                    ApiStreamEvent::MessageDelta(delta) => {
                        events.push(AssistantEvent::Usage(TokenUsage {
                            input_tokens: delta.usage.input_tokens,
                            output_tokens: delta.usage.output_tokens,
                            cache_creation_input_tokens: 0,
                            cache_read_input_tokens: 0,
                        }));
                    }
                    ApiStreamEvent::MessageStop(_) => {
                        saw_stop = true;
                        events.push(AssistantEvent::MessageStop);
                    }
                }
            }

            // Synthetic stop if stream ended without one but we got content
            if !saw_stop
                && events.iter().any(|e| {
                    matches!(e, AssistantEvent::TextDelta(t) if !t.is_empty())
                        || matches!(e, AssistantEvent::ToolUse { .. })
                })
            {
                events.push(AssistantEvent::MessageStop);
            }

            if events
                .iter()
                .any(|e| matches!(e, AssistantEvent::MessageStop))
            {
                return Ok(events);
            }

            // Fallback: non-streaming request
            let response = self
                .client
                .send_message(&MessageRequest {
                    stream: false,
            thinking: None,
                    ..message_request
                })
                .await
                .map_err(|e| RuntimeError::new(e.to_string()))?;
            Ok(response_to_events(response))
        })
    }
}

// ── Headless Tool Executor ──

struct HeadlessToolExecutor {
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    verbose: bool,
}

impl HeadlessToolExecutor {
    fn new(
        allowed_tools: Option<AllowedToolSet>,
        tool_registry: GlobalToolRegistry,
        verbose: bool,
    ) -> Self {
        Self {
            allowed_tools,
            tool_registry,
            verbose,
        }
    }
}

impl ToolExecutor for HeadlessToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        if self
            .allowed_tools
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(tool_name))
        {
            return Err(ToolError::new(format!(
                "tool `{tool_name}` is not enabled by the current allowed tools setting"
            )));
        }
        let value: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| ToolError::new(format!("invalid tool input JSON: {e}")))?;
        match self.tool_registry.execute(tool_name, &value) {
            Ok(output) => {
                if self.verbose {
                    let preview = if output.len() > 200 {
                        format!("{}...", &output[..200])
                    } else {
                        output.clone()
                    };
                    eprintln!("  [result] {tool_name}: {preview}");
                }
                Ok(output)
            }
            Err(error) => {
                if self.verbose {
                    eprintln!("  [error] {tool_name}: {error}");
                }
                Err(ToolError::new(error))
            }
        }
    }
}

// ── Shared helpers ──

fn resolve_default_model() -> String {
    env::var("OPENANALYST_MODEL")
        .or_else(|_| env::var("OPENANALYST_DEFAULT_MODEL"))
        .or_else(|_| env::var("ANTHROPIC_DEFAULT_SONNET_MODEL"))
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

fn max_tokens_for_model(model: &str) -> u32 {
    api::max_tokens_for_model(model)
}

fn chrono_free_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Civil date from Unix timestamp (days since 1970-01-01).
    // Algorithm from Howard Hinnant's civil_from_days, public domain.
    let mut days = (secs / 86400) as i64;
    days += 719_468; // shift epoch from 1970-01-01 to 0000-03-01
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097); // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // year of era
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month offset [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y}-{m:02}-{d:02}")
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content = message
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text { text: text.clone() },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or_else(|_| serde_json::json!({ "raw": input })),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect::<Vec<_>>();
            (!content.is_empty()).then(|| InputMessage {
                role: role.to_string(),
                content,
            })
        })
        .collect()
}

fn filter_tool_specs(
    registry: &GlobalToolRegistry,
    allowed: Option<&AllowedToolSet>,
) -> Vec<ToolDefinition> {
    registry.definitions(allowed.map(|s| {
        // Convert &BTreeSet<String> to a reference the registry expects
        s
    }))
}

fn push_output_block(
    block: OutputContentBlock,
    events: &mut Vec<AssistantEvent>,
    pending_tool: &mut Option<(String, String, String)>,
    verbose: bool,
) {
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
                if verbose {
                    let _ = write!(io::stdout(), "{text}");
                    let _ = io::stdout().flush();
                }
                events.push(AssistantEvent::TextDelta(text));
            }
        }
        OutputContentBlock::ToolUse { id, name, input } => {
            let input_str = if input.is_object() && input.as_object().unwrap().is_empty() {
                String::new()
            } else {
                input.to_string()
            };
            *pending_tool = Some((id, name, input_str));
        }
        OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
    }
}

fn response_to_events(response: MessageResponse) -> Vec<AssistantEvent> {
    let mut events = Vec::new();
    for block in response.content {
        match block {
            OutputContentBlock::Text { text } => {
                events.push(AssistantEvent::TextDelta(text));
            }
            OutputContentBlock::ToolUse { id, name, input } => {
                events.push(AssistantEvent::ToolUse {
                    id,
                    name,
                    input: input.to_string(),
                });
            }
            OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
        }
    }
    events.push(AssistantEvent::Usage(TokenUsage {
        input_tokens: response.usage.input_tokens,
        output_tokens: response.usage.output_tokens,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
    }));
    events.push(AssistantEvent::MessageStop);
    events
}

fn build_permission_policy(
    mode: PermissionMode,
    tool_registry: &GlobalToolRegistry,
) -> PermissionPolicy {
    tool_registry.permission_specs(None).into_iter().fold(
        PermissionPolicy::new(mode),
        |policy, (name, required_permission)| policy.with_tool_requirement(name, required_permission),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_expected_values() {
        let config = AgentConfig::default();
        assert_eq!(config.max_turns, DEFAULT_MAX_TURNS);
        assert_eq!(config.permission_mode, PermissionMode::WorkspaceWrite);
        assert!(config.allowed_tools.is_none());
        assert!(!config.verbose);
    }

    #[test]
    fn chrono_free_date_produces_valid_format() {
        let date = chrono_free_date();
        assert!(date.len() == 10, "date should be YYYY-MM-DD, got: {date}");
        let parts: Vec<&str> = date.split('-').collect();
        assert_eq!(parts.len(), 3, "date should have 3 parts: {date}");
        let year: u32 = parts[0].parse().expect("year should be numeric");
        let month: u32 = parts[1].parse().expect("month should be numeric");
        let day: u32 = parts[2].parse().expect("day should be numeric");
        assert!(year >= 2025 && year <= 2100, "year out of range: {year}");
        assert!((1..=12).contains(&month), "month out of range: {month}");
        assert!((1..=31).contains(&day), "day out of range: {day}");
    }
}
