//! Agent worker — runs `ConversationRuntime` in a blocking thread
//! with channel-based `ApiClient`, `ToolExecutor`, and `PermissionPrompter`.
//!
//! This is the bridge between the sync runtime and the async TUI.

use std::sync::Arc;
use std::time::{Duration, Instant};

use events::{AgentId, DiffHunk, DiffInfo, DiffLine, UiEvent, UiEventTx};
use runtime::{
    ApiClient, ApiRequest, AssistantEvent, ContentBlock, MessageRole, PermissionPromptDecision,
    PermissionPrompter, PermissionRequest, RuntimeError, ToolError, ToolExecutor,
};
use tokio::sync::Mutex;

use api::{InputContentBlock, InputMessage, ToolResultContentBlock};

use crate::loop_detection::LoopDetectionService;
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

    // Loop detection service — prevents infinite agent loops
    let loop_detector = std::sync::Arc::new(std::sync::Mutex::new(
        LoopDetectionService::new(config.max_turns.unwrap_or(200))
    ));

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
        _loop_detector: loop_detector.clone(),
    };

    let tool_executor = ChannelToolExecutor {
        agent_id: agent_id.clone(),
        tool_registry,
        ui_tx: ui_tx.clone(),
        mcp_connections,
        loop_detector,
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
    _loop_detector: std::sync::Arc<std::sync::Mutex<LoopDetectionService>>,
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
                                        if ui_tx
                                            .send(UiEvent::StreamDelta {
                                                agent_id: agent_id.clone(),
                                                text: text.clone(),
                                            })
                                            .await
                                            .is_err()
                                        {
                                            eprintln!("[worker] TUI channel closed — event dropped");
                                        }
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
                                    if ui_tx
                                        .send(UiEvent::ToolCallStart {
                                            agent_id: agent_id.clone(),
                                            call_id: id.clone(),
                                            tool_name: name.clone(),
                                            input_preview: truncate_utf8(&input, 120),
                                        })
                                        .await
                                        .is_err()
                                    {
                                        eprintln!("[worker] TUI channel closed — event dropped");
                                    }
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
                                if ui_tx
                                    .send(UiEvent::UsageUpdate {
                                        agent_id: agent_id.clone(),
                                        input_tokens: usage.input_tokens,
                                        output_tokens: usage.output_tokens,
                                    })
                                    .await
                                    .is_err()
                                {
                                    eprintln!("[worker] TUI channel closed — event dropped");
                                }
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
/// Handles built-in tools, plugin tools, and MCP tools.
struct ChannelToolExecutor {
    agent_id: AgentId,
    tool_registry: tools::GlobalToolRegistry,
    ui_tx: UiEventTx,
    mcp_connections: Vec<runtime::mcp_bridge::McpConnection>,
    loop_detector: std::sync::Arc<std::sync::Mutex<LoopDetectionService>>,
}

impl ToolExecutor for ChannelToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        // Loop detection: check for repeated identical tool calls
        {
            if let Ok(mut detector) = self.loop_detector.lock() {
                let result = detector.check_tool_call(tool_name, input);
                if result.is_loop() {
                    let detail = result.detail.unwrap_or_else(|| "Infinite loop detected".to_string());
                    return Err(ToolError::new(format!(
                        "Loop detected — aborting to prevent infinite execution. {detail}"
                    )));
                }
            }
        }

        let start = Instant::now();

        let input_value: serde_json::Value =
            serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string()));

        // Format-on-save resilience: capture file mtime before edit/write tools
        let is_file_tool = matches!(tool_name, "edit_file" | "Edit" | "write_file" | "Write");
        let file_path = if is_file_tool {
            input_value.get("file_path")
                .or_else(|| input_value.get("path"))
                .and_then(|v| v.as_str())
                .map(String::from)
        } else {
            None
        };
        let mtime_before = file_path.as_deref().and_then(|p| {
            std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
        });

        // Route MCP tools to their server connection
        let result = if tool_name.starts_with("mcp__") {
            self.execute_mcp_tool(tool_name, &input_value)
        } else {
            self.tool_registry.execute(tool_name, &input_value)
        };
        let duration = start.elapsed();

        // Format-on-save resilience: if a PostToolUse hook (formatter) changed the file
        // between our write and now, don't treat it as an error on the next edit.
        // Instead, note the external modification for the AI to re-read.
        let (mut output, is_error) = match &result {
            Ok(output) => (output.clone(), false),
            Err(err) => (err.clone(), true),
        };

        // Output masking: redact secrets before they reach the LLM or UI
        if runtime::output_masking::likely_contains_secrets(&output) {
            let (masked, _stats) = runtime::output_masking::mask_tool_output(&output);
            output = masked.into_owned();
        }

        if is_file_tool && !is_error {
            if let (Some(path), Some(before)) = (&file_path, mtime_before) {
                // Brief pause to let formatters run
                std::thread::sleep(std::time::Duration::from_millis(50));
                let mtime_after = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
                if let Some(after) = mtime_after {
                    if after > before {
                        // File was modified externally (likely format-on-save)
                        output.push_str("\n[Note: file was reformatted by an external tool after write]");
                    }
                }
            }
        }

        // Extract structured diff info for edit/write tools
        let diff = if is_file_tool && !is_error {
            parse_diff_info(&output)
        } else {
            None
        };

        // Send tool completion event to TUI
        if self.ui_tx.blocking_send(UiEvent::ToolCallEnd {
            agent_id: self.agent_id.clone(),
            call_id: String::new(),
            output: truncate_utf8(&output, 500),
            is_error,
            duration,
            diff,
        }).is_err() {
            eprintln!("[worker] TUI channel closed — event dropped");
        }

        if is_error {
            Err(ToolError::new(output))
        } else {
            Ok(output)
        }
    }
}

impl ChannelToolExecutor {
    /// Execute an MCP tool by finding its server connection and dispatching the call.
    fn execute_mcp_tool(&mut self, tool_name: &str, input: &serde_json::Value) -> Result<String, String> {
        for conn in &mut self.mcp_connections {
            if let Some(tool) = conn.tools.iter().find(|t| t.full_name == tool_name) {
                let original_name = tool.original_name.clone();
                return runtime::mcp_bridge::call_tool(conn, &original_name, input.clone());
            }
        }
        Err(format!("MCP tool '{tool_name}' has no active server connection"))
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
        if self.ui_tx.blocking_send(UiEvent::PermissionRequest {
            request_id: request_id.clone(),
            agent_id: self.agent_id.clone(),
            tool_name: request.tool_name.clone(),
            input: request.input.clone(),
            required_mode: format!("{:?}", request.required_mode),
        }).is_err() {
            eprintln!("[worker] TUI channel closed — event dropped");
        }

        // Block until the user responds, with a 120-second timeout to prevent hanging.
        // Uses a dedicated mini runtime to await with timeout since we're in spawn_blocking.
        let decision = {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .ok();
            match rt {
                Some(rt) => rt.block_on(async {
                    match tokio::time::timeout(std::time::Duration::from_secs(120), rx).await {
                        Ok(Ok(true)) => PermissionPromptDecision::Allow,
                        Ok(Ok(false)) => PermissionPromptDecision::Deny {
                            reason: "Denied by user".to_string(),
                        },
                        Ok(Err(_)) => PermissionPromptDecision::Deny {
                            reason: "Permission dialog was closed".to_string(),
                        },
                        Err(_) => PermissionPromptDecision::Deny {
                            reason: "Permission prompt timed out after 120 seconds".to_string(),
                        },
                    }
                }),
                None => PermissionPromptDecision::Deny {
                    reason: "Internal error: could not create runtime for permission check".to_string(),
                },
            }
        };
        decision
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

/// Parse structured diff info from Edit/Write tool JSON output.
fn parse_diff_info(output: &str) -> Option<DiffInfo> {
    let val: serde_json::Value = serde_json::from_str(output).ok()?;

    let file_path = val
        .get("filePath")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let patches = val.get("structuredPatch")?.as_array()?;
    let mut total_added = 0usize;
    let mut total_removed = 0usize;
    let mut hunks = Vec::new();

    for patch in patches {
        let old_start = patch
            .get("oldStart")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;
        let new_start = patch
            .get("newStart")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;

        let patch_lines = patch.get("lines")?.as_array()?;
        let mut diff_lines = Vec::new();

        for line in patch_lines {
            let s = line.as_str().unwrap_or("");
            if let Some(text) = s.strip_prefix('+') {
                diff_lines.push(DiffLine::Added(text.to_string()));
                total_added += 1;
            } else if let Some(text) = s.strip_prefix('-') {
                diff_lines.push(DiffLine::Removed(text.to_string()));
                total_removed += 1;
            } else if let Some(text) = s.strip_prefix(' ') {
                diff_lines.push(DiffLine::Context(text.to_string()));
            } else {
                diff_lines.push(DiffLine::Context(s.to_string()));
            }
        }

        hunks.push(DiffHunk {
            old_start,
            new_start,
            lines: diff_lines,
        });
    }

    Some(DiffInfo {
        file_path,
        added: total_added,
        removed: total_removed,
        hunks,
    })
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
