//! Agent worker — runs `ConversationRuntime` in a blocking thread
//! with channel-based `ApiClient`, `ToolExecutor`, and `PermissionPrompter`.
//!
//! This is the bridge between the sync runtime and the async TUI.

use std::sync::Arc;
use std::time::{Duration, Instant};

use events::{AgentId, UiEvent, UiEventTx};
use runtime::{
    ApiClient, ApiRequest, AssistantEvent, ContentBlock, MessageRole, PermissionPromptDecision,
    PermissionPrompter, PermissionRequest, RuntimeError, ToolError, ToolExecutor,
};
use tokio::sync::Mutex;

use api::{InputContentBlock, InputMessage, ToolResultContentBlock};

use crate::registry::AgentRegistry;
use crate::OrchestratorConfig;

/// Timeout for individual stream events (30 seconds).
const STREAM_EVENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Run a single agent turn in a blocking thread.
///
/// Uses `spawn_blocking` to bridge the sync `ConversationRuntime` to the async world.
/// A dedicated tokio runtime is created inside the blocking thread for async API calls.
pub async fn run_agent_turn(
    agent_id: AgentId,
    prompt: String,
    config: OrchestratorConfig,
    ui_tx: UiEventTx,
    effort_budget: Option<u32>,
    registry: Arc<Mutex<AgentRegistry>>,
) -> Result<(), String> {
    let result = tokio::task::spawn_blocking(move || {
        run_turn_blocking(agent_id, &prompt, &config, &ui_tx, effort_budget, registry)
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => Err(err),
        Err(join_err) => Err(format!("Agent task panicked: {join_err}")),
    }
}

/// The actual blocking turn execution.
fn run_turn_blocking(
    agent_id: AgentId,
    prompt: &str,
    config: &OrchestratorConfig,
    ui_tx: &UiEventTx,
    effort_budget: Option<u32>,
    registry: Arc<Mutex<AgentRegistry>>,
) -> Result<(), String> {
    use plugins::{PluginManager, PluginManagerConfig};
    use runtime::{ConfigLoader, ConversationRuntime, Session};
    use tools::GlobalToolRegistry;

    // Load configuration
    let loader = ConfigLoader::default_for(&config.cwd);
    let runtime_config = loader.load().map_err(|e| e.to_string())?;
    let feature_config = runtime_config.feature_config().clone();

    // Load plugins and tool registry
    let plugin_settings = runtime_config.plugins();
    let mut plugin_config = PluginManagerConfig::new(loader.config_home().to_path_buf());
    plugin_config.enabled_plugins = plugin_settings.enabled_plugins().clone();
    let plugin_manager = PluginManager::new(plugin_config);
    let mut tool_registry = GlobalToolRegistry::with_plugin_tools(
        plugin_manager.aggregated_tools().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    // Bootstrap MCP servers and register their tools
    let mcp_config = runtime_config.mcp();
    let mcp_connections = runtime::mcp_bridge::bootstrap_mcp_servers(mcp_config.servers());
    let mut mcp_registered = Vec::new();
    for conn in &mcp_connections {
        for tool in &conn.tools {
            mcp_registered.push(tools::McpRegisteredTool {
                name: tool.full_name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                server_name: conn.server_name.clone(),
                original_name: tool.original_name.clone(),
            });
        }
    }
    if !mcp_registered.is_empty() {
        tool_registry.register_mcp_tools(mcp_registered);
    }

    // Create a dedicated tokio runtime for async API calls.
    // This is safe because we're inside spawn_blocking (a non-tokio thread).
    let async_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    // Create channel-based implementations
    let api_client = ChannelApiClient {
        agent_id: agent_id.clone(),
        runtime: async_rt,
        client: api::ProviderClient::from_model(&config.model).map_err(|e| e.to_string())?,
        model: config.model.clone(),
        allowed_tools: config.allowed_tools.clone(),
        tool_registry: tool_registry.clone(),
        ui_tx: ui_tx.clone(),
        effort_budget,
    };

    let tool_executor = ChannelToolExecutor {
        agent_id: agent_id.clone(),
        tool_registry,
        ui_tx: ui_tx.clone(),
    };

    let permission_policy = runtime::PermissionPolicy::new(config.permission_mode);

    // Build the conversation runtime
    let session = Session::default();
    let mut runtime = ConversationRuntime::new_with_features(
        session,
        api_client,
        tool_executor,
        permission_policy,
        config.system_prompt.clone(),
        feature_config,
    )
    .with_max_iterations(100);

    // Permission prompter that sends requests to the TUI and blocks until user responds
    let mut prompter = TuiPermissionPrompter {
        agent_id: agent_id.to_string(),
        ui_tx: ui_tx.clone(),
        registry,
    };

    let _summary = runtime
        .run_turn(prompt, Some(&mut prompter))
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ── Channel-based ApiClient ──

/// `ApiClient` implementation that sends streaming events to the TUI via channels.
/// Uses a dedicated single-threaded tokio runtime for async API calls.
struct ChannelApiClient {
    agent_id: AgentId,
    runtime: tokio::runtime::Runtime,
    client: api::ProviderClient,
    model: String,
    allowed_tools: Option<std::collections::BTreeSet<String>>,
    tool_registry: tools::GlobalToolRegistry,
    ui_tx: UiEventTx,
    effort_budget: Option<u32>,
}

impl ApiClient for ChannelApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        use api::{
            ContentBlockDelta, MessageRequest, StreamEvent as ApiStreamEvent, ToolDefinition,
        };

        let tool_defs: Vec<ToolDefinition> = self
            .tool_registry
            .definitions(self.allowed_tools.as_ref())
            .into_iter()
            .collect();

        let messages: Vec<InputMessage> = convert_messages(&request.messages);

        let system_text = request.system_prompt.join("\n\n");
        let api_request = MessageRequest {
            model: self.model.clone(),
            max_tokens: api::max_tokens_for_model(&self.model),
            messages,
            system: if system_text.is_empty() {
                None
            } else {
                Some(system_text)
            },
            tools: if tool_defs.is_empty() {
                None
            } else {
                Some(tool_defs)
            },
            tool_choice: None,
            stream: true,
            thinking: self.effort_budget.map(|budget| api::ThinkingConfig {
                thinking_type: "enabled".to_string(),
                budget_tokens: budget,
            }),
        };

        let ui_tx = self.ui_tx.clone();
        let agent_id = self.agent_id.clone();

        self.runtime.block_on(async {
            let mut stream = self
                .client
                .stream_message(&api_request)
                .await
                .map_err(|e| RuntimeError::new(e.to_string()))?;

            let mut events = Vec::new();
            let mut pending_tool: Option<(String, String, String)> = None;

            // Stream with per-event timeout to detect hung connections
            loop {
                let next = tokio::time::timeout(
                    STREAM_EVENT_TIMEOUT,
                    stream.next_event(),
                );
                match next.await {
                    Ok(Ok(Some(event))) => {
                        match event {
                            ApiStreamEvent::ContentBlockStart(start) => {
                                if let api::OutputContentBlock::ToolUse { id, name, .. } =
                                    &start.content_block
                                {
                                    pending_tool =
                                        Some((id.clone(), name.clone(), String::new()));
                                }
                            }
                            ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                                ContentBlockDelta::TextDelta { text } => {
                                    if !text.is_empty() {
                                        let _ = ui_tx
                                            .send(UiEvent::StreamDelta {
                                                agent_id: agent_id.clone(),
                                                text: text.clone(),
                                            })
                                            .await;
                                        events.push(AssistantEvent::TextDelta(text));
                                    }
                                }
                                ContentBlockDelta::InputJsonDelta { partial_json } => {
                                    if let Some((_, _, input)) = &mut pending_tool {
                                        input.push_str(&partial_json);
                                    }
                                }
                                _ => {}
                            },
                            ApiStreamEvent::ContentBlockStop(_) => {
                                if let Some((id, name, input)) = pending_tool.take() {
                                    let _ = ui_tx
                                        .send(UiEvent::ToolCallStart {
                                            agent_id: agent_id.clone(),
                                            call_id: id.clone(),
                                            tool_name: name.clone(),
                                            input_preview: truncate_utf8(&input, 120),
                                        })
                                        .await;
                                    events.push(AssistantEvent::ToolUse { id, name, input });
                                }
                            }
                            ApiStreamEvent::MessageDelta(delta) => {
                                let usage = runtime::TokenUsage {
                                    input_tokens: delta.usage.input_tokens,
                                    output_tokens: delta.usage.output_tokens,
                                    cache_creation_input_tokens: 0,
                                    cache_read_input_tokens: 0,
                                };
                                let _ = ui_tx
                                    .send(UiEvent::UsageUpdate {
                                        agent_id: agent_id.clone(),
                                        input_tokens: usage.input_tokens,
                                        output_tokens: usage.output_tokens,
                                    })
                                    .await;
                                events.push(AssistantEvent::Usage(usage));
                            }
                            ApiStreamEvent::MessageStop(_) => {
                                events.push(AssistantEvent::MessageStop);
                                break; // Clean exit
                            }
                            _ => {}
                        }
                    }
                    Ok(Ok(None)) => break, // Stream ended
                    Ok(Err(e)) => {
                        return Err(RuntimeError::new(format!("Stream error: {e}")));
                    }
                    Err(_elapsed) => {
                        return Err(RuntimeError::new(
                            "Stream timed out — no data received for 30 seconds. \
                             The provider may be overloaded or the connection was lost."
                                .to_string(),
                        ));
                    }
                }
            }

            if !events
                .iter()
                .any(|e| matches!(e, AssistantEvent::MessageStop))
            {
                events.push(AssistantEvent::MessageStop);
            }

            Ok(events)
        })
    }
}

// ── Channel-based ToolExecutor ──

/// `ToolExecutor` implementation that sends tool call events to the TUI.
struct ChannelToolExecutor {
    agent_id: AgentId,
    tool_registry: tools::GlobalToolRegistry,
    ui_tx: UiEventTx,
}

impl ToolExecutor for ChannelToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        let start = Instant::now();

        let input_value: serde_json::Value =
            serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string()));

        let result = self.tool_registry.execute(tool_name, &input_value);
        let duration = start.elapsed();

        let (output, is_error) = match &result {
            Ok(output) => (output.clone(), false),
            Err(err) => (err.clone(), true),
        };

        // Send tool completion event to TUI (blocking_send is safe from spawn_blocking)
        let _ = self.ui_tx.blocking_send(UiEvent::ToolCallEnd {
            agent_id: self.agent_id.clone(),
            call_id: String::new(),
            output: truncate_utf8(&output, 500),
            is_error,
            duration,
        });

        result.map_err(ToolError::new)
    }
}

// ── TUI Permission Prompter ──

/// Permission prompter that sends requests to the TUI dialog and blocks
/// until the user responds (Allow/Deny) via the oneshot channel in the registry.
struct TuiPermissionPrompter {
    agent_id: String,
    ui_tx: UiEventTx,
    registry: Arc<Mutex<AgentRegistry>>,
}

impl PermissionPrompter for TuiPermissionPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
        let request_id = format!(
            "perm-{}-{}-{}",
            self.agent_id,
            request.tool_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        // Create a oneshot channel for the response
        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();

        // Register the oneshot in the registry so the orchestrator can resolve it.
        // We use blocking_lock because we're in a sync context (spawn_blocking).
        {
            let mut reg = self.registry.blocking_lock();
            reg.register_permission(request_id.clone(), tx);
        }

        // Send permission request to TUI — shows the dialog
        let _ = self.ui_tx.blocking_send(UiEvent::PermissionRequest {
            request_id: request_id.clone(),
            agent_id: self.agent_id.clone(),
            tool_name: request.tool_name.clone(),
            input: request.input.clone(),
            required_mode: format!("{:?}", request.required_mode),
        });

        // Block until the user responds or timeout expires.
        // The TUI sends Action::PermissionResponse → orchestrator → registry.resolve_permission → oneshot.
        match rx.blocking_recv() {
            Ok(true) => PermissionPromptDecision::Allow,
            Ok(false) => PermissionPromptDecision::Deny {
                reason: "Denied by user".to_string(),
            },
            Err(_) => {
                // Channel dropped — TUI closed or timeout. Deny for safety.
                PermissionPromptDecision::Deny {
                    reason: "Permission prompt timed out or was cancelled".to_string(),
                }
            }
        }
    }
}

// ── Helpers ──

/// Convert runtime `ConversationMessage` to API `InputMessage`.
fn convert_messages(messages: &[runtime::ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content: Vec<InputContentBlock> = message
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
                .collect();
            (!content.is_empty()).then(|| InputMessage {
                role: role.to_string(),
                content,
            })
        })
        .collect()
}

/// UTF-8 safe string truncation. Never panics on multi-byte characters.
fn truncate_utf8(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else if max_chars > 3 {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{truncated}...")
    } else {
        s.chars().take(max_chars).collect()
    }
}
