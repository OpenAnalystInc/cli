mod bash;
mod bootstrap;
mod compact;
mod config;
mod conversation;
mod file_ops;
mod hooks;
mod json;
mod mcp;
pub mod mcp_bridge;
mod mcp_client;
mod mcp_stdio;
mod oauth;
pub mod output_masking;
pub mod folder_trust;
pub mod ide_detect;
pub mod compression;
pub mod policy;
pub mod settings;
pub mod skills;
mod permissions;
mod prompt;
mod remote;
pub mod sandbox;
mod session;
pub mod tokenizer;
mod usage;

pub use lsp::{
    FileDiagnostics, LspContextEnrichment, LspError, LspManager, LspServerConfig,
    SymbolLocation, WorkspaceDiagnostics,
};
pub use bash::{execute_bash, BashCommandInput, BashCommandOutput};
pub use bootstrap::{BootstrapPhase, BootstrapPlan};
pub use compact::{
    compact_session, estimate_session_tokens, format_compact_summary,
    get_compact_continuation_message, should_compact, CompactionConfig, CompactionResult,
};
pub use config::{
    ConfigEntry, ConfigError, ConfigLoader, ConfigSource, McpManagedProxyServerConfig,
    McpConfigCollection, McpOAuthConfig, McpRemoteServerConfig, McpSdkServerConfig,
    McpServerConfig, McpStdioServerConfig, McpTransport, McpWebSocketServerConfig, OAuthConfig,
    ResolvedPermissionMode, RuntimeConfig, RuntimeFeatureConfig, RuntimeHookConfig,
    RuntimePluginConfig, ScopedMcpServerConfig, OPENANALYST_SETTINGS_SCHEMA_NAME,
};
pub use conversation::{
    ApiClient, ApiRequest, AssistantEvent, ConversationRuntime, RuntimeError, StaticToolExecutor,
    ToolError, ToolExecutor, TurnSummary,
};
pub use file_ops::{
    edit_file, glob_search, grep_search, read_file, write_file, EditFileOutput, GlobSearchOutput,
    GrepSearchInput, GrepSearchOutput, ReadFileOutput, StructuredPatchHunk, TextFilePayload,
    WriteFileOutput,
};
pub use hooks::{HookEvent, HookRunResult, HookRunner};
pub use mcp::{
    mcp_server_signature, mcp_tool_name, mcp_tool_prefix, normalize_name_for_mcp,
    scoped_mcp_config_hash, unwrap_ccr_proxy_url,
};
pub use mcp_client::{
    McpManagedProxyTransport, McpClientAuth, McpClientBootstrap, McpClientTransport,
    McpRemoteTransport, McpSdkTransport, McpStdioTransport,
};
pub use mcp_stdio::{
    spawn_mcp_stdio_process, JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse,
    ManagedMcpTool, McpInitializeClientInfo, McpInitializeParams, McpInitializeResult,
    McpInitializeServerInfo, McpListResourcesParams, McpListResourcesResult, McpListToolsParams,
    McpListToolsResult, McpReadResourceParams, McpReadResourceResult, McpResource,
    McpResourceContents, McpServerManager, McpServerManagerError, McpStdioProcess, McpTool,
    McpToolCallContent, McpToolCallParams, McpToolCallResult, UnsupportedMcpServer,
};
pub use oauth::{
    clear_oauth_credentials, code_challenge_s256, create_dotenv_template,
    credentials_config_home, credentials_path, generate_pkce_pair, generate_state,
    load_dotenv, load_dotenv_from, load_oauth_credentials, load_provider_oauth_token,
    loopback_redirect_uri, parse_oauth_callback_query, parse_oauth_callback_request_target,
    save_oauth_credentials, save_provider_oauth_token, start_oauth_callback_server,
    OAuthAuthorizationRequest, OAuthCallbackParams, OAuthRefreshRequest,
    OAuthTokenExchangeRequest, OAuthTokenSet, PkceChallengeMethod, PkceCodePair,
};
pub use permissions::{
    PermissionMode, PermissionOutcome, PermissionPolicy, PermissionPromptDecision,
    PermissionPrompter, PermissionRequest,
};
pub use prompt::{
    load_system_prompt, prepend_bullets, ContextFile, ProjectContext, PromptBuildError,
    SystemPromptBuilder, FRONTIER_MODEL_NAME, SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
};
pub use remote::{
    inherited_upstream_proxy_env, no_proxy_list, read_token, upstream_proxy_ws_url,
    RemoteSessionContext, UpstreamProxyBootstrap, UpstreamProxyState, DEFAULT_REMOTE_BASE_URL,
    DEFAULT_SESSION_TOKEN_PATH, DEFAULT_SYSTEM_CA_BUNDLE, NO_PROXY_HOSTS, UPSTREAM_PROXY_ENV_KEYS,
};
pub use session::{ContentBlock, ConversationMessage, MessageRole, Session, SessionError};
pub use usage::{
    format_usd, pricing_for_model, ModelPricing, TokenUsage, UsageCostEstimate, UsageTracker,
};

// ── New battle-tested modules ──
pub use compression::{
    estimate_text_tokens, estimate_conversation_tokens, needs_compression,
    truncate_tool_outputs, find_compress_split_point, extract_state_snapshot,
    CompressionResult, CompressionStatus, COMPRESSION_SYSTEM_PROMPT, VERIFICATION_PROMPT,
};
pub use folder_trust::{discover_trust, trust_folder, untrust_folder, TrustLevel, TrustInfo};
pub use ide_detect::{detect_ide, ide_context_string, Ide};
pub use output_masking::{mask_tool_output, likely_contains_secrets, MaskingStats};
pub use policy::{
    PolicyEngine, PolicyDecision, ApprovalMode, PolicyTier, PolicyRule,
    PolicyCheckResult, load_policy_toml, load_workspace_policy, load_user_policy,
};
pub use settings::{
    load_settings, load_user_settings, load_project_settings, save_project_settings,
    update_setting, load_mcp_json, save_mcp_server, remove_mcp_server,
    Settings, McpServerEntry, HookSettings, SkillSettings, PluginSettings,
};
pub use skills::{
    load_skills, execute_skill, format_skills_list, SkillDefinition, SkillScope,
};

#[cfg(test)]
pub(crate) fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
