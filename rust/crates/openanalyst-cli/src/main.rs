mod init;
mod input;
mod render;

use std::collections::BTreeSet;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use api::{
    resolve_startup_auth_source, AuthSource, ContentBlockDelta, InputContentBlock,
    InputMessage, MessageRequest, MessageResponse, OutputContentBlock,
    StreamEvent as ApiStreamEvent, ToolChoice, ToolDefinition, ToolResultContentBlock,
};

use commands::{
    handle_agents_slash_command, handle_plugins_slash_command, handle_skills_slash_command,
    render_slash_command_help, resume_supported_slash_commands, slash_command_specs, SlashCommand,
};
use compat_harness::{extract_manifest, UpstreamPaths};
use init::initialize_repo;
use plugins::{PluginManager, PluginManagerConfig};
use render::{MarkdownStreamState, Spinner, TerminalRenderer};
use runtime::{
    clear_oauth_credentials, generate_pkce_pair, generate_state, load_system_prompt,
    loopback_redirect_uri, save_provider_oauth_token, start_oauth_callback_server,
    ApiClient, ApiRequest, AssistantEvent, CompactionConfig, ConfigLoader,
    ConfigSource, ContentBlock, ConversationMessage, ConversationRuntime, MessageRole,
    OAuthAuthorizationRequest, OAuthConfig, OAuthTokenExchangeRequest,
    PermissionMode, PermissionPolicy, ProjectContext, RuntimeError,
    Session, TokenUsage, ToolError, ToolExecutor, UsageTracker,
};
use serde_json::json;
use tools::GlobalToolRegistry;

const DEFAULT_MODEL: &str = "openanalyst-beta";

/// Resolve the default model: env vars take priority, then detect from available API keys,
/// then fall back to built-in default.
/// Supports: OPENANALYST_MODEL, OPENANALYST_DEFAULT_MODEL, ANTHROPIC_DEFAULT_SONNET_MODEL
fn resolve_default_model() -> String {
    // 1. Explicit env var overrides always win
    if let Some(model) = env::var("OPENANALYST_MODEL")
        .or_else(|_| env::var("OPENANALYST_DEFAULT_MODEL"))
        .or_else(|_| env::var("ANTHROPIC_DEFAULT_SONNET_MODEL"))
        .ok()
        .filter(|v| !v.is_empty())
    {
        return model;
    }

    // 2. Auto-detect from available API keys — use the provider's default model
    //    Priority: OpenAnalyst > Anthropic > OpenAI > xAI > Gemini > OpenRouter > Bedrock
    if env::var("OPENANALYST_API_KEY").ok().filter(|v| !v.is_empty()).is_some()
        || env::var("OPENANALYST_AUTH_TOKEN").ok().filter(|v| !v.is_empty()).is_some()
    {
        // Free tier uses gpt-oss-120b; API credits use openanalyst-beta
        let mode = env::var("OPENANALYST_MODE").unwrap_or_else(|_| "api".to_string());
        return if mode == "free" {
            "gpt-oss-120b".to_string()
        } else {
            DEFAULT_MODEL.to_string()
        };
    }
    if env::var("ANTHROPIC_API_KEY").ok().filter(|v| !v.is_empty()).is_some() {
        return "claude-sonnet-4-6".to_string();
    }
    if env::var("OPENAI_API_KEY").ok().filter(|v| !v.is_empty()).is_some() {
        return "gpt-4o".to_string();
    }
    if env::var("XAI_API_KEY").ok().filter(|v| !v.is_empty()).is_some() {
        return "grok-3".to_string();
    }
    if env::var("GEMINI_API_KEY").ok().filter(|v| !v.is_empty()).is_some() {
        return "gemini-2.5-pro".to_string();
    }
    if env::var("OPENROUTER_API_KEY").ok().filter(|v| !v.is_empty()).is_some() {
        return "openrouter/auto".to_string();
    }
    if env::var("BEDROCK_API_KEY").ok().filter(|v| !v.is_empty()).is_some() {
        return "bedrock/claude".to_string();
    }

    // 3. Fall back to built-in default
    DEFAULT_MODEL.to_string()
}

fn max_tokens_for_model(model: &str) -> u32 {
    if model.contains("opus") {
        32_000
    } else {
        64_000
    }
}
const DEFAULT_DATE: &str = "2026-03-31";
const VERSION: &str = env!("OA_BUILD_VERSION");
const BUILD_TARGET: Option<&str> = option_env!("TARGET");
const GIT_SHA: Option<&str> = option_env!("GIT_SHA");
const INTERNAL_PROGRESS_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);

type AllowedToolSet = BTreeSet<String>;

/// Background update check — silently notifies if a newer version is available.
/// Does NOT auto-download; just creates a marker file that the TUI/REPL can read.
fn background_update_check() {
    const CURRENT_VERSION: &str = VERSION;
    const REPO: &str = "OpenAnalystInc/openanalyst-cli";

    // Only check once per day
    let config_dir = env::var("OPENANALYST_CONFIG_HOME")
        .or_else(|_| env::var("HOME").map(|h| format!("{h}/.openanalyst")))
        .or_else(|_| env::var("USERPROFILE").map(|h| format!("{h}\\.openanalyst")))
        .unwrap_or_default();
    if config_dir.is_empty() { return; }
    let marker_path = PathBuf::from(&config_dir).join(".update-check");
    if let Ok(meta) = fs::metadata(&marker_path) {
        if let Ok(modified) = meta.modified() {
            if modified.elapsed().unwrap_or_default() < Duration::from_secs(86400) {
                return; // Checked within 24h
            }
        }
    }

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return,
    };
    let latest = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build().ok()?;
        let resp = client
            .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
            .header("User-Agent", "openanalyst-cli")
            .send().await.ok()?;
        let body: serde_json::Value = resp.json().await.ok()?;
        body.get("tag_name")
            .and_then(|v| v.as_str())
            .map(|v| v.trim_start_matches('v').to_string())
    });

    // Write marker regardless (so we don't check again for 24h)
    let _ = fs::create_dir_all(&config_dir);
    let content = match &latest {
        Some(v) if v != CURRENT_VERSION => format!("{v}\n"),
        _ => String::new(),
    };
    let _ = fs::write(&marker_path, content);
}

/// Detect and load auth tokens from installed CLIs (Claude, Codex, Gemini).
/// Only sets env vars that are NOT already set — user's explicit config always wins.
///
/// Credential locations:
///   Claude CLI:  ~/.claude/.credentials.json  → claudeAiOauth.accessToken
///   Codex CLI:   ~/.codex/auth.json           → tokens.access_token
///   Gemini CLI:  ~/AppData/Roaming/gcloud/application_default_credentials.json (Windows)
///                ~/.config/gcloud/application_default_credentials.json (Unix)
fn load_external_cli_credentials() {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_default();
    if home.is_empty() { return; }
    let home = PathBuf::from(&home);

    // ── Claude CLI → ANTHROPIC_API_KEY ──
    if env::var("ANTHROPIC_API_KEY").ok().filter(|v| !v.is_empty()).is_none() {
        let claude_creds = home.join(".claude").join(".credentials.json");
        if let Ok(content) = fs::read_to_string(&claude_creds) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(token) = val
                    .get("claudeAiOauth")
                    .and_then(|o| o.get("accessToken"))
                    .and_then(|t| t.as_str())
                    .filter(|t| !t.is_empty())
                {
                    env::set_var("ANTHROPIC_API_KEY", token);
                }
            }
        }
    }

    // ── Codex CLI → OPENAI_API_KEY ──
    if env::var("OPENAI_API_KEY").ok().filter(|v| !v.is_empty()).is_none() {
        let codex_auth = home.join(".codex").join("auth.json");
        if let Ok(content) = fs::read_to_string(&codex_auth) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(token) = val
                    .get("tokens")
                    .and_then(|t| t.get("access_token"))
                    .and_then(|t| t.as_str())
                    .filter(|t| !t.is_empty())
                {
                    env::set_var("OPENAI_API_KEY", token);
                }
            }
        }
    }

    // ── Gemini CLI (gcloud ADC) → GEMINI_API_KEY ──
    if env::var("GEMINI_API_KEY").ok().filter(|v| !v.is_empty()).is_none() {
        // Windows: ~/AppData/Roaming/gcloud/  Unix: ~/.config/gcloud/
        let gcloud_dir = if cfg!(target_os = "windows") {
            home.join("AppData").join("Roaming").join("gcloud")
        } else {
            home.join(".config").join("gcloud")
        };
        let adc_path = gcloud_dir.join("application_default_credentials.json");
        if let Ok(content) = fs::read_to_string(&adc_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                // gcloud ADC has refresh_token + client_id + client_secret.
                // We exchange the refresh token for a fresh access token.
                let refresh_token = val.get("refresh_token").and_then(|v| v.as_str());
                let client_id = val.get("client_id").and_then(|v| v.as_str());
                let client_secret = val.get("client_secret").and_then(|v| v.as_str());
                if let (Some(rt), Some(cid), Some(cs)) = (refresh_token, client_id, client_secret) {
                    if let Some(access_token) = exchange_google_refresh_token(rt, cid, cs) {
                        env::set_var("GEMINI_API_KEY", &access_token);
                    }
                }
            }
        }
    }
}

/// Exchange a Google refresh token for a fresh access token.
fn exchange_google_refresh_token(refresh_token: &str, client_id: &str, client_secret: &str) -> Option<String> {
    let rt = tokio::runtime::Runtime::new().ok()?;
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        let resp = client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", client_id),
                ("client_secret", client_secret),
            ])
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() { return None; }
        let body: serde_json::Value = resp.json().await.ok()?;
        body.get("access_token").and_then(|v| v.as_str()).map(ToOwned::to_owned)
    })
}

/// Load ALL saved provider credentials from ~/.openanalyst/credentials.json
/// Sets env vars for EVERY saved provider so /model switching works across providers.
/// Env vars already set by the user take priority (not overwritten).
/// Ensure the directory containing the running binary is in the system PATH.
/// On Windows, updates the user-level PATH in the registry.
/// On Unix, appends to ~/.bashrc and ~/.zshrc if needed.
/// Only runs once — creates a marker file to avoid repeating.
fn ensure_path_registered() {
    let Ok(exe_path) = env::current_exe() else { return };
    let Some(bin_dir) = exe_path.parent() else { return };
    let bin_dir_str = bin_dir.to_string_lossy();

    // Check if already in PATH
    let path_var = env::var("PATH").unwrap_or_default();
    let separator = if cfg!(windows) { ';' } else { ':' };
    if path_var.split(separator).any(|p| {
        let p = p.trim_end_matches(['/', '\\']);
        let b = bin_dir_str.trim_end_matches(['/', '\\']);
        p.eq_ignore_ascii_case(b)
    }) {
        return; // Already in PATH
    }

    // Check marker file — only attempt once per install location
    let config_dir = runtime::credentials_config_home().unwrap_or_else(|_| PathBuf::from(".openanalyst"));
    let marker = config_dir.join(".path_registered");
    if marker.exists() { return; }

    #[cfg(target_os = "windows")]
    {
        // Windows: update user PATH via registry
        let result = std::process::Command::new("powershell")
            .args([
                "-NoProfile", "-Command",
                &format!(
                    "$p = [Environment]::GetEnvironmentVariable('Path','User'); \
                     if ($p -notlike '*{}*') {{ \
                       [Environment]::SetEnvironmentVariable('Path', $p + ';{}', 'User'); \
                       Write-Output 'added' \
                     }}",
                    bin_dir_str.replace('\'', "''"),
                    bin_dir_str.replace('\'', "''"),
                ),
            ])
            .output();

        if let Ok(output) = result {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("added") {
                eprintln!(
                    "  \x1b[38;5;45m\u{2713}\x1b[0m Added \x1b[1m{}\x1b[0m to your PATH.",
                    bin_dir_str
                );
                eprintln!("    \x1b[2mRestart your terminal for this to take effect.\x1b[0m");
                let _ = fs::write(&marker, "done");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Unix: append to shell profiles
        let home = env::var("HOME").unwrap_or_default();
        let export_line = format!("\n# OpenAnalyst CLI\nexport PATH=\"{}:$PATH\"\n", bin_dir_str);
        let mut added = false;

        for profile in &[".bashrc", ".zshrc", ".profile"] {
            let profile_path = PathBuf::from(&home).join(profile);
            if profile_path.exists() {
                let content = fs::read_to_string(&profile_path).unwrap_or_default();
                if !content.contains(&*bin_dir_str) {
                    let _ = fs::OpenOptions::new()
                        .append(true)
                        .open(&profile_path)
                        .and_then(|mut f| {
                            use std::io::Write;
                            f.write_all(export_line.as_bytes())
                        });
                    added = true;
                }
            }
        }

        if added {
            eprintln!(
                "  \x1b[38;5;45m\u{2713}\x1b[0m Added \x1b[1m{}\x1b[0m to your shell PATH.",
                bin_dir_str
            );
            eprintln!("    \x1b[2mRestart your terminal or run: source ~/.bashrc\x1b[0m");
            let _ = fs::write(&marker, "done");
        }
    }
}

fn load_saved_provider_credentials() {
    let config_dir = runtime::credentials_config_home().ok()
        .or_else(|| {
            env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok()
                .map(|h| PathBuf::from(h).join(".openanalyst"))
        });
    let Some(config_dir) = config_dir else { return };

    // ── Layer 1: credentials.json ──
    let creds_path = config_dir.join("credentials.json");
    if creds_path.exists() {
        if let Ok(content) = fs::read_to_string(&creds_path) {
            if let Ok(creds) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(providers) = creds.get("providers").and_then(|v| v.as_object()) {
                    for (_name, provider_data) in providers {
                        if let (Some(env_var), Some(api_key)) = (
                            provider_data.get("env_var").and_then(|v| v.as_str()),
                            provider_data.get("api_key").and_then(|v| v.as_str()),
                        ) {
                            if !api_key.is_empty() && env::var(env_var).ok().filter(|v| !v.is_empty()).is_none() {
                                env::set_var(env_var, api_key);
                            }
                        }
                    }
                }
                // Load OpenAnalyst routing mode (free or api)
                if let Some(mode) = creds.get("openanalyst_mode").and_then(|v| v.as_str()) {
                    if env::var("OPENANALYST_MODE").ok().filter(|v| !v.is_empty()).is_none() {
                        env::set_var("OPENANALYST_MODE", mode);
                    }
                }
            }
        }
    }

    // ── Layer 2: SQLite fallback — picks up anything credentials.json missed ──
    if let Ok(db) = orchestrator::knowledge::LearningDb::open() {
        if let Ok(creds) = db.load_all_credentials() {
            for (_provider, env_var, api_key) in creds {
                if !api_key.is_empty() && env::var(&env_var).ok().filter(|v| !v.is_empty()).is_none() {
                    env::set_var(&env_var, &api_key);
                }
            }
        }
    }
}

fn main() {
    // 1. Create ~/.openanalyst/.env template on first run (if missing)
    let _ = runtime::create_dotenv_template();
    // 2. Load project-level .openanalyst/.env first (takes priority)
    let _ = runtime::load_dotenv_from(&std::path::Path::new(".openanalyst").join(".env"));
    // 3. Load global ~/.openanalyst/.env (won't override project-level vars)
    let _ = runtime::load_dotenv();
    // 4. Load saved credentials from ~/.openanalyst/credentials.json into env vars
    load_saved_provider_credentials();
    // 4. Detect auth from installed CLIs (Claude, Codex, Gemini)
    load_external_cli_credentials();
    // 5. Ensure the CLI binary directory is in PATH
    ensure_path_registered();
    // Prune old/empty sessions in the background (non-blocking)
    std::thread::spawn(cleanup_old_sessions);
    // 6. Check for updates in background (non-blocking, silent)
    std::thread::spawn(background_update_check);

    if let Err(error) = run() {
        let msg = error.to_string();
        if msg.contains("credentials") || msg.contains("API_KEY") || msg.contains("missing") {
            eprintln!();
            eprintln!("  \x1b[38;5;208mNo API credentials found.\x1b[0m");
            eprintln!();
            eprintln!("  Get started with one of:");
            eprintln!("    \x1b[1mopenanalyst login\x1b[0m          Interactive login (browser or API key)");
            eprintln!("    \x1b[1medit ~/.openanalyst/.env\x1b[0m   Add your API keys to the config file");
            eprintln!();
            eprintln!("  \x1b[2mOnce logged in, just run \x1b[0m\x1b[1mopenanalyst\x1b[0m\x1b[2m to start a new session.\x1b[0m");
            eprintln!("  \x1b[2mYour credentials are saved and remembered across sessions.\x1b[0m");
            eprintln!();
        } else {
            eprintln!(
                "error: {error}\n\nRun `openanalyst --help` for usage."
            );
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args)? {
        CliAction::DumpManifests => dump_manifests(),
        CliAction::BootstrapPlan => print_bootstrap_plan(),
        CliAction::Agents { args } => LiveCli::print_agents(args.as_deref())?,
        CliAction::Skills { args } => LiveCli::print_skills(args.as_deref())?,
        CliAction::PrintSystemPrompt { cwd, date } => print_system_prompt(cwd, date),
        CliAction::Version => print_version(),
        CliAction::ResumeSession {
            session_path,
            commands,
        } => resume_session(&session_path, &commands),
        CliAction::Prompt {
            prompt,
            model,
            output_format,
            allowed_tools,
            permission_mode,
        } => LiveCli::new(model, true, allowed_tools, permission_mode)?
            .run_turn_with_output(&prompt, output_format)?,
        CliAction::Login => {
            if let Some(model) = run_login()? {
                run_tui(model, None, PermissionMode::WorkspaceWrite)?;
            }
        }
        CliAction::Logout => run_logout()?,
        CliAction::WhoAmI => run_whoami()?,
        CliAction::Update => run_update()?,
        CliAction::Uninstall => run_uninstall()?,
        CliAction::Init => run_init()?,
        CliAction::Agent {
            task,
            model,
            max_turns,
            permission_mode,
            allowed_tools,
            verbose,
        } => run_agent(task, model, max_turns, permission_mode, allowed_tools, verbose)?,
        CliAction::Repl {
            model,
            allowed_tools,
            permission_mode,
            use_tui,
        } => {
            if use_tui {
                run_tui(model, allowed_tools, permission_mode)?;
            } else {
                // Legacy REPL mode (--no-tui)
                run_repl(model, allowed_tools, permission_mode)?;
            }
        }
        CliAction::Help => print_help(),
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliAction {
    DumpManifests,
    BootstrapPlan,
    Agents {
        args: Option<String>,
    },
    Skills {
        args: Option<String>,
    },
    PrintSystemPrompt {
        cwd: PathBuf,
        date: String,
    },
    Version,
    ResumeSession {
        session_path: PathBuf,
        commands: Vec<String>,
    },
    Prompt {
        prompt: String,
        model: String,
        output_format: CliOutputFormat,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    },
    Login,
    Logout,
    WhoAmI,
    Init,
    Update,
    Uninstall,
    Repl {
        model: String,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
        use_tui: bool,
    },
    Agent {
        task: String,
        model: String,
        max_turns: usize,
        permission_mode: PermissionMode,
        allowed_tools: Option<AllowedToolSet>,
        verbose: bool,
    },
    // prompt-mode formatting is only supported for non-interactive runs
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliOutputFormat {
    Text,
    Json,
}

impl CliOutputFormat {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "unsupported value for --output-format: {other} (expected text or json)"
            )),
        }
    }
}

#[allow(clippy::too_many_lines)]
fn parse_args(args: &[String]) -> Result<CliAction, String> {
    let mut model = resolve_default_model();
    let mut output_format = CliOutputFormat::Text;
    let mut permission_mode = default_permission_mode();
    let mut wants_version = false;
    let mut wants_no_tui = false;
    let mut allowed_tool_values = Vec::new();
    let mut rest = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--version" | "-V" => {
                wants_version = true;
                index += 1;
            }
            "--no-tui" => {
                wants_no_tui = true;
                index += 1;
            }
            "--model" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --model".to_string())?;
                model = resolve_model_alias(value).to_string();
                index += 2;
            }
            flag if flag.starts_with("--model=") => {
                model = resolve_model_alias(&flag[8..]).to_string();
                index += 1;
            }
            "--output-format" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --output-format".to_string())?;
                output_format = CliOutputFormat::parse(value)?;
                index += 2;
            }
            "--permission-mode" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --permission-mode".to_string())?;
                permission_mode = parse_permission_mode_arg(value)?;
                index += 2;
            }
            flag if flag.starts_with("--output-format=") => {
                output_format = CliOutputFormat::parse(&flag[16..])?;
                index += 1;
            }
            flag if flag.starts_with("--permission-mode=") => {
                permission_mode = parse_permission_mode_arg(&flag[18..])?;
                index += 1;
            }
            "--dangerously-skip-permissions" => {
                permission_mode = PermissionMode::DangerFullAccess;
                index += 1;
            }
            "-p" => {
                // OpenAnalyst compat: -p "prompt" = one-shot prompt
                let prompt = args[index + 1..].join(" ");
                if prompt.trim().is_empty() {
                    return Err("-p requires a prompt string".to_string());
                }
                return Ok(CliAction::Prompt {
                    prompt,
                    model: resolve_model_alias(&model).to_string(),
                    output_format,
                    allowed_tools: normalize_allowed_tools(&allowed_tool_values)?,
                    permission_mode,
                });
            }
            "--print" => {
                // OpenAnalyst compat: --print makes output non-interactive
                output_format = CliOutputFormat::Text;
                index += 1;
            }
            "--allowedTools" | "--allowed-tools" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --allowedTools".to_string())?;
                allowed_tool_values.push(value.clone());
                index += 2;
            }
            flag if flag.starts_with("--allowedTools=") => {
                allowed_tool_values.push(flag[15..].to_string());
                index += 1;
            }
            flag if flag.starts_with("--allowed-tools=") => {
                allowed_tool_values.push(flag[16..].to_string());
                index += 1;
            }
            other => {
                rest.push(other.to_string());
                index += 1;
            }
        }
    }

    if wants_version {
        return Ok(CliAction::Version);
    }

    let allowed_tools = normalize_allowed_tools(&allowed_tool_values)?;

    if rest.is_empty() {
        return Ok(CliAction::Repl {
            model,
            allowed_tools,
            permission_mode,
            use_tui: !wants_no_tui,
        });
    }
    if matches!(rest.first().map(String::as_str), Some("--help" | "-h")) {
        return Ok(CliAction::Help);
    }
    if rest.first().map(String::as_str) == Some("--resume") {
        return parse_resume_args(&rest[1..]);
    }

    match rest[0].as_str() {
        "dump-manifests" => Ok(CliAction::DumpManifests),
        "bootstrap-plan" => Ok(CliAction::BootstrapPlan),
        "agents" => Ok(CliAction::Agents {
            args: join_optional_args(&rest[1..]),
        }),
        "skills" => Ok(CliAction::Skills {
            args: join_optional_args(&rest[1..]),
        }),
        "system-prompt" => parse_system_prompt_args(&rest[1..]),
        "agent" => parse_agent_args(&rest[1..], &model, permission_mode, &allowed_tool_values),
        "login" => Ok(CliAction::Login),
        "logout" => Ok(CliAction::Logout),
        "whoami" => Ok(CliAction::WhoAmI),
        "init" => Ok(CliAction::Init),
        "update" => Ok(CliAction::Update),
        "uninstall" => Ok(CliAction::Uninstall),
        "prompt" => {
            let prompt = rest[1..].join(" ");
            if prompt.trim().is_empty() {
                return Err("prompt subcommand requires a prompt string".to_string());
            }
            Ok(CliAction::Prompt {
                prompt,
                model,
                output_format,
                allowed_tools,
                permission_mode,
            })
        }
        other if other.starts_with('/') => parse_direct_slash_cli_action(&rest),
        _other => Ok(CliAction::Prompt {
            prompt: rest.join(" "),
            model,
            output_format,
            allowed_tools,
            permission_mode,
        }),
    }
}

fn join_optional_args(args: &[String]) -> Option<String> {
    let joined = args.join(" ");
    let trimmed = joined.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn parse_direct_slash_cli_action(rest: &[String]) -> Result<CliAction, String> {
    let raw = rest.join(" ");
    match SlashCommand::parse(&raw) {
        Some(SlashCommand::Help) => Ok(CliAction::Help),
        Some(SlashCommand::Agents { args }) => Ok(CliAction::Agents { args }),
        Some(SlashCommand::Skills { args }) => Ok(CliAction::Skills { args }),
        Some(command) => Err(format!(
            "unsupported direct slash command outside the REPL: {command_name}",
            command_name = match command {
                SlashCommand::Unknown(name) => format!("/{name}"),
                _ => rest[0].clone(),
            }
        )),
        None => Err(format!("unknown subcommand: {}", rest[0])),
    }
}

fn resolve_model_alias(model: &str) -> &str {
    match model {
        "opus" => "claude-opus-4-6",
        "sonnet" => "claude-sonnet-4-6",
        "haiku" => "claude-haiku-4-5-20251213",
        _ => model,
    }
}

fn normalize_allowed_tools(values: &[String]) -> Result<Option<AllowedToolSet>, String> {
    current_tool_registry()?.normalize_allowed_tools(values)
}

fn current_tool_registry() -> Result<GlobalToolRegistry, String> {
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load().map_err(|error| error.to_string())?;
    let plugin_manager = build_plugin_manager(&cwd, &loader, &runtime_config);
    let plugin_tools = plugin_manager
        .aggregated_tools()
        .map_err(|error| error.to_string())?;
    GlobalToolRegistry::with_plugin_tools(plugin_tools)
}

fn parse_permission_mode_arg(value: &str) -> Result<PermissionMode, String> {
    normalize_permission_mode(value)
        .ok_or_else(|| {
            format!(
                "unsupported permission mode '{value}'. Use read-only, workspace-write, or danger-full-access."
            )
        })
        .map(permission_mode_from_label)
}

fn permission_mode_from_label(mode: &str) -> PermissionMode {
    match mode {
        "read-only" => PermissionMode::ReadOnly,
        "workspace-write" => PermissionMode::WorkspaceWrite,
        "danger-full-access" => PermissionMode::DangerFullAccess,
        other => panic!("unsupported permission mode label: {other}"),
    }
}

fn parse_agent_args(
    args: &[String],
    model: &str,
    permission_mode: PermissionMode,
    allowed_tool_values: &[String],
) -> Result<CliAction, String> {
    let mut index = 0;
    let mut max_turns: usize = 30;
    let mut verbose = false;
    let mut task_parts = Vec::new();
    let mut sub_model = model.to_string();

    // First arg should be "run" (only subcommand for now)
    if args.first().map(String::as_str) != Some("run") {
        if args.is_empty() {
            return Err("usage: openanalyst agent run <task>\n  Subcommands: run".to_string());
        }
        return Err(format!(
            "unknown agent subcommand '{}'. Use: openanalyst agent run <task>",
            args[0]
        ));
    }
    index += 1; // skip "run"

    while index < args.len() {
        match args[index].as_str() {
            "--max-turns" => {
                let val = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --max-turns".to_string())?;
                max_turns = val
                    .parse()
                    .map_err(|_| format!("invalid --max-turns value: {val}"))?;
                index += 2;
            }
            flag if flag.starts_with("--max-turns=") => {
                let val = &flag[12..];
                max_turns = val
                    .parse()
                    .map_err(|_| format!("invalid --max-turns value: {val}"))?;
                index += 1;
            }
            "--model" => {
                let val = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --model".to_string())?;
                sub_model = resolve_model_alias(val).to_string();
                index += 2;
            }
            flag if flag.starts_with("--model=") => {
                sub_model = resolve_model_alias(&flag[8..]).to_string();
                index += 1;
            }
            "--verbose" | "-v" => {
                verbose = true;
                index += 1;
            }
            other => {
                task_parts.push(other.to_string());
                index += 1;
            }
        }
    }

    let task = task_parts.join(" ");
    if task.trim().is_empty() {
        return Err("usage: openanalyst agent run <task>".to_string());
    }

    let allowed_tools = normalize_allowed_tools(allowed_tool_values)?;

    Ok(CliAction::Agent {
        task,
        model: sub_model,
        max_turns,
        permission_mode,
        allowed_tools,
        verbose,
    })
}

fn run_agent(
    task: String,
    model: String,
    max_turns: usize,
    permission_mode: PermissionMode,
    allowed_tools: Option<AllowedToolSet>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use openanalyst_agent::{AgentConfig, AgentRunner};

    let config = AgentConfig {
        model: model.clone(),
        max_turns,
        permission_mode,
        allowed_tools,
        cwd: env::current_dir()?,
        verbose,
        system_context: None,
    };

    eprintln!();
    eprintln!(
        "  \x1b[38;5;45m\x1b[1mOpenAnalyst Agent\x1b[0m \x1b[2m-- autonomous mode\x1b[0m"
    );
    eprintln!("  \x1b[2mModel\x1b[0m       {model}");
    eprintln!("  \x1b[2mMax turns\x1b[0m   {max_turns}");
    eprintln!();

    let runner = AgentRunner::new(config);
    let result = runner.run(&task)?;

    // Print final summary
    eprintln!();
    eprintln!("  \x1b[38;5;45m\x1b[1mAgent complete\x1b[0m");
    eprintln!(
        "  \x1b[2mTurns\x1b[0m       {}",
        result.turns
    );
    eprintln!(
        "  \x1b[2mTokens\x1b[0m      {} in / {} out",
        result.input_tokens, result.output_tokens
    );
    eprintln!(
        "  \x1b[2mDuration\x1b[0m    {:.1}s",
        result.duration_secs
    );
    eprintln!(
        "  \x1b[2mTool calls\x1b[0m  {}",
        result.tool_calls.len()
    );
    eprintln!();

    if !result.final_text.is_empty() {
        println!("{}", result.final_text);
    }

    Ok(())
}

fn default_permission_mode() -> PermissionMode {
    env::var("OPENANALYST_PERMISSION_MODE")
        .ok()
        .as_deref()
        .and_then(normalize_permission_mode)
        .map_or(PermissionMode::DangerFullAccess, permission_mode_from_label)
}

fn filter_tool_specs(
    tool_registry: &GlobalToolRegistry,
    allowed_tools: Option<&AllowedToolSet>,
) -> Vec<ToolDefinition> {
    tool_registry.definitions(allowed_tools)
}

fn parse_system_prompt_args(args: &[String]) -> Result<CliAction, String> {
    let mut cwd = env::current_dir().map_err(|error| error.to_string())?;
    let mut date = DEFAULT_DATE.to_string();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--cwd" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --cwd".to_string())?;
                cwd = PathBuf::from(value);
                index += 2;
            }
            "--date" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing value for --date".to_string())?;
                date.clone_from(value);
                index += 2;
            }
            other => return Err(format!("unknown system-prompt option: {other}")),
        }
    }

    Ok(CliAction::PrintSystemPrompt { cwd, date })
}

fn parse_resume_args(args: &[String]) -> Result<CliAction, String> {
    let session_path = args
        .first()
        .ok_or_else(|| "missing session path for --resume".to_string())
        .map(PathBuf::from)?;
    let commands = args[1..].to_vec();
    if commands
        .iter()
        .any(|command| !command.trim_start().starts_with('/'))
    {
        return Err("--resume trailing arguments must be slash commands".to_string());
    }
    Ok(CliAction::ResumeSession {
        session_path,
        commands,
    })
}

fn dump_manifests() {
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = UpstreamPaths::from_workspace_dir(&workspace_dir);
    match extract_manifest(&paths) {
        Ok(manifest) => {
            println!("commands: {}", manifest.commands.entries().len());
            println!("tools: {}", manifest.tools.entries().len());
            println!("bootstrap phases: {}", manifest.bootstrap.phases().len());
        }
        Err(error) => {
            eprintln!("failed to extract manifests: {error}");
            std::process::exit(1);
        }
    }
}

fn print_bootstrap_plan() {
    for phase in runtime::BootstrapPlan::openanalyst_default().phases() {
        println!("- {phase:?}");
    }
}

// ── Provider definitions for interactive login ──

struct ProviderOption {
    name: &'static str,
    description: &'static str,
    env_var: &'static str,
    test_url: &'static str,
    test_header: &'static str, // "bearer" or "x-api-key"
    dashboard_url: &'static str, // URL to open in browser for key generation
    models_url: &'static str,    // URL to fetch available models after login
    /// OAuth config for providers that support browser-based login.
    /// When set, `openanalyst login` will use the OAuth flow instead of API key paste.
    oauth: Option<ProviderOAuthMeta>,
}

#[derive(Clone)]
struct ProviderOAuthMeta {
    /// Environment variable override for OAuth client ID (optional — if set, takes priority)
    client_id_env: &'static str,
    /// Built-in default OAuth client ID — used when env var is not set.
    /// Registered with the provider's developer portal for OpenAnalyst CLI.
    default_client_id: &'static str,
    /// Public client secret for installed-app OAuth flows (e.g., Google).
    /// Not truly secret — embedded in open-source CLIs per provider convention.
    default_client_secret: &'static str,
    authorize_url: &'static str,
    token_url: &'static str,
    scopes: &'static [&'static str],
    /// The env var to store the resulting API key / bearer token
    token_env_var: &'static str,
    /// Extra query parameters for the authorization URL (e.g., OpenAI's special params)
    extra_authorize_params: &'static [(&'static str, &'static str)],
}

const LOGIN_PROVIDERS: &[ProviderOption] = &[
    ProviderOption {
        name: "OpenAnalyst",
        description: "gpt-oss-120b free model or API key with credits",
        env_var: "OPENANALYST_AUTH_TOKEN",
        test_url: "https://api.openanalyst.com/api/health",
        test_header: "bearer",
        dashboard_url: "https://10x.in/dashboard",
        models_url: "",
        // No OAuth — OpenAnalyst uses free model (auto key) or API key only
        // OAuth reserved for future browser-based login
        oauth: None,
    },
    ProviderOption {
        name: "Anthropic / Claude",
        description: "opus, sonnet, haiku — API key",
        env_var: "ANTHROPIC_API_KEY",
        test_url: "https://api.anthropic.com/v1/messages",
        test_header: "x-api-key",
        dashboard_url: "https://console.anthropic.com/settings/keys",
        models_url: "https://api.anthropic.com/v1/models",
        // OAuth disabled — Anthropic restricts OAuth tokens to Claude Code only.
        // Users must use API keys from console.anthropic.com
        oauth: None,
    },
    ProviderOption {
        name: "OpenAI / Codex",
        description: "gpt-4o, o3, codex-mini — API key",
        env_var: "OPENAI_API_KEY",
        test_url: "https://api.openai.com/v1/models",
        test_header: "bearer",
        dashboard_url: "https://platform.openai.com/api-keys",
        models_url: "https://api.openai.com/v1/models",
        // OAuth disabled — OpenAI restricts OAuth tokens to Codex CLI only.
        // Users must use API keys from platform.openai.com
        oauth: None,
    },
    ProviderOption {
        name: "Google Gemini",
        description: "gemini-2.5-pro, flash — OAuth or API key",
        env_var: "GEMINI_API_KEY",
        test_url: "https://generativelanguage.googleapis.com/v1beta/openai/models",
        test_header: "bearer",
        dashboard_url: "https://aistudio.google.com/apikey",
        models_url: "https://generativelanguage.googleapis.com/v1beta/openai/models",
        oauth: Some(ProviderOAuthMeta {
            client_id_env: "OPENANALYST_GOOGLE_CLIENT_ID",
            // Public client ID from Gemini CLI (installed-app flow — not secret)
            default_client_id: "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com",
            // Public client secret for Google installed-app OAuth (not truly secret, per Google's convention)
            default_client_secret: "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",
            authorize_url: "https://accounts.google.com/o/oauth2/v2/auth",
            token_url: "https://oauth2.googleapis.com/token",
            scopes: &[
                "https://www.googleapis.com/auth/cloud-platform",
                "https://www.googleapis.com/auth/userinfo.email",
                "https://www.googleapis.com/auth/userinfo.profile",
            ],
            token_env_var: "GEMINI_API_KEY",
            extra_authorize_params: &[
                ("access_type", "offline"),
                ("prompt", "consent"),
            ],
        }),
    },
    ProviderOption {
        name: "xAI / Grok",
        description: "grok-3, grok-mini",
        env_var: "XAI_API_KEY",
        test_url: "https://api.x.ai/v1/models",
        test_header: "bearer",
        dashboard_url: "https://console.x.ai",
        models_url: "https://api.x.ai/v1/models",
        oauth: None,
    },
    ProviderOption {
        name: "OpenRouter",
        description: "350+ models via one key",
        env_var: "OPENROUTER_API_KEY",
        test_url: "https://openrouter.ai/api/v1/models",
        test_header: "bearer",
        dashboard_url: "https://openrouter.ai/keys",
        models_url: "https://openrouter.ai/api/v1/models",
        oauth: None,
    },
    ProviderOption {
        name: "Amazon Bedrock",
        description: "AWS Bedrock gateway",
        env_var: "BEDROCK_API_KEY",
        test_url: "",
        test_header: "bearer",
        dashboard_url: "",
        models_url: "",
        oauth: None,
    },
    ProviderOption {
        name: "Stability AI",
        description: "Stable Diffusion (/image)",
        env_var: "STABILITY_API_KEY",
        test_url: "",
        test_header: "bearer",
        dashboard_url: "https://platform.stability.ai/account/keys",
        models_url: "",
        oauth: None,
    },
];

/// Open a URL in the default browser (cross-platform).
fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "windows")]
    { std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn().is_ok() }
    #[cfg(target_os = "macos")]
    { std::process::Command::new("open").arg(url).spawn().is_ok() }
    #[cfg(target_os = "linux")]
    { std::process::Command::new("xdg-open").arg(url).spawn().is_ok() }
}

fn run_login() -> Result<Option<String>, Box<dyn std::error::Error>> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
        execute,
        terminal,
    };

    let mut stdout = io::stdout();

    // ── Show header ──
    println!();
    println!("  \x1b[38;5;45m\x1b[1mOpenAnalyst CLI \x1b[0m\x1b[2m— Login\x1b[0m");
    println!("  \x1b[2mSelect your LLM provider to authenticate:\x1b[0m");
    println!();

    // ── Interactive arrow-key selector ──
    let mut selected: usize = 0;
    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    // Drain any buffered key events (e.g. the Enter from typing the command)
    while event::poll(Duration::from_millis(50))? {
        let _ = event::read()?;
    }

    let draw_menu = |sel: usize, out: &mut io::Stdout| -> io::Result<()> {
        for (i, provider) in LOGIN_PROVIDERS.iter().enumerate() {
            let prefix = if i == sel { "\x1b[38;5;45m  > " } else { "    " };
            let name_color = if i == sel { "\x1b[1m" } else { "\x1b[2m" };
            let desc = if i == sel {
                format!("  \x1b[2m{}\x1b[0m", provider.description)
            } else {
                String::new()
            };
            let _ = write!(
                out,
                "\r\x1b[2K{prefix}{name_color}{}{desc}\x1b[0m\r\n",
                provider.name
            );
        }
        out.flush()
    };

    draw_menu(selected, &mut stdout)?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, modifiers, kind, .. }) = event::read()? {
                // Only handle Press events — Windows emits Press+Release, causing double-step
                if kind != event::KeyEventKind::Press { continue; }
                match code {
                    KeyCode::Up if selected > 0 => {
                        selected -= 1;
                        let _ = write!(stdout, "\x1b[{}A", LOGIN_PROVIDERS.len());
                        draw_menu(selected, &mut stdout)?;
                    }
                    KeyCode::Down if selected < LOGIN_PROVIDERS.len() - 1 => {
                        selected += 1;
                        let _ = write!(stdout, "\x1b[{}A", LOGIN_PROVIDERS.len());
                        draw_menu(selected, &mut stdout)?;
                    }
                    KeyCode::Enter => break,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        terminal::disable_raw_mode()?;
                        execute!(stdout, cursor::Show)?;
                        println!();
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        terminal::disable_raw_mode()?;
                        execute!(stdout, cursor::Show)?;
                        println!();
                        return Ok(None);
                    }
                    _ => {}
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    execute!(stdout, cursor::Show)?;

    let provider = &LOGIN_PROVIDERS[selected];
    println!();
    println!(
        "  \x1b[38;5;45m\x1b[1m{}\x1b[0m",
        provider.name
    );
    println!();

    // ── OpenAnalyst: free model vs API key (no OAuth) ──
    // ── Other providers with OAuth: browser login vs API key ──
    let is_openanalyst = provider.name == "OpenAnalyst";
    if is_openanalyst || provider.oauth.is_some() {
        let auth_methods = if is_openanalyst {
            [
                ("Use free model", "gpt-oss-120b — no credits needed"),
                ("Use API key", "OpenAnalyst API with credits"),
            ]
        } else {
            [
                ("Login with browser", "sign in with your account (recommended)"),
                ("Use API key", "paste an API key manually"),
            ]
        };

        println!("  \x1b[2mHow would you like to authenticate?\x1b[0m");
        println!();

        // Mini selector for auth method
        let mut method_sel: usize = 0;
        {
            use crossterm::{cursor, event::{self, Event, KeyCode, KeyEvent, KeyModifiers}, execute, terminal};
            terminal::enable_raw_mode()?;
            execute!(stdout, cursor::Hide)?;

            // Drain any buffered key events
            while event::poll(Duration::from_millis(50))? {
                let _ = event::read()?;
            }

            let draw = |sel: usize, out: &mut io::Stdout| -> io::Result<()> {
                for (i, (name, desc)) in auth_methods.iter().enumerate() {
                    let prefix = if i == sel { "\x1b[38;5;45m  > " } else { "    " };
                    let style = if i == sel { "\x1b[1m" } else { "\x1b[2m" };
                    let hint = if i == sel { format!("  \x1b[2m{desc}\x1b[0m") } else { String::new() };
                    let _ = write!(out, "\r\x1b[2K{prefix}{style}{name}{hint}\x1b[0m\r\n");
                }
                out.flush()
            };
            draw(method_sel, &mut stdout)?;

            loop {
                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(KeyEvent { code, modifiers, kind, .. }) = event::read()? {
                        if kind != event::KeyEventKind::Press { continue; }
                        match code {
                            KeyCode::Up if method_sel > 0 => {
                                method_sel -= 1;
                                let _ = write!(stdout, "\x1b[{}A", auth_methods.len());
                                draw(method_sel, &mut stdout)?;
                            }
                            KeyCode::Down if method_sel < auth_methods.len() - 1 => {
                                method_sel += 1;
                                let _ = write!(stdout, "\x1b[{}A", auth_methods.len());
                                draw(method_sel, &mut stdout)?;
                            }
                            KeyCode::Enter => break,
                            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                                terminal::disable_raw_mode()?;
                                execute!(stdout, cursor::Show)?;
                                println!();
                                return Ok(None);
                            }
                            KeyCode::Esc => {
                                terminal::disable_raw_mode()?;
                                execute!(stdout, cursor::Show)?;
                                println!();
                                return Ok(None);
                            }
                            _ => {}
                        }
                    }
                }
            }

            terminal::disable_raw_mode()?;
            execute!(stdout, cursor::Show)?;
        }
        println!();

        if method_sel == 0 && is_openanalyst {
            // OpenAnalyst free model — auto key, gpt-oss-120b
            save_openanalyst_mode("free");
            run_openanalyst_free_login(provider)?;
        } else if method_sel == 0 && provider.oauth.is_some() {
            // Other providers: OAuth browser login
            let oauth_meta = provider.oauth.as_ref().unwrap();
            let client_id = env::var(oauth_meta.client_id_env)
                .ok()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| oauth_meta.default_client_id.to_string());
            run_oauth_login(provider, oauth_meta, &client_id)?;
        } else {
            // API key path
            if is_openanalyst {
                save_openanalyst_mode("api");
            }
            run_apikey_login(provider)?;
        }
    } else {
        // Non-OAuth providers: straight to API key
        run_apikey_login(provider)?;
    }

    println!();

    // Ask user whether to launch the TUI immediately
    print!("  Launch OpenAnalyst now? [\x1b[1mY\x1b[0m/n] ");
    io::stdout().flush()?;

    {
        use crossterm::{cursor, event::{self, Event, KeyCode, KeyEvent}, execute, terminal};
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();

        loop {
            if event::poll(std::time::Duration::from_secs(30))? {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    match code {
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            terminal::disable_raw_mode()?;
                            execute!(stdout, cursor::Show)?;
                            println!();
                            let model = resolve_default_model();
                            return Ok(Some(model));
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            terminal::disable_raw_mode()?;
                            execute!(stdout, cursor::Show)?;
                            println!();
                            println!();
                            println!("  Run \x1b[1mopenanalyst\x1b[0m to start, or \x1b[1mopenanalyst login\x1b[0m to add another provider.");
                            println!("  Use \x1b[1mopenanalyst whoami\x1b[0m to see all logged-in providers.");
                            println!();
                            return Ok(None);
                        }
                        _ => {}
                    }
                }
            } else {
                // Timeout — default to launching TUI
                terminal::disable_raw_mode()?;
                execute!(stdout, cursor::Show)?;
                println!();
                let model = resolve_default_model();
                return Ok(Some(model));
            }
        }
    }
}

/// OAuth browser login: start callback server → open browser → wait for redirect → exchange code → save token.
fn run_oauth_login(
    provider: &ProviderOption,
    oauth_meta: &ProviderOAuthMeta,
    client_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Start local callback server
    let (port, callback_rx) = start_oauth_callback_server()?;
    let redirect_uri = loopback_redirect_uri(port);

    // 2. Generate PKCE + state
    let pkce = generate_pkce_pair()?;
    let state = generate_state()?;

    // 3. Build OAuth config
    let oauth_config = OAuthConfig {
        client_id: client_id.to_string(),
        authorize_url: oauth_meta.authorize_url.to_string(),
        token_url: oauth_meta.token_url.to_string(),
        callback_port: Some(port),
        manual_redirect_url: None,
        scopes: oauth_meta.scopes.iter().map(|s| s.to_string()).collect(),
    };

    // 4. Build authorization URL (with any provider-specific extra params)
    let auth_request = OAuthAuthorizationRequest::from_config(
        &oauth_config,
        &redirect_uri,
        &state,
        &pkce,
    );
    let mut auth_url = auth_request.build_url();
    for (key, value) in oauth_meta.extra_authorize_params {
        auth_url.push_str(&format!("&{key}={value}"));
    }

    // 5. Open browser
    println!("  \x1b[1mStep 1\x1b[0m  Opening {} login in your browser...", provider.name);
    println!("  \x1b[2m{}\x1b[0m", &auth_url[..auth_url.find('?').unwrap_or(auth_url.len())]);
    println!();

    if !open_browser(&auth_url) {
        println!("  \x1b[38;5;208mCould not open browser automatically.\x1b[0m");
        println!("  \x1b[2mOpen this URL manually:\x1b[0m");
        println!("  {auth_url}");
        println!();
    }

    // 6. Wait for callback (with timeout)
    print!("  \x1b[1mStep 2\x1b[0m  Waiting for authentication... ");
    io::stdout().flush()?;

    let callback_result = callback_rx.recv_timeout(Duration::from_secs(120));

    match callback_result {
        Ok(Ok(params)) => {
            // Check for errors
            if let Some(error) = &params.error {
                let desc = params.error_description.as_deref().unwrap_or("unknown error");
                println!("\x1b[38;5;196m\u{2717} Failed\x1b[0m");
                println!("  \x1b[38;5;196m{error}: {desc}\x1b[0m");
                return Ok(());
            }

            // Validate state
            let callback_state = params.state.as_deref().unwrap_or("");
            if callback_state != state {
                println!("\x1b[38;5;196m\u{2717} State mismatch (possible CSRF)\x1b[0m");
                return Ok(());
            }

            let Some(code) = params.code else {
                println!("\x1b[38;5;196m\u{2717} No authorization code received\x1b[0m");
                return Ok(());
            };

            println!("\x1b[38;5;46m\u{2713} Received\x1b[0m");

            // 7. Exchange code for token
            print!("  \x1b[1mStep 3\x1b[0m  Exchanging authorization code... ");
            io::stdout().flush()?;

            let exchange_request = OAuthTokenExchangeRequest::from_config(
                &oauth_config,
                &code,
                &state,
                &pkce.verifier,
                &redirect_uri,
            );

            let client_secret = oauth_meta.default_client_secret.to_string();
            let rt = tokio::runtime::Runtime::new()?;
            let token_result = rt.block_on(async {
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build()?;
                let mut params = exchange_request.form_params();
                // Google's installed-app OAuth requires client_secret in token exchange
                if !client_secret.is_empty() {
                    params.insert("client_secret", client_secret);
                }
                let resp = client
                    .post(&oauth_config.token_url)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .form(&params)
                    .send()
                    .await?;
                let status = resp.status();
                let body = resp.text().await?;
                if !status.is_success() {
                    return Err(format!("token exchange failed ({status}): {body}").into());
                }
                // Try standard OAuth response first, then Anthropic-style
                let token_set: serde_json::Value = serde_json::from_str(&body)?;
                let access_token = token_set.get("access_token")
                    .or_else(|| token_set.get("accessToken"))
                    .and_then(|v| v.as_str())
                    .ok_or("no access_token in response")?
                    .to_string();
                let refresh_token = token_set.get("refresh_token")
                    .or_else(|| token_set.get("refreshToken"))
                    .and_then(|v| v.as_str())
                    .map(ToOwned::to_owned);
                let expires_in = token_set.get("expires_in")
                    .or_else(|| token_set.get("expiresIn"))
                    .and_then(|v| v.as_u64());
                let expires_at = expires_in.map(|ei| {
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        + ei
                });
                Ok::<_, Box<dyn std::error::Error>>(runtime::OAuthTokenSet {
                    access_token,
                    refresh_token,
                    expires_at,
                    scopes: oauth_meta.scopes.iter().map(|s| s.to_string()).collect(),
                })
            });

            match token_result {
                Ok(token_set) => {
                    println!("\x1b[38;5;46m\u{2713} Authenticated\x1b[0m");

                    // 8. Save OAuth token per provider
                    let provider_key = provider.name.replace([' ', '/'], "_").to_lowercase();
                    save_provider_oauth_token(&provider_key, &token_set)?;

                    // 9. Also save as regular credential so /model switching works
                    let api_key = &token_set.access_token;
                    save_provider_credential(provider, api_key)?;

                    // 9b. Save OAuth-specific fields to SQLite
                    if let Ok(db) = orchestrator::knowledge::LearningDb::open() {
                        let _ = db.save_credential(
                            provider.name,
                            provider.env_var,
                            api_key,
                            "oauth",
                            token_set.refresh_token.as_deref(),
                            token_set.expires_at.map(|e| e as i64),
                        );
                    }
                    env::set_var(oauth_meta.token_env_var, api_key);

                    // 10. Fetch models
                    let models = if !provider.models_url.is_empty() {
                        print!("  \x1b[1mStep 4\x1b[0m  Fetching available models... ");
                        io::stdout().flush()?;
                        let fetched = fetch_provider_models(
                            provider.models_url,
                            api_key,
                            provider.test_header,
                        );
                        if fetched.is_empty() {
                            println!("\x1b[2m(none fetched)\x1b[0m");
                        } else {
                            println!("\x1b[38;5;46m{} models\x1b[0m", fetched.len());
                        }
                        fetched
                    } else {
                        Vec::new()
                    };

                    // Summary
                    print_login_summary(provider, api_key, &models);
                }
                Err(err) => {
                    println!("\x1b[38;5;196m\u{2717} Failed\x1b[0m");
                    println!("  \x1b[38;5;196m{err}\x1b[0m");
                    println!();
                    println!("  \x1b[2mFalling back to API key login...\x1b[0m");
                    println!();
                    run_apikey_login(provider)?;
                }
            }
        }
        Ok(Err(err)) => {
            println!("\x1b[38;5;196m\u{2717} Callback error: {err}\x1b[0m");
            println!();
            println!("  \x1b[2mFalling back to API key login...\x1b[0m");
            println!();
            run_apikey_login(provider)?;
        }
        Err(_) => {
            println!("\x1b[38;5;208m\u{26a0} Timed out (2 min)\x1b[0m");
            println!();
            println!("  \x1b[2mFalling back to API key login...\x1b[0m");
            println!();
            run_apikey_login(provider)?;
        }
    }

    Ok(())
}

/// API key login: open dashboard → paste key → validate → save.
fn run_apikey_login(provider: &ProviderOption) -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Open dashboard
    if !provider.dashboard_url.is_empty() {
        println!("  \x1b[1mStep 1\x1b[0m  Opening {} dashboard in your browser...", provider.name);
        println!("  \x1b[2m{}\x1b[0m", provider.dashboard_url);
        println!();
        if !open_browser(provider.dashboard_url) {
            println!("  \x1b[38;5;208mCould not open browser. Visit the URL above manually.\x1b[0m");
        }
        println!("  \x1b[2mCreate or copy your API key from the dashboard, then paste it below.\x1b[0m");
        println!();
    }

    // Step 2: Prompt for key
    println!("  \x1b[1mStep 2\x1b[0m  Paste your API key");
    print!("\x1b[38;5;45m  > \x1b[0m");
    io::stdout().flush()?;
    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)?;
    let api_key = api_key.trim().to_string();

    if api_key.is_empty() {
        println!("  \x1b[38;5;196mNo key entered. Aborted.\x1b[0m");
        return Ok(());
    }

    // Step 3: Validate
    println!();
    print!("  \x1b[1mStep 3\x1b[0m  Authenticating with {}... ", provider.name);
    io::stdout().flush()?;

    let key_valid = if provider.test_url.is_empty() {
        true
    } else {
        test_api_key(provider.test_url, &api_key, provider.test_header)
    };

    if key_valid {
        println!("\x1b[38;5;46m\u{2713} Authenticated\x1b[0m");
    } else {
        println!("\x1b[38;5;208m\u{26a0} Could not verify (key saved anyway)\x1b[0m");
    }

    // Step 4: Fetch models
    let models = if !provider.models_url.is_empty() {
        print!("  \x1b[1mStep 4\x1b[0m  Fetching available models... ");
        io::stdout().flush()?;
        let fetched = fetch_provider_models(provider.models_url, &api_key, provider.test_header);
        if fetched.is_empty() {
            println!("\x1b[2m(none fetched)\x1b[0m");
        } else {
            println!("\x1b[38;5;46m{} models\x1b[0m", fetched.len());
        }
        fetched
    } else {
        Vec::new()
    };

    // Step 5: Save
    save_provider_credential(provider, &api_key)?;
    env::set_var(provider.env_var, &api_key);

    print_login_summary(provider, &api_key, &models);
    Ok(())
}

/// OpenAnalyst free model login — auto-generate key, connect through api.openanalyst.com.
fn run_openanalyst_free_login(provider: &ProviderOption) -> Result<(), Box<dyn std::error::Error>> {
    // Generate a unique sk-oa-free key
    let machine_id = env::var("COMPUTERNAME")
        .or_else(|_| env::var("HOSTNAME"))
        .unwrap_or_else(|_| "cli".to_string())
        .to_lowercase();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let api_key = format!("sk-oa-free-{machine_id}-{timestamp}");

    print!("  \x1b[1mStep 1\x1b[0m  Connecting to OpenAnalyst... ");
    io::stdout().flush()?;

    // Verify API is reachable
    let health_ok = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()
        .and_then(|c| c.get("https://api.openanalyst.com/api/health").send().ok())
        .map_or(false, |r| r.status().is_success());

    if health_ok {
        println!("\x1b[38;5;46m\u{2713} Connected\x1b[0m");
    } else {
        println!("\x1b[38;5;208m\u{26a0} API may be starting up\x1b[0m");
    }

    // Save credentials
    save_provider_credential(provider, &api_key)?;
    env::set_var(provider.env_var, &api_key);

    println!();
    println!("  \x1b[38;5;46m\u{2713}\x1b[0m \x1b[1mFree model access configured\x1b[0m");
    println!();
    println!("  \x1b[2mModel:    openai/gpt-oss-120b\x1b[0m");
    println!("  \x1b[2mCredits:  unlimited (free tier)\x1b[0m");
    println!("  \x1b[2mKey:      {}...{}\x1b[0m", &api_key[..12], &api_key[api_key.len()-4..]);

    Ok(())
}

/// Save OpenAnalyst routing mode (free or api) to credentials and env.
fn save_openanalyst_mode(mode: &str) {
    env::set_var("OPENANALYST_MODE", mode);
    // Also persist to credentials.json so it survives restarts
    let config_dir = runtime::credentials_config_home().ok()
        .or_else(|| {
            env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok()
                .map(|h| PathBuf::from(h).join(".openanalyst"))
        });
    if let Some(config_dir) = config_dir {
        let creds_path = config_dir.join("credentials.json");
        if let Ok(content) = fs::read_to_string(&creds_path) {
            if let Ok(mut creds) = serde_json::from_str::<serde_json::Value>(&content) {
                creds["openanalyst_mode"] = json!(mode);
                let _ = fs::write(&creds_path, serde_json::to_string_pretty(&creds).unwrap_or_default());
            }
        }
        // Also persist to .env
        let _ = upsert_dotenv_key(&config_dir.join(".env"), "OPENANALYST_MODE", mode);
    }
}

/// Save a provider's API key to ~/.openanalyst/credentials.json.
fn save_provider_credential(provider: &ProviderOption, api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = runtime::credentials_config_home()
        .unwrap_or_else(|_| {
            let home = env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".openanalyst")
        });
    fs::create_dir_all(&config_dir)?;

    // ── Save to credentials.json (structured provider map) ──
    let creds_path = config_dir.join("credentials.json");
    let mut creds: serde_json::Value = if creds_path.exists() {
        let content = fs::read_to_string(&creds_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    creds["active_provider"] = json!(provider.name);
    creds["providers"] = creds.get("providers").cloned().unwrap_or_else(|| json!({}));
    creds["providers"][provider.name] = json!({
        "env_var": provider.env_var,
        "api_key": api_key,
    });

    fs::write(&creds_path, serde_json::to_string_pretty(&creds)?)?;

    // ── Also persist to .env (single source of truth for all env-based auth) ──
    upsert_dotenv_key(&config_dir.join(".env"), provider.env_var, api_key)?;

    // ── Also persist to SQLite (3rd fallback layer) ──
    if let Ok(db) = orchestrator::knowledge::LearningDb::open() {
        let _ = db.save_credential(provider.name, provider.env_var, api_key, "api_key", None, None);
    }

    Ok(())
}

/// Insert or update a key=value pair in a .env file.
/// If the key exists (commented or uncommented), replaces the line.
/// If it doesn't exist, appends it at the end.
fn upsert_dotenv_key(env_path: &Path, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(env_path).unwrap_or_default();
    let new_line = format!("{key}={value}");
    let mut found = false;
    let mut lines: Vec<String> = content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            // Match: KEY=..., # KEY=..., or #KEY=...
            let bare = trimmed.trim_start_matches('#').trim();
            if bare.starts_with(key) && bare[key.len()..].starts_with('=') {
                found = true;
                new_line.clone()
            } else {
                line.to_string()
            }
        })
        .collect();

    if !found {
        // Append under a blank line at the end
        if !lines.last().map_or(true, |l| l.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(new_line);
    }

    fs::write(env_path, lines.join("\n"))?;
    Ok(())
}

/// Print login success summary.
fn print_login_summary(provider: &ProviderOption, api_key: &str, models: &[String]) {
    let creds_path = runtime::credentials_config_home()
        .map(|d| d.join("credentials.json"))
        .unwrap_or_else(|_| PathBuf::from("~/.openanalyst/credentials.json"));

    println!();
    println!("  \x1b[38;5;45m\x1b[1m\u{2713} Login complete\x1b[0m");
    println!();
    println!("  \x1b[2mProvider\x1b[0m     {}", provider.name);
    println!("  \x1b[2mEnv var\x1b[0m      {}", provider.env_var);

    let masked = if api_key.len() > 8 {
        format!("{}...{}", &api_key[..4], &api_key[api_key.len()-4..])
    } else {
        "****".to_string()
    };
    println!("  \x1b[2mAPI key\x1b[0m      {}", masked);

    if !models.is_empty() {
        let display_models: Vec<&str> = models.iter().take(8).map(|s| s.as_str()).collect();
        println!("  \x1b[2mModels\x1b[0m       {}", display_models.join(", "));
        if models.len() > 8 {
            println!("               \x1b[2m...and {} more\x1b[0m", models.len() - 8);
        }
    }

    println!("  \x1b[2mSaved to\x1b[0m     {}", creds_path.display());
}

struct ProviderModelsConfig {
    name: &'static str,
    keys: &'static [&'static str],
    models_url: &'static str,
    auth_header: &'static str,
}

/// Fetch model list from a provider's /models endpoint. Returns model IDs.
fn fetch_provider_models(url: &str, key: &str, header_type: &str) -> Vec<String> {
    if url.is_empty() || key.is_empty() {
        return Vec::new();
    }
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return Vec::new(),
    };
    rt.block_on(async {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut req = client.get(url);
        match header_type {
            "bearer" => req = req.bearer_auth(key),
            "x-api-key" => req = req.header("x-api-key", key),
            _ => req = req.bearer_auth(key),
        }
        req = req.header("anthropic-version", "2023-06-01");

        let resp = match req.send().await {
            Ok(r) if r.status().is_success() => r,
            _ => return Vec::new(),
        };
        let body: serde_json::Value = match resp.json().await {
            Ok(b) => b,
            Err(_) => return Vec::new(),
        };

        // Parse models from response — handles multiple formats:
        // OpenAI/xAI/OpenRouter: {"data": [{"id": "model-name"}, ...]}
        // Anthropic: {"data": [{"id": "model-name"}, ...]} or {"models": [...]}
        // OpenAnalyst: could be array or {"data": [...]} or {"models": [...]}
        let models_array = body.get("data")
            .and_then(|d| d.as_array())
            .or_else(|| body.get("models").and_then(|m| m.as_array()))
            .or_else(|| body.as_array());

        let Some(models) = models_array else {
            return Vec::new();
        };

        models.iter()
            .filter_map(|m| {
                // Each model can be {"id": "..."} or just a string
                m.get("id").and_then(|v| v.as_str())
                    .or_else(|| m.get("name").and_then(|v| v.as_str()))
                    .or_else(|| m.as_str())
                    .map(ToOwned::to_owned)
            })
            .filter(|id| {
                // Filter out embedding/moderation/whisper/dall-e models for cleaner output
                !id.contains("embed")
                    && !id.contains("moderation")
                    && !id.contains("whisper")
                    && !id.contains("dall-e")
                    && !id.contains("tts")
                    && !id.contains("babbage")
                    && !id.contains("davinci")
            })
            .collect()
    })
}

fn test_api_key(url: &str, key: &str, header_type: &str) -> bool {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return false,
    };
    rt.block_on(async {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };
        let mut req = client.get(url);
        match header_type {
            "bearer" => req = req.bearer_auth(key),
            "x-api-key" => req = req.header("x-api-key", key),
            _ => req = req.bearer_auth(key),
        }
        req = req.header("anthropic-version", "2023-06-01");
        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                // 200 = valid, 401 = bad key, anything else = endpoint works but needs body
                status != 401 && status != 403
            }
            Err(_) => false,
        }
    })
}

fn run_logout() -> Result<(), Box<dyn std::error::Error>> {
    // Clear OAuth credentials
    let _ = clear_oauth_credentials();

    // Clear saved provider credentials
    let config_dir = runtime::credentials_config_home()
        .unwrap_or_else(|_| {
            let home = env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".openanalyst")
        });
    let creds_path = config_dir.join("credentials.json");
    if creds_path.exists() {
        fs::remove_file(&creds_path)?;
    }

    // Clear env vars
    env::remove_var("OPENANALYST_AUTH_TOKEN");
    env::remove_var("OPENANALYST_API_KEY");
    env::remove_var("OPENANALYST_MODE");
    env::remove_var("ANTHROPIC_API_KEY");
    env::remove_var("OPENAI_API_KEY");
    env::remove_var("GEMINI_API_KEY");
    env::remove_var("XAI_API_KEY");

    println!();
    println!("  \x1b[38;5;45m\u{2713}\x1b[0m \x1b[1mLogged out successfully.\x1b[0m");
    println!();
    println!("  Run \x1b[1mopenanalyst login\x1b[0m to sign in again.");
    println!();
    Ok(())
}

fn run_update() -> Result<(), Box<dyn std::error::Error>> {
    const CURRENT_VERSION: &str = VERSION;
    const REPO: &str = "OpenAnalystInc/openanalyst-cli";

    println!();
    println!("  \x1b[38;5;45m\x1b[1mOpenAnalyst CLI \x1b[0m\x1b[2m— Update\x1b[0m");
    println!();
    println!("  \x1b[2mCurrent version:\x1b[0m v{CURRENT_VERSION}");

    // 1. Check latest release from GitHub API
    print!("  \x1b[2mChecking for updates...\x1b[0m");
    io::stdout().flush()?;

    let rt = tokio::runtime::Runtime::new()?;
    let latest_version = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build().ok()?;
        let resp = client
            .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
            .header("User-Agent", "openanalyst-cli")
            .send().await.ok()?;
        let body: serde_json::Value = resp.json().await.ok()?;
        body.get("tag_name")
            .and_then(|v| v.as_str())
            .map(|v| v.trim_start_matches('v').to_string())
    });

    let Some(latest) = latest_version else {
        println!(" \x1b[38;5;208mcould not reach GitHub\x1b[0m");
        println!();
        return Ok(());
    };

    if latest == CURRENT_VERSION {
        println!(" \x1b[38;5;46m\u{2713} already up to date (v{latest})\x1b[0m");
        println!();
        return Ok(());
    }

    println!(" \x1b[38;5;46mv{latest} available\x1b[0m");
    println!();

    // 2. Detect platform and download binary
    let current_exe = env::current_exe().unwrap_or_default();
    let target = if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { "aarch64-apple-darwin" } else { "x86_64-apple-darwin" }
    } else {
        if cfg!(target_arch = "aarch64") { "aarch64-unknown-linux-gnu" } else { "x86_64-unknown-linux-gnu" }
    };

    let ext = if cfg!(target_os = "windows") { ".exe" } else { "" };
    let download_url = format!(
        "https://github.com/{REPO}/releases/download/v{latest}/openanalyst-{target}{ext}"
    );

    print!("  \x1b[2mDownloading v{latest}...\x1b[0m");
    io::stdout().flush()?;

    let download_result = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;
        let resp = client.get(&download_url).send().await?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()).into());
        }
        let bytes = resp.bytes().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(bytes)
    });

    match download_result {
        Ok(bytes) => {
            println!(" \x1b[38;5;46m\u{2713} {:.1} MB\x1b[0m", bytes.len() as f64 / 1_048_576.0);

            // 3. Replace binary
            let temp_path = current_exe.with_extension("update");
            fs::write(&temp_path, &bytes)?;

            if cfg!(target_os = "windows") {
                // Windows: rename current → .old, new → current
                let old_path = current_exe.with_extension("old.exe");
                let _ = fs::remove_file(&old_path);
                fs::rename(&current_exe, &old_path)?;
                fs::rename(&temp_path, &current_exe)?;
                println!("  \x1b[38;5;46m\u{2713} Updated to v{latest}\x1b[0m");
                println!("  \x1b[2mRestart your terminal to use the new version.\x1b[0m");
            } else {
                // Unix: replace in place, set executable
                fs::rename(&temp_path, &current_exe)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&current_exe, fs::Permissions::from_mode(0o755));
                }
                println!("  \x1b[38;5;46m\u{2713} Updated to v{latest}\x1b[0m");
            }
        }
        Err(e) => {
            println!(" \x1b[38;5;208mfailed: {e}\x1b[0m");
            println!();
            println!("  \x1b[2mManual update:\x1b[0m");
            if cfg!(target_os = "windows") {
                println!("  \x1b[38;5;45mirm https://raw.githubusercontent.com/OpenAnalystInc/openanalyst-cli/main/install.ps1 | iex\x1b[0m");
            } else {
                println!("  \x1b[38;5;45mcurl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/openanalyst-cli/main/install.sh | bash\x1b[0m");
            }
        }
    }

    println!();
    Ok(())
}

fn run_uninstall() -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal,
    };

    println!();
    println!("  \x1b[38;5;45m\x1b[1mOpenAnalyst CLI \x1b[0m\x1b[2m— Uninstall\x1b[0m");
    println!();

    let current_exe = env::current_exe().unwrap_or_default();
    let config_dir = env::var("OPENANALYST_CONFIG_HOME")
        .or_else(|_| env::var("HOME").map(|h| format!("{h}/.openanalyst")))
        .or_else(|_| env::var("USERPROFILE").map(|h| format!("{h}\\.openanalyst")))
        .unwrap_or_else(|_| "~/.openanalyst".to_string());

    println!("  This will remove:");
    println!("    \x1b[2m•\x1b[0m Binary:  {}", current_exe.display());
    println!("    \x1b[2m•\x1b[0m Config:  {}", config_dir);
    println!();
    print!("  \x1b[38;5;208mAre you sure? [y/N]\x1b[0m ");
    io::stdout().flush()?;

    terminal::enable_raw_mode()?;

    // Drain any buffered key events (e.g. the Enter from typing the command)
    while event::poll(Duration::from_millis(50))? {
        let _ = event::read()?;
    }

    let confirmed = loop {
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => break true,
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Enter => break false,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break false,
                    _ => {}
                }
            }
        }
    };
    terminal::disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, cursor::Show)?;

    if !confirmed {
        println!("\n\n  \x1b[2mUninstall cancelled.\x1b[0m\n");
        return Ok(());
    }

    println!("y\n");

    // Remove config directory
    let config_path = PathBuf::from(&config_dir);
    if config_path.exists() {
        match fs::remove_dir_all(&config_path) {
            Ok(()) => println!("  \x1b[38;5;46m\u{2713}\x1b[0m Removed {config_dir}"),
            Err(e) => println!("  \x1b[38;5;208m\u{26a0}\x1b[0m Could not remove {config_dir}: {e}"),
        }
    }

    // Remove PATH entry (best-effort)
    if let Some(parent) = current_exe.parent() {
        let install_dir = parent.to_string_lossy();
        // Try removing from shell rc files
        for rc_name in &[".zshrc", ".bashrc", ".bash_profile"] {
            let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).unwrap_or_default();
            let rc_path = PathBuf::from(&home).join(rc_name);
            if rc_path.exists() {
                if let Ok(content) = fs::read_to_string(&rc_path) {
                    let filtered: Vec<&str> = content.lines()
                        .filter(|line| !line.contains(&*install_dir) && !line.contains("# OpenAnalyst CLI"))
                        .collect();
                    let _ = fs::write(&rc_path, filtered.join("\n") + "\n");
                }
            }
        }
        println!("  \x1b[38;5;46m\u{2713}\x1b[0m Removed PATH entry");
    }

    // Remove binary (schedule self-deletion)
    if current_exe.exists() {
        if cfg!(target_os = "windows") {
            // On Windows, can't delete running exe — schedule deletion
            let _ = Command::new("cmd")
                .args(["/C", "ping", "127.0.0.1", "-n", "2", ">nul", "&", "del", "/f",
                    &current_exe.to_string_lossy()])
                .spawn();
            println!("  \x1b[38;5;46m\u{2713}\x1b[0m Binary will be removed on exit");
        } else {
            match fs::remove_file(&current_exe) {
                Ok(()) => println!("  \x1b[38;5;46m\u{2713}\x1b[0m Removed {}", current_exe.display()),
                Err(e) => println!("  \x1b[38;5;208m\u{26a0}\x1b[0m Could not remove binary: {e}"),
            }
        }
    }

    println!();
    println!("  \x1b[38;5;45mOpenAnalyst CLI has been uninstalled.\x1b[0m");
    println!("  \x1b[2mTo reinstall:\x1b[0m");
    if cfg!(target_os = "windows") {
        println!("  \x1b[38;5;45mirm https://raw.githubusercontent.com/OpenAnalystInc/openanalyst-cli/main/install.ps1 | iex\x1b[0m");
    } else {
        println!("  \x1b[38;5;45mcurl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/openanalyst-cli/main/install.sh | bash\x1b[0m");
    }
    println!();
    Ok(())
}

fn run_whoami() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = runtime::credentials_config_home()
        .unwrap_or_else(|_| {
            let home = env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".openanalyst")
        });
    let creds_path = config_dir.join("credentials.json");

    println!();
    println!("  \x1b[38;5;45m\x1b[1mOpenAnalyst CLI \x1b[0m\x1b[2m— Provider Status\x1b[0m");
    println!();

    let creds: serde_json::Value = if creds_path.exists() {
        let content = fs::read_to_string(&creds_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        println!("  \x1b[2mNo providers configured. Run \x1b[0m\x1b[1mopenanalyst login\x1b[0m\x1b[2m to get started.\x1b[0m");
        println!();
        return Ok(());
    };

    let active = creds.get("active_provider").and_then(|v| v.as_str()).unwrap_or("");
    let providers = creds.get("providers").and_then(|v| v.as_object());

    let mut logged_in = 0u32;

    for provider_opt in LOGIN_PROVIDERS {
        // Check env var (loaded from credentials.json at startup)
        let has_key = env::var(provider_opt.env_var).ok().filter(|v| !v.is_empty()).is_some();
        // Also check credentials.json
        let saved_key = providers.and_then(|p| {
            p.get(provider_opt.name)
                .and_then(|v| v.get("api_key"))
                .and_then(|v| v.as_str())
                .filter(|k| !k.is_empty())
        });

        let is_active = provider_opt.name == active;

        if has_key || saved_key.is_some() {
            logged_in += 1;
            let key_display = saved_key
                .map(ToOwned::to_owned)
                .or_else(|| env::var(provider_opt.env_var).ok().filter(|v| !v.is_empty()))
                .unwrap_or_default();
            let masked = if key_display.len() > 8 {
                format!("{}...{}", &key_display[..4], &key_display[key_display.len()-4..])
            } else if !key_display.is_empty() {
                "****".to_string()
            } else {
                String::new()
            };
            let active_badge = if is_active { " \x1b[38;5;46m[active]\x1b[0m" } else { "" };
            println!("  \x1b[38;5;46m\u{2713}\x1b[0m \x1b[1m{}\x1b[0m{}", provider_opt.name, active_badge);
            println!("    \x1b[2mKey:\x1b[0m  {}  \x1b[2m({})\x1b[0m", masked, provider_opt.env_var);
        } else {
            println!("  \x1b[2m\u{2717} {}\x1b[0m", provider_opt.name);
        }
    }

    println!();
    if logged_in == 0 {
        println!("  \x1b[2mNo providers logged in.\x1b[0m");
        println!("  \x1b[2mRun \x1b[0m\x1b[1mopenanalyst login\x1b[0m\x1b[2m or edit \x1b[0m\x1b[1m~/.openanalyst/.env\x1b[0m\x1b[2m to add your API keys.\x1b[0m");
    } else {
        println!("  \x1b[2m{} provider(s) authenticated.\x1b[0m", logged_in);
        println!("  \x1b[2mSwitch models with\x1b[0m /model <name> \x1b[2min the REPL.\x1b[0m");
        println!("  \x1b[2mAdd more with\x1b[0m openanalyst login\x1b[2m.\x1b[0m");
    }
    println!();
    Ok(())
}

fn print_system_prompt(cwd: PathBuf, date: String) {
    match load_system_prompt(cwd, date, env::consts::OS, "unknown") {
        Ok(sections) => println!("{}", sections.join("\n\n")),
        Err(error) => {
            eprintln!("failed to build system prompt: {error}");
            std::process::exit(1);
        }
    }
}

fn print_version() {
    println!("{}", render_version_report());
}

fn resume_session(session_path: &Path, commands: &[String]) {
    let session = match Session::load_from_path(session_path) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("failed to restore session: {error}");
            std::process::exit(1);
        }
    };

    if commands.is_empty() {
        println!(
            "Restored session from {} ({} messages).",
            session_path.display(),
            session.messages.len()
        );
        return;
    }

    let mut session = session;
    for raw_command in commands {
        let Some(command) = SlashCommand::parse(raw_command) else {
            eprintln!("unsupported resumed command: {raw_command}");
            std::process::exit(2);
        };
        match run_resume_command(session_path, &session, &command) {
            Ok(ResumeCommandOutcome {
                session: next_session,
                message,
            }) => {
                session = next_session;
                if let Some(message) = message {
                    println!("{message}");
                }
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ResumeCommandOutcome {
    session: Session,
    message: Option<String>,
}

#[derive(Debug, Clone)]
struct StatusContext {
    cwd: PathBuf,
    session_path: Option<PathBuf>,
    loaded_config_files: usize,
    discovered_config_files: usize,
    memory_file_count: usize,
    project_root: Option<PathBuf>,
    git_branch: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct StatusUsage {
    message_count: usize,
    turns: u32,
    latest: TokenUsage,
    cumulative: TokenUsage,
    estimated_tokens: usize,
}

fn format_model_report(model: &str, message_count: usize, turns: u32) -> String {
    format!(
        "Model
  Current model    {model}
  Session messages {message_count}
  Session turns    {turns}

Usage
  Inspect current model with /model
  Switch models with /model <name>"
    )
}

fn format_model_switch_report(previous: &str, next: &str, message_count: usize) -> String {
    format!(
        "Model updated
  Previous         {previous}
  Current          {next}
  Preserved msgs   {message_count}"
    )
}

fn format_permissions_report(mode: &str) -> String {
    let modes = [
        ("read-only", "Read/search tools only", mode == "read-only"),
        (
            "workspace-write",
            "Edit files inside the workspace",
            mode == "workspace-write",
        ),
        (
            "danger-full-access",
            "Unrestricted tool access",
            mode == "danger-full-access",
        ),
    ]
    .into_iter()
    .map(|(name, description, is_current)| {
        let marker = if is_current {
            "● current"
        } else {
            "○ available"
        };
        format!("  {name:<18} {marker:<11} {description}")
    })
    .collect::<Vec<_>>()
    .join(
        "
",
    );

    format!(
        "Permissions
  Active mode      {mode}
  Mode status      live session default

Modes
{modes}

Usage
  Inspect current mode with /permissions
  Switch modes with /permissions <mode>"
    )
}

fn format_permissions_switch_report(previous: &str, next: &str) -> String {
    format!(
        "Permissions updated
  Result           mode switched
  Previous mode    {previous}
  Active mode      {next}
  Applies to       subsequent tool calls
  Usage            /permissions to inspect current mode"
    )
}

fn format_cost_report(usage: TokenUsage) -> String {
    format!(
        "Cost
  Input tokens     {}
  Output tokens    {}
  Cache create     {}
  Cache read       {}
  Total tokens     {}",
        usage.input_tokens,
        usage.output_tokens,
        usage.cache_creation_input_tokens,
        usage.cache_read_input_tokens,
        usage.total_tokens(),
    )
}

fn format_resume_report(session_path: &str, message_count: usize, turns: u32) -> String {
    format!(
        "Session resumed
  Session file     {session_path}
  Messages         {message_count}
  Turns            {turns}"
    )
}

fn format_compact_report(removed: usize, resulting_messages: usize, skipped: bool) -> String {
    if skipped {
        format!(
            "Compact
  Result           skipped
  Reason           session below compaction threshold
  Messages kept    {resulting_messages}"
        )
    } else {
        format!(
            "Compact
  Result           compacted
  Messages removed {removed}
  Messages kept    {resulting_messages}"
        )
    }
}

fn parse_git_status_metadata(status: Option<&str>) -> (Option<PathBuf>, Option<String>) {
    let Some(status) = status else {
        return (None, None);
    };
    let branch = status.lines().next().and_then(|line| {
        line.strip_prefix("## ")
            .map(|line| {
                line.split(['.', ' '])
                    .next()
                    .unwrap_or_default()
                    .to_string()
            })
            .filter(|value| !value.is_empty())
    });
    let project_root = find_git_root().ok();
    (project_root, branch)
}

fn find_git_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        return Err("not a git repository".into());
    }
    let path = String::from_utf8(output.stdout)?.trim().to_string();
    if path.is_empty() {
        return Err("empty git root".into());
    }
    Ok(PathBuf::from(path))
}

#[allow(clippy::too_many_lines)]
fn run_resume_command(
    session_path: &Path,
    session: &Session,
    command: &SlashCommand,
) -> Result<ResumeCommandOutcome, Box<dyn std::error::Error>> {
    match command {
        SlashCommand::Help => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_repl_help()),
        }),
        SlashCommand::Compact => {
            let result = runtime::compact_session(
                session,
                CompactionConfig {
                    max_estimated_tokens: 0,
                    ..CompactionConfig::default()
                },
            );
            let removed = result.removed_message_count;
            let kept = result.compacted_session.messages.len();
            let skipped = removed == 0;
            result.compacted_session.save_to_path(session_path)?;
            Ok(ResumeCommandOutcome {
                session: result.compacted_session,
                message: Some(format_compact_report(removed, kept, skipped)),
            })
        }
        SlashCommand::Clear { confirm } => {
            if !confirm {
                return Ok(ResumeCommandOutcome {
                    session: session.clone(),
                    message: Some(
                        "clear: confirmation required; rerun with /clear --confirm".to_string(),
                    ),
                });
            }
            let cleared = Session::new();
            cleared.save_to_path(session_path)?;
            Ok(ResumeCommandOutcome {
                session: cleared,
                message: Some(format!(
                    "Cleared resumed session file {}.",
                    session_path.display()
                )),
            })
        }
        SlashCommand::Status => {
            let tracker = UsageTracker::from_session(session);
            let usage = tracker.cumulative_usage();
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format_status_report(
                    "restored-session",
                    StatusUsage {
                        message_count: session.messages.len(),
                        turns: tracker.turns(),
                        latest: tracker.current_turn_usage(),
                        cumulative: usage,
                        estimated_tokens: 0,
                    },
                    default_permission_mode().as_str(),
                    &status_context(Some(session_path))?,
                )),
            })
        }
        SlashCommand::Cost => {
            let usage = UsageTracker::from_session(session).cumulative_usage();
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format_cost_report(usage)),
            })
        }
        SlashCommand::Config { section } => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_config_report(section.as_deref())?),
        }),
        SlashCommand::Memory => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_memory_report()?),
        }),
        SlashCommand::Init => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(init_openanalyst_md()?),
        }),
        SlashCommand::Diff => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_diff_report()?),
        }),
        SlashCommand::Version => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(render_version_report()),
        }),
        SlashCommand::Export { path } => {
            let export_path = resolve_export_path(path.as_deref(), session)?;
            fs::write(&export_path, render_export_text(session))?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format!(
                    "Export\n  Result           wrote transcript\n  File             {}\n  Messages         {}",
                    export_path.display(),
                    session.messages.len(),
                )),
            })
        }
        SlashCommand::Agents { args } => {
            let cwd = env::current_dir()?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(handle_agents_slash_command(args.as_deref(), &cwd)?),
            })
        }
        SlashCommand::Skills { args } => {
            let cwd = env::current_dir()?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(handle_skills_slash_command(args.as_deref(), &cwd)?),
            })
        }
        SlashCommand::Bughunter { .. }
        | SlashCommand::Branch { .. }
        | SlashCommand::Worktree { .. }
        | SlashCommand::CommitPushPr { .. }
        | SlashCommand::Commit
        | SlashCommand::Pr { .. }
        | SlashCommand::Issue { .. }
        | SlashCommand::Ultraplan { .. }
        | SlashCommand::Teleport { .. }
        | SlashCommand::DebugToolCall
        | SlashCommand::Resume { .. }
        | SlashCommand::Model { .. }
        | SlashCommand::Permissions { .. }
        | SlashCommand::Session { .. }
        | SlashCommand::Plugins { .. }
        | SlashCommand::Image { .. }
        | SlashCommand::Voice { .. }
        | SlashCommand::Speak { .. }
        | SlashCommand::Vision { .. }
        | SlashCommand::Diagram { .. }
        | SlashCommand::Translate { .. }
        | SlashCommand::Tokens { .. }
        | SlashCommand::DiffReview { .. }
        | SlashCommand::Scrape { .. }
        | SlashCommand::Json { .. }
        | SlashCommand::Dev { .. }
        | SlashCommand::Mcp { .. }
        | SlashCommand::Knowledge { .. }
        | SlashCommand::Explore { .. }
        | SlashCommand::Doctor
        | SlashCommand::Login
        | SlashCommand::Logout
        | SlashCommand::Vim
        | SlashCommand::Think { .. }
        | SlashCommand::Effort { .. }
        | SlashCommand::Route { .. }
        | SlashCommand::Context
        | SlashCommand::Changelog { .. }
        | SlashCommand::AddDir { .. }
        | SlashCommand::Exit
        | SlashCommand::Sidebar
        | SlashCommand::Swarm { .. }
        | SlashCommand::OpenAnalyst { .. }
        | SlashCommand::Ask { .. }
        | SlashCommand::UserPrompt { .. }
        | SlashCommand::Hooks { .. }
        | SlashCommand::Trust { .. }
        | SlashCommand::Undo
        | SlashCommand::Feedback { .. }
        | SlashCommand::Unknown(_) => Err("unsupported resumed slash command".into()),
    }
}

fn run_tui(
    model: String,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    use orchestrator::OrchestratorConfig;
    use tui::banner::BannerAccountInfo;

    let (ui_tx, ui_rx) = tokio::sync::mpsc::channel(256);
    let (action_tx, action_rx) = tokio::sync::mpsc::channel(64);

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Folder trust check — warn if workspace is blocked
    let trust_info = runtime::discover_trust(&cwd);
    if trust_info.level == runtime::TrustLevel::Blocked {
        eprintln!("Workspace is blocked by trust policy: {}", trust_info.reason);
        return Err("Blocked workspace".into());
    }

    // IDE detection — inject into system prompt for context-aware behavior
    let _ide = runtime::detect_ide();
    let ide_context = runtime::ide_context_string();

    // Load system prompt with IDE context
    let mut system_prompt = runtime::load_system_prompt(&cwd, DEFAULT_DATE, "Windows", "11")?;
    if !ide_context.is_empty() {
        system_prompt.push(ide_context);
    }
    if trust_info.level == runtime::TrustLevel::Untrusted {
        system_prompt.push("Note: This workspace is untrusted. Hooks and skills are disabled. Run /trust to enable.".to_string());
    }

    let config = OrchestratorConfig {
        model: model.clone(),
        permission_mode,
        allowed_tools,
        cwd: cwd.clone(),
        system_prompt,
        max_turns: None, // Use default (200 turns)
    };

    let orchestrator = orchestrator::AgentOrchestrator::new(config, ui_tx, action_rx, None);

    let mut app = tui::app::App::new(ui_rx, action_tx, &model);

    // Set banner info
    let account = fetch_startup_account_info(&model);
    app.set_banner(BannerAccountInfo {
        display_name: account.display_name,
        model_display: account.model_display,
        provider_name: account.provider_name,
        user_email: account.user_email,
        organization: account.organization,
        cwd: cwd.display().to_string(),
        version: VERSION.to_string(),
    });

    // Count configured MCP servers for status display
    if let Ok(loader) = runtime::ConfigLoader::default_for(&cwd).load() {
        let mcp_count = loader.mcp().servers().len();
        app.sidebar_state.mcp_servers_connected = mcp_count;
    }

    // Install panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = tui::restore_terminal();
        original_hook(info);
    }));

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Spawn orchestrator in background
        let orchestrator_handle = tokio::spawn(orchestrator.run());

        // Run TUI event loop
        let terminal = tui::setup_terminal()?;
        let result = tui::event_loop::run_event_loop(app, terminal).await;
        tui::restore_terminal()?;

        // Send Quit to orchestrator and wait for clean shutdown
        orchestrator_handle.abort();
        let _ = orchestrator_handle.await;

        // If user logged out (credentials cleared), show re-login instructions
        let has_creds = env::var("OPENANALYST_AUTH_TOKEN").ok().filter(|v| !v.is_empty()).is_some()
            || env::var("ANTHROPIC_API_KEY").ok().filter(|v| !v.is_empty()).is_some()
            || env::var("OPENAI_API_KEY").ok().filter(|v| !v.is_empty()).is_some()
            || env::var("GEMINI_API_KEY").ok().filter(|v| !v.is_empty()).is_some();
        if !has_creds {
            println!();
            println!("  \x1b[38;5;45m\u{2713}\x1b[0m \x1b[1mLogged out.\x1b[0m");
            println!();
            println!("  Run \x1b[1mopenanalyst login\x1b[0m to authenticate again.");
            println!();
        }

        result
    })?;

    Ok(())
}

fn run_repl(
    model: String,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cli = LiveCli::new(model, true, allowed_tools, permission_mode)?;
    let mut editor = input::LineEditor::new("> ", slash_command_completion_candidates());
    cli.animate_startup_banner();

    loop {
        match editor.read_line()? {
            input::ReadOutcome::Submit(input) => {
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if matches!(trimmed.as_str(), "/exit" | "/quit") {
                    cli.persist_session()?;
                    break;
                }
                if let Some(command) = SlashCommand::parse(&trimmed) {
                    if cli.handle_repl_command(command)? {
                        cli.persist_session()?;
                    }
                    continue;
                }
                editor.push_history(input);
                cli.run_turn(&trimmed)?;
            }
            input::ReadOutcome::Cancel => {}
            input::ReadOutcome::Exit => {
                cli.persist_session()?;
                break;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct SessionHandle {
    id: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct ManagedSessionSummary {
    id: String,
    path: PathBuf,
    modified_epoch_secs: u64,
    message_count: usize,
}

struct LiveCli {
    model: String,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    system_prompt: Vec<String>,
    runtime: ConversationRuntime<DefaultRuntimeClient, CliToolExecutor>,
    session: SessionHandle,
}

impl LiveCli {
    fn new(
        model: String,
        enable_tools: bool,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let system_prompt = build_system_prompt()?;
        let session = create_managed_session_handle()?;
        let runtime = build_runtime(
            Session::new(),
            model.clone(),
            system_prompt.clone(),
            enable_tools,
            true,
            allowed_tools.clone(),
            permission_mode,
            None,
        )?;
        let cli = Self {
            model,
            allowed_tools,
            permission_mode,
            system_prompt,
            runtime,
            session,
        };
        cli.persist_session()?;
        Ok(cli)
    }

    fn animate_startup_banner(&self) {
        let cwd = env::current_dir().map_or_else(
            |_| "<unknown>".to_string(),
            |path| path.display().to_string(),
        );

        // Detect which auth provider is active and fetch account info from API
        let account = fetch_startup_account_info(&self.model);

        let welcome_msg = format!("Welcome back, {}!", account.display_name);
        let provider_label = format!(
            "{} \u{00b7} {}",
            account.model_display, account.provider_name
        );
        let user_line = {
            let mut parts: Vec<String> = Vec::new();
            if let Some(ref name) = account.user_name { parts.push(name.clone()); }
            if let Some(ref email) = account.user_email { parts.push(email.clone()); }
            if let Some(ref sub) = account.subscription { parts.push(sub.clone()); }
            if let Some(ref org) = account.organization {
                parts.push(format!("{org}'s Organization"));
            }
            if parts.is_empty() { None } else { Some(parts.join(" \u{00b7} ")) }
        };

        let mut stdout = io::stdout();

        // ── ANSI color constants ──
        let c_border = "\x1b[38;5;39m";   // blue for borders
        let c_accent = "\x1b[38;5;45m";   // cyan for accents/headings
        let c_text   = "\x1b[38;5;252m";  // light gray for body text
        let c_dim    = "\x1b[2m";          // dim for secondary info
        let c_bold   = "\x1b[1m";          // bold
        let c_reset  = "\x1b[0m";

        // ── Big OA block letters (cyan/blue) ──
        let mascot: &[&str] = &[
            "  \x1b[38;5;39m ██████  \x1b[38;5;45m █████ \x1b[0m",
            "  \x1b[38;5;39m██    ██ \x1b[38;5;45m██   ██\x1b[0m",
            "  \x1b[38;5;39m██    ██ \x1b[38;5;45m███████\x1b[0m",
            "  \x1b[38;5;39m██    ██ \x1b[38;5;45m██   ██\x1b[0m",
            "  \x1b[38;5;39m ██████  \x1b[38;5;45m██   ██\x1b[0m",
        ];

        // ── Layout dimensions ──
        let total_width: usize = 80;
        let left_width: usize = 46;
        let right_width: usize = total_width - left_width - 1; // -1 for middle border

        // ── Hide cursor ──
        let _ = write!(stdout, "\x1b[?25l");

        // ── Phase 1: Header line — typewriter reveal ──
        let header_text = format!(" OpenAnalyst CLI v{VERSION} ");
        let header_pad = total_width.saturating_sub(header_text.len() + 2);
        let header_line: String = format!(
            "\u{2500} {header_text} {}\u{2500}",
            "\u{2500}".repeat(header_pad)
        );

        for ch in header_line.chars() {
            let _ = write!(stdout, "{c_accent}{ch}{c_reset}");
            let _ = stdout.flush();
            let delay = if ch == '\u{2500}' { 3 } else { 15 };
            thread::sleep(Duration::from_millis(delay));
        }
        let _ = writeln!(stdout);
        let _ = stdout.flush();

        // ── Phase 2: Draw the box layout line by line ──
        let draw_delay = Duration::from_millis(25);

        // Helper: visible width (strips ANSI escape sequences)
        let visible_width = |s: &str| -> usize {
            let mut width: usize = 0;
            let mut in_escape = false;
            for ch in s.chars() {
                if in_escape {
                    if ch.is_ascii_alphabetic() {
                        in_escape = false;
                    }
                } else if ch == '\x1b' {
                    in_escape = true;
                } else {
                    width += 1;
                }
            }
            width
        };
        // Helper: pad a string to a given display width
        let pad = |s: &str, width: usize| -> String {
            let vw = visible_width(s);
            let pad_needed = width.saturating_sub(vw);
            format!("{s}{}", " ".repeat(pad_needed))
        };

        // Build rows: (left_content, right_content)
        // Model and directory are shown in the box rows below
        let rows: Vec<(String, String)> = vec![
            (
                String::new(),
                format!("{c_accent}{c_bold}Tips for getting started{c_reset}"),
            ),
            (
                format!("  {c_text}{c_bold}{welcome_msg}{c_reset}"),
                format!("{c_text}Run {c_bold}/init{c_reset}{c_text} to create an{c_reset}"),
            ),
            (
                String::new(),
                format!("{c_text}OPENANALYST.md file with{c_reset}"),
            ),
            (
                mascot[0].to_string(),
                format!("{c_text}instructions for OpenAnalyst{c_reset}"),
            ),
            (
                mascot[1].to_string(),
                format!("{c_border}{}{c_reset}", "\u{2500}".repeat(right_width - 2)),
            ),
            (
                mascot[2].to_string(),
                format!("{c_accent}{c_bold}Recent activity{c_reset}"),
            ),
            (
                mascot[3].to_string(),
                format!("{c_dim}No recent activity{c_reset}"),
            ),
            (
                mascot[4].to_string(),
                String::new(),
            ),
            (
                String::new(),
                String::new(),
            ),
            (
                format!("  {c_text}{provider_label}{c_reset}"),
                String::new(),
            ),
            (
                user_line.as_ref().map_or_else(String::new, |line| format!("  {c_dim}{line}{c_reset}")),
                String::new(),
            ),
            (
                format!("  {c_dim}{cwd}{c_reset}"),
                String::new(),
            ),
        ];

        for (left, right) in &rows {
            let left_padded = pad(left, left_width - 2);
            let right_padded = pad(right, right_width - 2);
            let _ = writeln!(
                stdout,
                "{c_border}\u{2502}{c_reset} {left_padded}{c_border}\u{2502}{c_reset} {right_padded}{c_border}\u{2502}{c_reset}"
            );
            let _ = stdout.flush();
            thread::sleep(draw_delay);
        }

        // Bottom border
        let _ = writeln!(
            stdout,
            "{c_border}\u{2514}{}\u{2534}{}\u{2518}{c_reset}",
            "\u{2500}".repeat(left_width - 1),
            "\u{2500}".repeat(right_width),
        );
        let _ = stdout.flush();

        // OA already rendered in blue/cyan from the box draw above

        // ── Show cursor ──
        let _ = write!(stdout, "\x1b[?25h");
        let _ = stdout.flush();
        println!();
    }

    fn run_turn(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut spinner = Spinner::new();
        let mut stdout = io::stdout();
        spinner.tick(
            "OpenAnalyst is thinking...",
            TerminalRenderer::new().color_theme(),
            &mut stdout,
        )?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = self.runtime.run_turn(input, Some(&mut permission_prompter));
        match result {
            Ok(_) => {
                spinner.finish(
                    "OpenAnalyst responded",
                    TerminalRenderer::new().color_theme(),
                    &mut stdout,
                )?;
                println!();
                // Track usage per-provider per-day
                let usage = self.runtime.usage().cumulative_usage();
                record_session_usage(&self.model, &usage);
                self.persist_session()?;
                Ok(())
            }
            Err(error) => {
                spinner.fail(
                    "Request failed",
                    TerminalRenderer::new().color_theme(),
                    &mut stdout,
                )?;
                Err(Box::new(error))
            }
        }
    }

    fn run_turn_with_output(
        &mut self,
        input: &str,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match output_format {
            CliOutputFormat::Text => self.run_turn(input),
            CliOutputFormat::Json => self.run_prompt_json(input),
        }
    }

    fn run_prompt_json(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        let session = self.runtime.session().clone();
        let mut runtime = build_runtime(
            session,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            false,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let summary = runtime.run_turn(input, Some(&mut permission_prompter))?;
        self.runtime = runtime;
        self.persist_session()?;
        println!(
            "{}",
            json!({
                "message": final_assistant_text(&summary),
                "model": self.model,
                "iterations": summary.iterations,
                "tool_uses": collect_tool_uses(&summary),
                "tool_results": collect_tool_results(&summary),
                "usage": {
                    "input_tokens": summary.usage.input_tokens,
                    "output_tokens": summary.usage.output_tokens,
                    "cache_creation_input_tokens": summary.usage.cache_creation_input_tokens,
                    "cache_read_input_tokens": summary.usage.cache_read_input_tokens,
                }
            })
        );
        Ok(())
    }

    fn handle_repl_command(
        &mut self,
        command: SlashCommand,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(match command {
            SlashCommand::Help => {
                println!("{}", render_repl_help());
                false
            }
            SlashCommand::Status => {
                self.print_status();
                false
            }
            SlashCommand::Bughunter { scope } => {
                self.run_bughunter(scope.as_deref())?;
                false
            }
            SlashCommand::Commit => {
                self.run_commit()?;
                true
            }
            SlashCommand::Pr { context } => {
                self.run_pr(context.as_deref())?;
                false
            }
            SlashCommand::Issue { context } => {
                self.run_issue(context.as_deref())?;
                false
            }
            SlashCommand::Ultraplan { task } => {
                self.run_ultraplan(task.as_deref())?;
                false
            }
            SlashCommand::Teleport { target } => {
                self.run_teleport(target.as_deref())?;
                false
            }
            SlashCommand::DebugToolCall => {
                self.run_debug_tool_call()?;
                false
            }
            SlashCommand::Compact => {
                self.compact()?;
                false
            }
            SlashCommand::Model { model } => self.set_model(model)?,
            SlashCommand::Permissions { mode } => self.set_permissions(mode)?,
            SlashCommand::Clear { confirm } => self.clear_session(confirm)?,
            SlashCommand::Cost => {
                self.print_cost();
                false
            }
            SlashCommand::Resume { session_path } => self.resume_session(session_path)?,
            SlashCommand::Config { section } => {
                Self::print_config(section.as_deref())?;
                false
            }
            SlashCommand::Memory => {
                Self::print_memory()?;
                false
            }
            SlashCommand::Init => {
                run_init()?;
                false
            }
            SlashCommand::Diff => {
                Self::print_diff()?;
                false
            }
            SlashCommand::Version => {
                Self::print_version();
                false
            }
            SlashCommand::Export { path } => {
                self.export_session(path.as_deref())?;
                false
            }
            SlashCommand::Session { action, target } => {
                self.handle_session_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Plugins { action, target } => {
                self.handle_plugins_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Agents { args } => {
                Self::print_agents(args.as_deref())?;
                false
            }
            SlashCommand::Skills { args } => {
                Self::print_skills(args.as_deref())?;
                false
            }
            SlashCommand::Branch { .. } => {
                eprintln!("git branch commands not yet wired to REPL");
                false
            }
            SlashCommand::Worktree { .. } => {
                eprintln!("git worktree commands not yet wired to REPL");
                false
            }
            SlashCommand::CommitPushPr { .. } => {
                eprintln!("commit-push-pr not yet wired to REPL");
                false
            }
            // ── Multimedia & AI Commands ──
            SlashCommand::Image { prompt } => {
                self.run_image(prompt.as_deref())?;
                false
            }
            SlashCommand::Voice { file_path } => {
                self.run_voice(file_path.as_deref())?;
                false
            }
            SlashCommand::Speak { text } => {
                self.run_speak(text.as_deref())?;
                false
            }
            SlashCommand::Vision { image_path, prompt } => {
                self.run_vision(image_path.as_deref(), prompt.as_deref())?;
                false
            }
            SlashCommand::Diagram { description } => {
                self.run_diagram(description.as_deref())?;
                false
            }
            SlashCommand::Translate { language, text } => {
                self.run_translate(language.as_deref(), text.as_deref())?;
                false
            }
            SlashCommand::Tokens { target } => {
                self.run_tokens(target.as_deref())?;
                false
            }
            SlashCommand::DiffReview { file } => {
                self.run_diff_review(file.as_deref())?;
                false
            }
            SlashCommand::Scrape { url, selector } => {
                Self::run_scrape(url.as_deref(), selector.as_deref())?;
                false
            }
            SlashCommand::Json { url } => {
                Self::run_json(url.as_deref())?;
                false
            }
            SlashCommand::Dev { action, target } => {
                self.run_dev(action.as_deref(), target.as_deref())?;
                false
            }
            SlashCommand::Mcp { action, args } => {
                Self::run_mcp(action.as_deref(), args.as_deref())?;
                false
            }
            // ── Knowledge & Exploration commands ──
            SlashCommand::Knowledge { query } => {
                self.run_knowledge(query.as_deref())?;
                false
            }
            SlashCommand::Explore { target } => {
                self.run_explore(target.as_deref())?;
                false
            }
            // ── Claude Code parity commands ──
            SlashCommand::Doctor => {
                self.run_doctor()?;
                false
            }
            SlashCommand::Login => {
                println!("  Launching provider login...\n");
                drop(crossterm::terminal::disable_raw_mode());
                run_login()?;
                println!("\n  \x1b[2mRestart the CLI to use the new credentials.\x1b[0m");
                false
            }
            SlashCommand::Logout => {
                run_logout()?;
                println!("  \x1b[2mRestart the CLI for changes to take effect.\x1b[0m");
                false
            }
            SlashCommand::Vim => {
                println!("  Vim mode: toggle with Ctrl+V in the input editor.");
                false
            }
            SlashCommand::Think { prompt } => {
                let think_prompt = if let Some(p) = prompt {
                    format!(
                        "Use extended thinking for this task. Before answering:\n\
                         1. Identify all assumptions and constraints\n\
                         2. Consider multiple approaches and their trade-offs\n\
                         3. Reason through edge cases and failure modes\n\
                         4. Only then provide your well-reasoned answer\n\n{p}"
                    )
                } else {
                    "For the next prompt, use extended thinking — identify assumptions, consider alternatives, reason through edge cases, then answer.".to_string()
                };
                self.run_turn(&think_prompt)?;
                true
            }
            SlashCommand::Effort { category, level } => {
                match (category, level) {
                    (Some(cat), Some(lvl)) => println!("  Effort for {cat} set to: {lvl}"),
                    (None, Some(lvl)) => println!("  Effort set globally to: {lvl}"),
                    (Some(cat), None) => println!("  Showing config for: {cat}"),
                    (None, None) => println!("  Current effort: medium\n  Options: /effort [category] <level>\n  Categories: explore, research, code, write"),
                }
                false
            }
            SlashCommand::Route { .. } => {
                println!("  /route is only available in TUI mode.");
                false
            }
            SlashCommand::Context => {
                self.print_context();
                false
            }
            SlashCommand::Changelog { since } => {
                self.run_changelog(since.as_deref())?;
                false
            }
            SlashCommand::AddDir { path } => {
                self.run_add_dir(path.as_deref())?;
                false
            }
            SlashCommand::Exit => {
                true // signal quit
            }
            SlashCommand::Sidebar => {
                // Sidebar is TUI-only, no-op in legacy REPL
                false
            }
            SlashCommand::Swarm { .. } => {
                eprintln!("Swarm mode requires the TUI (remove --no-tui)");
                false
            }
            SlashCommand::OpenAnalyst { .. } => {
                eprintln!("Autonomous mode requires the TUI (remove --no-tui)");
                false
            }
            SlashCommand::Ask { question } => {
                if let Some(q) = question {
                    let prompt = format!(
                        "Answer this question directly and concisely. Do NOT use any tools:\n\n{q}"
                    );
                    self.run_turn(&prompt)?;
                } else {
                    eprintln!("Usage: /ask <question>");
                }
                false
            }
            SlashCommand::UserPrompt { prompt } => {
                if let Some(p) = prompt {
                    self.run_turn(&p)?;
                } else {
                    eprintln!("Usage: /user-prompt <message>");
                }
                false
            }
            SlashCommand::Hooks { .. } => {
                eprintln!("Use /hooks in the TUI mode for interactive hook management.");
                false
            }
            SlashCommand::Trust { .. } => {
                eprintln!("Use /trust in the TUI mode.");
                false
            }
            SlashCommand::Undo => {
                eprintln!("Use /undo in the TUI mode.");
                false
            }
            SlashCommand::Feedback { text } => {
                if let Some(t) = text {
                    eprintln!("Feedback recorded: {t}");
                } else {
                    eprintln!("Usage: /feedback <your correction or comment>");
                }
                false
            }
            SlashCommand::Unknown(name) => {
                eprintln!("unknown slash command: /{name}");
                false
            }
        })
    }

    fn persist_session(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.session().save_to_path(&self.session.path)?;
        Ok(())
    }

    fn print_status(&self) {
        let cumulative = self.runtime.usage().cumulative_usage();
        let latest = self.runtime.usage().current_turn_usage();
        println!(
            "{}",
            format_status_report(
                &self.model,
                StatusUsage {
                    message_count: self.runtime.session().messages.len(),
                    turns: self.runtime.usage().turns(),
                    latest,
                    cumulative,
                    estimated_tokens: self.runtime.estimated_tokens(),
                },
                self.permission_mode.as_str(),
                &status_context(Some(&self.session.path)).expect("status context should load"),
            )
        );
    }

    fn set_model(&mut self, model: Option<String>) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(model) = model else {
            println!(
                "{}",
                format_model_report(
                    &self.model,
                    self.runtime.session().messages.len(),
                    self.runtime.usage().turns(),
                )
            );
            // Show available models fetched live from each provider's API
            println!("\n  \x1b[1mAvailable models\x1b[0m \x1b[2m(fetching from providers...)\x1b[0m\n");
            let provider_configs: &[ProviderModelsConfig] = &[
                ProviderModelsConfig {
                    name: "OpenAnalyst",
                    keys: &["OPENANALYST_AUTH_TOKEN", "OPENANALYST_API_KEY"],
                    models_url: "https://api.openanalyst.com/api/ai/models",
                    auth_header: "bearer",
                },
                ProviderModelsConfig {
                    name: "Anthropic",
                    keys: &["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"],
                    models_url: "https://api.anthropic.com/v1/models",
                    auth_header: "x-api-key",
                },
                ProviderModelsConfig {
                    name: "OpenAI",
                    keys: &["OPENAI_API_KEY"],
                    models_url: "https://api.openai.com/v1/models",
                    auth_header: "bearer",
                },
                ProviderModelsConfig {
                    name: "xAI",
                    keys: &["XAI_API_KEY"],
                    models_url: "https://api.x.ai/v1/models",
                    auth_header: "bearer",
                },
                ProviderModelsConfig {
                    name: "OpenRouter",
                    keys: &["OPENROUTER_API_KEY"],
                    models_url: "https://openrouter.ai/api/v1/models",
                    auth_header: "bearer",
                },
                ProviderModelsConfig {
                    name: "Amazon Bedrock",
                    keys: &["BEDROCK_API_KEY"],
                    models_url: "",
                    auth_header: "bearer",
                },
                ProviderModelsConfig {
                    name: "Google Gemini",
                    keys: &["GEMINI_API_KEY"],
                    models_url: "https://generativelanguage.googleapis.com/v1beta/openai/models",
                    auth_header: "bearer",
                },
            ];
            for config in provider_configs {
                let api_key = config.keys.iter()
                    .find_map(|k| env::var(k).ok().filter(|v| !v.is_empty()));
                let has_key = api_key.is_some();
                let icon = if has_key { "\x1b[38;5;46m●\x1b[0m" } else { "\x1b[38;5;240m○\x1b[0m" };
                let name_style = if has_key { "\x1b[1m" } else { "\x1b[2m" };

                if !has_key {
                    println!("  {icon} {name_style}{}\x1b[0m \x1b[2m(run: openanalyst login)\x1b[0m", config.name);
                    continue;
                }

                // Fetch models from API
                let models = fetch_provider_models(
                    config.models_url,
                    api_key.as_deref().unwrap_or(""),
                    config.auth_header,
                );
                let model_list = if models.is_empty() {
                    "\x1b[2munable to fetch models\x1b[0m".to_string()
                } else {
                    let display: Vec<&str> = models.iter().map(String::as_str).take(8).collect();
                    let suffix = if models.len() > 8 {
                        format!(" \x1b[2m(+{} more)\x1b[0m", models.len() - 8)
                    } else {
                        String::new()
                    };
                    format!("\x1b[38;5;45m{}\x1b[0m{suffix}", display.join(", "))
                };
                println!("  {icon} {name_style}{}\x1b[0m", config.name);
                println!("    {model_list}");
            }
            println!("\n  \x1b[2mSwitch: /model <name>\x1b[0m");
            return Ok(false);
        };

        let model = resolve_model_alias(&model).to_string();

        if model == self.model {
            println!(
                "{}",
                format_model_report(
                    &self.model,
                    self.runtime.session().messages.len(),
                    self.runtime.usage().turns(),
                )
            );
            return Ok(false);
        }

        let previous = self.model.clone();
        let session = self.runtime.session().clone();
        let message_count = session.messages.len();

        // Detect the target provider for a clear error message
        let target_provider = api::detect_provider_kind(&model);

        match build_runtime(
            session.clone(),
            model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        ) {
            Ok(new_runtime) => {
                self.runtime = new_runtime;
                self.model.clone_from(&model);
                println!(
                    "{}",
                    format_model_switch_report(&previous, &model, message_count)
                );
                println!(
                    "  \x1b[2mProvider\x1b[0m         {}",
                    target_provider.display_name()
                );
                println!(
                    "  \x1b[2mSession\x1b[0m          \x1b[38;5;45mpersisted\x1b[0m ({message_count} messages carried over)"
                );
                Ok(true)
            }
            Err(error) => {
                // Restore original session — don't lose conversation
                println!(
                    "  \x1b[38;5;196mCannot switch to {model}\x1b[0m — {error}"
                );
                println!(
                    "  \x1b[2mProvider {} requires credentials.\x1b[0m",
                    target_provider.display_name()
                );
                println!(
                    "  Run \x1b[1mopenanalyst login\x1b[0m and add {} credentials.",
                    target_provider.display_name()
                );
                println!(
                    "  \x1b[2mCurrent model unchanged:\x1b[0m {}",
                    self.model
                );
                Ok(false)
            }
        }
    }

    fn set_permissions(
        &mut self,
        mode: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(mode) = mode else {
            println!(
                "{}",
                format_permissions_report(self.permission_mode.as_str())
            );
            return Ok(false);
        };

        let normalized = normalize_permission_mode(&mode).ok_or_else(|| {
            format!(
                "unsupported permission mode '{mode}'. Use read-only, workspace-write, or danger-full-access."
            )
        })?;

        if normalized == self.permission_mode.as_str() {
            println!("{}", format_permissions_report(normalized));
            return Ok(false);
        }

        let previous = self.permission_mode.as_str().to_string();
        let session = self.runtime.session().clone();
        self.permission_mode = permission_mode_from_label(normalized);
        self.runtime = build_runtime(
            session,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        println!(
            "{}",
            format_permissions_switch_report(&previous, normalized)
        );
        Ok(true)
    }

    fn clear_session(&mut self, confirm: bool) -> Result<bool, Box<dyn std::error::Error>> {
        if !confirm {
            println!(
                "clear: confirmation required; run /clear --confirm to start a fresh session."
            );
            return Ok(false);
        }

        self.session = create_managed_session_handle()?;
        self.runtime = build_runtime(
            Session::new(),
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        println!(
            "Session cleared\n  Mode             fresh session\n  Preserved model  {}\n  Permission mode  {}\n  Session          {}",
            self.model,
            self.permission_mode.as_str(),
            self.session.id,
        );
        Ok(true)
    }

    fn print_cost(&self) {
        let cumulative = self.runtime.usage().cumulative_usage();
        println!("{}", format_cost_report(cumulative));
    }

    fn resume_session(
        &mut self,
        session_path: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(session_ref) = session_path else {
            println!("Usage: /resume <session-path>");
            return Ok(false);
        };

        let handle = resolve_session_reference(&session_ref)?;
        let session = Session::load_from_path(&handle.path)?;
        let message_count = session.messages.len();
        self.runtime = build_runtime(
            session,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.session = handle;
        println!(
            "{}",
            format_resume_report(
                &self.session.path.display().to_string(),
                message_count,
                self.runtime.usage().turns(),
            )
        );
        Ok(true)
    }

    fn print_config(section: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_config_report(section)?);
        Ok(())
    }

    fn print_memory() -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_memory_report()?);
        Ok(())
    }

    fn print_agents(args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        println!("{}", handle_agents_slash_command(args, &cwd)?);
        Ok(())
    }

    fn print_skills(args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        println!("{}", handle_skills_slash_command(args, &cwd)?);
        Ok(())
    }

    fn print_diff() -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_diff_report()?);
        Ok(())
    }

    fn print_version() {
        println!("{}", render_version_report());
    }

    fn export_session(
        &self,
        requested_path: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let export_path = resolve_export_path(requested_path, self.runtime.session())?;
        fs::write(&export_path, render_export_text(self.runtime.session()))?;
        println!(
            "Export\n  Result           wrote transcript\n  File             {}\n  Messages         {}",
            export_path.display(),
            self.runtime.session().messages.len(),
        );
        Ok(())
    }

    fn handle_session_command(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match action {
            None | Some("list") => {
                println!("{}", render_session_list(&self.session.id)?);
                Ok(false)
            }
            Some("switch") => {
                let Some(target) = target else {
                    println!("Usage: /session switch <session-id>");
                    return Ok(false);
                };
                let handle = resolve_session_reference(target)?;
                let session = Session::load_from_path(&handle.path)?;
                let message_count = session.messages.len();
                self.runtime = build_runtime(
                    session,
                    self.model.clone(),
                    self.system_prompt.clone(),
                    true,
                    true,
                    self.allowed_tools.clone(),
                    self.permission_mode,
                    None,
                )?;
                self.session = handle;
                println!(
                    "Session switched\n  Active session   {}\n  File             {}\n  Messages         {}",
                    self.session.id,
                    self.session.path.display(),
                    message_count,
                );
                Ok(true)
            }
            Some(other) => {
                println!("Unknown /session action '{other}'. Use /session list or /session switch <session-id>.");
                Ok(false)
            }
        }
    }

    fn handle_plugins_command(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        let loader = ConfigLoader::default_for(&cwd);
        let runtime_config = loader.load()?;
        let mut manager = build_plugin_manager(&cwd, &loader, &runtime_config);
        let result = handle_plugins_slash_command(action, target, &mut manager)?;
        println!("{}", result.message);
        if result.reload_runtime {
            self.reload_runtime_features()?;
        }
        Ok(false)
    }

    fn reload_runtime_features(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime = build_runtime(
            self.runtime.session().clone(),
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.persist_session()
    }

    fn compact(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.runtime.compact(CompactionConfig::default());
        let removed = result.removed_message_count;
        let kept = result.compacted_session.messages.len();
        let skipped = removed == 0;
        self.runtime = build_runtime(
            result.compacted_session,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.persist_session()?;
        println!("{}", format_compact_report(removed, kept, skipped));
        Ok(())
    }

    fn run_internal_prompt_text_with_progress(
        &self,
        prompt: &str,
        enable_tools: bool,
        progress: Option<InternalPromptProgressReporter>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let session = self.runtime.session().clone();
        let mut runtime = build_runtime(
            session,
            self.model.clone(),
            self.system_prompt.clone(),
            enable_tools,
            false,
            self.allowed_tools.clone(),
            self.permission_mode,
            progress,
        )?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let summary = runtime.run_turn(prompt, Some(&mut permission_prompter))?;
        Ok(final_assistant_text(&summary).trim().to_string())
    }

    fn run_internal_prompt_text(
        &self,
        prompt: &str,
        enable_tools: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.run_internal_prompt_text_with_progress(prompt, enable_tools, None)
    }

    fn run_bughunter(&self, scope: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let scope = scope.unwrap_or("the current repository");
        let prompt = format!(
            "You are /bughunter. Inspect {scope} and identify the most likely bugs or correctness issues. Prioritize concrete findings with file paths, severity, and suggested fixes. Use tools if needed."
        );
        println!("{}", self.run_internal_prompt_text(&prompt, true)?);
        Ok(())
    }

    fn run_ultraplan(&self, task: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let task = task.unwrap_or("the current repo work");
        let prompt = format!(
            "You are /ultraplan. Produce a deep multi-step execution plan for {task}. Include goals, risks, implementation sequence, verification steps, and rollback considerations. Use tools if needed."
        );
        let mut progress = InternalPromptProgressRun::start_ultraplan(task);
        match self.run_internal_prompt_text_with_progress(&prompt, true, Some(progress.reporter()))
        {
            Ok(plan) => {
                progress.finish_success();
                println!("{plan}");
                Ok(())
            }
            Err(error) => {
                progress.finish_failure(&error.to_string());
                Err(error)
            }
        }
    }

    #[allow(clippy::unused_self)]
    fn run_teleport(&self, target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(target) = target.map(str::trim).filter(|value| !value.is_empty()) else {
            println!("Usage: /teleport <symbol-or-path>");
            return Ok(());
        };

        println!("{}", render_teleport_report(target)?);
        Ok(())
    }

    fn run_debug_tool_call(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_last_tool_debug_report(self.runtime.session())?);
        Ok(())
    }

    fn run_commit(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let status = git_output(&["status", "--short"])?;
        if status.trim().is_empty() {
            println!("Commit\n  Result           skipped\n  Reason           no workspace changes");
            return Ok(());
        }

        git_status_ok(&["add", "-A"])?;
        let staged_stat = git_output(&["diff", "--cached", "--stat"])?;
        let prompt = format!(
            "Generate a git commit message in plain text Lore format only. Base it on this staged diff summary:\n\n{}\n\nRecent conversation context:\n{}",
            truncate_for_prompt(&staged_stat, 8_000),
            recent_user_context(self.runtime.session(), 6)
        );
        let message = sanitize_generated_message(&self.run_internal_prompt_text(&prompt, false)?);
        if message.trim().is_empty() {
            return Err("generated commit message was empty".into());
        }

        let path = write_temp_text_file("openanalyst-commit-message.txt", &message)?;
        let output = Command::new("git")
            .args(["commit", "--file"])
            .arg(&path)
            .current_dir(env::current_dir()?)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("git commit failed: {stderr}").into());
        }

        println!(
            "Commit\n  Result           created\n  Message file     {}\n\n{}",
            path.display(),
            message.trim()
        );
        Ok(())
    }

    fn run_pr(&self, context: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let staged = git_output(&["diff", "--stat"])?;
        let prompt = format!(
            "Generate a pull request title and body from this conversation and diff summary. Output plain text in this format exactly:\nTITLE: <title>\nBODY:\n<body markdown>\n\nContext hint: {}\n\nDiff summary:\n{}",
            context.unwrap_or("none"),
            truncate_for_prompt(&staged, 10_000)
        );
        let draft = sanitize_generated_message(&self.run_internal_prompt_text(&prompt, false)?);
        let (title, body) = parse_titled_body(&draft)
            .ok_or_else(|| "failed to parse generated PR title/body".to_string())?;

        if command_exists("gh") {
            let body_path = write_temp_text_file("openanalyst-pr-body.md", &body)?;
            let output = Command::new("gh")
                .args(["pr", "create", "--title", &title, "--body-file"])
                .arg(&body_path)
                .current_dir(env::current_dir()?)
                .output()?;
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!(
                    "PR\n  Result           created\n  Title            {title}\n  URL              {}",
                    if stdout.is_empty() { "<unknown>" } else { &stdout }
                );
                return Ok(());
            }
        }

        println!("PR draft\n  Title            {title}\n\n{body}");
        Ok(())
    }

    fn run_issue(&self, context: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let prompt = format!(
            "Generate a GitHub issue title and body from this conversation. Output plain text in this format exactly:\nTITLE: <title>\nBODY:\n<body markdown>\n\nContext hint: {}\n\nConversation context:\n{}",
            context.unwrap_or("none"),
            truncate_for_prompt(&recent_user_context(self.runtime.session(), 10), 10_000)
        );
        let draft = sanitize_generated_message(&self.run_internal_prompt_text(&prompt, false)?);
        let (title, body) = parse_titled_body(&draft)
            .ok_or_else(|| "failed to parse generated issue title/body".to_string())?;

        if command_exists("gh") {
            let body_path = write_temp_text_file("openanalyst-issue-body.md", &body)?;
            let output = Command::new("gh")
                .args(["issue", "create", "--title", &title, "--body-file"])
                .arg(&body_path)
                .current_dir(env::current_dir()?)
                .output()?;
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!(
                    "Issue\n  Result           created\n  Title            {title}\n  URL              {}",
                    if stdout.is_empty() { "<unknown>" } else { &stdout }
                );
                return Ok(());
            }
        }

        println!("Issue draft\n  Title            {title}\n\n{body}");
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    //  Multimedia & AI Slash Commands
    // ════════════════════════════════════════════════════════════════════

    fn run_image(&self, prompt: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(prompt) = prompt else {
            println!("Usage: /image <prompt>\n  Example: /image a sunset over mountains in watercolor style");
            return Ok(());
        };
        // Try providers in order: Gemini Imagen, OpenAI DALL-E, Stability
        let (provider, api_key, url, request_body) =
            if let Some(key) = env::var("GEMINI_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
                ("Gemini Imagen", key.clone(),
                 format!("https://generativelanguage.googleapis.com/v1beta/models/imagen-3.0-generate-002:predict?key={key}"),
                 json!({
                     "instances": [{"prompt": prompt}],
                     "parameters": {"sampleCount": 1, "aspectRatio": "1:1"}
                 }))
            } else if let Some(key) = env::var("OPENAI_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
                ("DALL-E 3", key,
                 "https://api.openai.com/v1/images/generations".to_string(),
                 json!({
                     "model": "dall-e-3",
                     "prompt": prompt,
                     "n": 1,
                     "size": "1024x1024",
                     "response_format": "b64_json"
                 }))
            } else if let Some(key) = env::var("STABILITY_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
                ("Stability AI", key,
                 "https://api.stability.ai/v2beta/stable-image/generate/sd3".to_string(),
                 json!({ "prompt": prompt, "output_format": "png" }))
            } else {
                println!("No image generation API key found.\n  Set GEMINI_API_KEY, OPENAI_API_KEY, or STABILITY_API_KEY.");
                return Ok(());
            };

        println!("  Generating image via {provider}...");
        let rt = tokio::runtime::Runtime::new()?;
        let result: Result<serde_json::Value, Box<dyn std::error::Error>> = rt.block_on(async {
            let client = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
            let mut req = client.post(&url).json(&request_body);
            if provider != "Gemini Imagen" {
                req = req.bearer_auth(&api_key);
            }
            let resp = req.send().await?;
            let status = resp.status();
            let body: serde_json::Value = resp.json().await?;
            if !status.is_success() {
                return Err(format!("API error {status}: {body}").into());
            }
            Ok(body)
        });

        match result {
            Ok(body) => {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                let filename = format!("generated-{timestamp}.png");
                // Try to extract base64 image from response
                // Extract base64 image from response — try all known provider formats:
                // OpenAI DALL-E: data[0].b64_json or data[0].url
                // Stability AI: artifacts[0].base64
                // Gemini Imagen: predictions[0].bytesBase64Encoded
                let b64 = body.pointer("/data/0/b64_json")
                    .or_else(|| body.pointer("/artifacts/0/base64"))
                    .or_else(|| body.pointer("/predictions/0/bytesBase64Encoded"))
                    .and_then(|v| v.as_str());
                if let Some(b64_data) = b64 {
                    use std::io::Write as _;
                    let decoded = base64_decode(b64_data);
                    let mut file = fs::File::create(&filename)?;
                    file.write_all(&decoded)?;
                    println!("  Image saved: {filename} ({} bytes)", decoded.len());
                } else if let Some(img_url) = body.pointer("/data/0/url").and_then(|v| v.as_str()) {
                    println!("  Image URL: {img_url}");
                } else {
                    println!("  Response: {}", serde_json::to_string_pretty(&body)?);
                }
            }
            Err(e) => println!("  Image generation failed: {e}"),
        }
        Ok(())
    }

    fn run_voice(&self, file_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(file_path) = file_path else {
            println!("Usage: /voice <audio-file>\n  Supported: mp3, wav, m4a, webm, mp4, ogg, flac");
            return Ok(());
        };
        if !Path::new(file_path).exists() {
            println!("  File not found: {file_path}");
            return Ok(());
        }
        let openai_key = env::var("OPENAI_API_KEY").ok().filter(|k| !k.trim().is_empty());
        let gemini_key = env::var("GEMINI_API_KEY").ok().filter(|k| !k.trim().is_empty());
        let api_key = openai_key.clone().or(gemini_key.clone());
        let Some(api_key) = api_key else {
            println!("No transcription API key found.\n  Set OPENAI_API_KEY (Whisper) or GEMINI_API_KEY.");
            return Ok(());
        };

        let use_openai = openai_key.is_some();
        let provider = if use_openai { "OpenAI Whisper" } else { "Gemini" };
        println!("  Transcribing via {provider}...");

        let file_bytes = fs::read(file_path)?;
        let file_name = Path::new(file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let mime = mime_from_extension(file_path);

        let rt = tokio::runtime::Runtime::new()?;
        let result: Result<String, Box<dyn std::error::Error>> = rt.block_on(async {
            let client = reqwest::Client::builder().timeout(Duration::from_secs(120)).build()?;
            if use_openai {
                let part = reqwest::multipart::Part::bytes(file_bytes)
                    .file_name(file_name)
                    .mime_str(mime)?;
                let form = reqwest::multipart::Form::new()
                    .text("model", "whisper-1")
                    .text("response_format", "text")
                    .part("file", part);
                let resp = client.post("https://api.openai.com/v1/audio/transcriptions")
                    .bearer_auth(&api_key)
                    .multipart(form)
                    .send().await?;
                let status = resp.status();
                let text = resp.text().await?;
                if !status.is_success() {
                    return Err(format!("Whisper API error {status}: {text}").into());
                }
                Ok(text)
            } else {
                let b64 = base64_encode(&file_bytes);
                let body = json!({
                    "contents": [{"parts": [
                        {"inlineData": {"mimeType": mime, "data": b64}},
                        {"text": "Transcribe this audio accurately. Return only the transcription text."}
                    ]}]
                });
                let resp = client.post(format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={api_key}"
                )).json(&body).send().await?;
                let status = resp.status();
                let result: serde_json::Value = resp.json().await?;
                if !status.is_success() {
                    return Err(format!("Gemini API error {status}: {result}").into());
                }
                let text = result.pointer("/candidates/0/content/parts/0/text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no transcription)")
                    .to_string();
                Ok(text)
            }
        });

        match result {
            Ok(transcript) => {
                println!("\n  Transcript:\n  {}", transcript.trim().replace('\n', "\n  "));
            }
            Err(e) => println!("  Transcription failed: {e}"),
        }
        Ok(())
    }

    fn run_speak(&self, text: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(text) = text else {
            println!("Usage: /speak <text>\n  Example: /speak Hello, this is OpenAnalyst speaking.");
            return Ok(());
        };
        let api_key = env::var("OPENAI_API_KEY").map(|k| k.trim().to_string()).ok().filter(|k| !k.is_empty());
        let Some(api_key) = api_key else {
            println!("No TTS API key found.\n  Set OPENAI_API_KEY for text-to-speech.");
            return Ok(());
        };

        println!("  Generating speech via OpenAI TTS...");
        let rt = tokio::runtime::Runtime::new()?;
        let result: Result<Vec<u8>, Box<dyn std::error::Error>> = rt.block_on(async {
            let client = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
            let resp = client.post("https://api.openai.com/v1/audio/speech")
                .bearer_auth(&api_key)
                .json(&json!({
                    "model": "tts-1",
                    "input": text,
                    "voice": "alloy",
                    "response_format": "mp3"
                }))
                .send().await?;
            let status = resp.status();
            if !status.is_success() {
                let error_text = resp.text().await?;
                return Err(format!("TTS API error {status}: {error_text}").into());
            }
            let bytes = resp.bytes().await?.to_vec();
            Ok(bytes)
        });

        match result {
            Ok(bytes) => {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                let filename = format!("speech-{timestamp}.mp3");
                fs::write(&filename, &bytes)?;
                println!("  Audio saved: {filename} ({} bytes)", bytes.len());
            }
            Err(e) => println!("  Speech generation failed: {e}"),
        }
        Ok(())
    }

    fn run_vision(
        &self,
        image_path: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(image_path) = image_path else {
            println!("Usage: /vision <image-path> [prompt]\n  Example: /vision screenshot.png What does this show?");
            return Ok(());
        };
        if !Path::new(image_path).exists() {
            println!("  File not found: {image_path}");
            return Ok(());
        }
        let prompt = prompt.unwrap_or("Describe this image in detail.");
        let file_bytes = fs::read(image_path)?;
        let b64 = base64_encode(&file_bytes);
        let mime = mime_from_extension(image_path);

        // Try providers: OpenAnalyst (default) → Gemini → OpenAI → Anthropic → Grok
        let (provider, api_key) =
            if let Some(key) = env::var("GEMINI_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
                ("Gemini", key)
            } else if let Some(key) = env::var("OPENAI_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
                ("GPT-4o", key)
            } else if let Some(key) = env::var("ANTHROPIC_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
                ("Claude", key)
            } else if let Some(key) = env::var("XAI_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
                ("Grok", key)
            } else {
                // Fallback: use OpenAnalyst gateway (no key required)
                ("OpenAnalyst", String::new())
            };

        println!("  Analyzing image via {provider}...");
        let rt = tokio::runtime::Runtime::new()?;
        let result: Result<String, Box<dyn std::error::Error>> = rt.block_on(async {
            let client = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
            match provider {
                "Gemini" => {
                    let body = json!({
                        "contents": [{"parts": [
                            {"inlineData": {"mimeType": mime, "data": b64}},
                            {"text": prompt}
                        ]}]
                    });
                    let resp = client.post(format!(
                        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={api_key}"
                    )).json(&body).send().await?;
                    let result: serde_json::Value = resp.json().await?;
                    Ok(result.pointer("/candidates/0/content/parts/0/text")
                        .and_then(|v| v.as_str()).unwrap_or("(no response)").to_string())
                }
                "GPT-4o" => {
                    let body = json!({
                        "model": "gpt-4o",
                        "messages": [{"role": "user", "content": [
                            {"type": "image_url", "image_url": {"url": format!("data:{mime};base64,{b64}")}},
                            {"type": "text", "text": prompt}
                        ]}],
                        "max_tokens": 1024
                    });
                    let resp = client.post("https://api.openai.com/v1/chat/completions")
                        .bearer_auth(&api_key).json(&body).send().await?;
                    let result: serde_json::Value = resp.json().await?;
                    Ok(result.pointer("/choices/0/message/content")
                        .and_then(|v| v.as_str()).unwrap_or("(no response)").to_string())
                }
                "Grok" => {
                    // xAI Grok uses OpenAI-compatible vision format
                    let body = json!({
                        "model": "grok-3",
                        "messages": [{"role": "user", "content": [
                            {"type": "image_url", "image_url": {"url": format!("data:{mime};base64,{b64}")}},
                            {"type": "text", "text": prompt}
                        ]}],
                        "max_tokens": 1024
                    });
                    let resp = client.post("https://api.x.ai/v1/chat/completions")
                        .bearer_auth(&api_key).json(&body).send().await?;
                    let result: serde_json::Value = resp.json().await?;
                    Ok(result.pointer("/choices/0/message/content")
                        .and_then(|v| v.as_str()).unwrap_or("(no response)").to_string())
                }
                "Claude" => {
                    let body = json!({
                        "model": "claude-sonnet-4-6",
                        "max_tokens": 1024,
                        "messages": [{"role": "user", "content": [
                            {"type": "image", "source": {"type": "base64", "media_type": mime, "data": b64}},
                            {"type": "text", "text": prompt}
                        ]}]
                    });
                    let resp = client.post("https://api.anthropic.com/v1/messages")
                        .header("x-api-key", &api_key)
                        .header("anthropic-version", "2023-06-01")
                        .json(&body).send().await?;
                    let result: serde_json::Value = resp.json().await?;
                    Ok(result.pointer("/content/0/text")
                        .and_then(|v| v.as_str()).unwrap_or("(no response)").to_string())
                }
                _ => {
                    // OpenAnalyst model server (OpenAI-compat format)
                    let oa_base = env::var("OPENANALYST_BASE_URL")
                        .unwrap_or_else(|_| "https://api.openanalyst.com".to_string());
                    let body = json!({
                        "model": "openai/gpt-oss-120b",
                        "messages": [{"role": "user", "content": [
                            {"type": "image_url", "image_url": {"url": format!("data:{mime};base64,{b64}")}},
                            {"type": "text", "text": prompt}
                        ]}],
                        "max_tokens": 1024
                    });
                    let resp = client.post(format!("{}/v1/chat/completions", oa_base.trim_end_matches('/')))
                        .json(&body).send().await?;
                    let result: serde_json::Value = resp.json().await?;
                    Ok(result.pointer("/choices/0/message/content")
                        .and_then(|v| v.as_str()).unwrap_or("(no response)").to_string())
                }
            }
        });

        match result {
            Ok(description) => println!("\n{description}"),
            Err(e) => println!("  Vision analysis failed: {e}"),
        }
        Ok(())
    }

    fn run_diagram(&mut self, description: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(description) = description else {
            println!("Usage: /diagram <description>\n  Example: /diagram user authentication flow with JWT");
            return Ok(());
        };
        let diagram_prompt = format!(
            "Generate a Mermaid diagram for: {description}\n\n\
             Requirements:\n\
             1. Choose the best diagram type (graph TD, sequenceDiagram, classDiagram, erDiagram, stateDiagram-v2, flowchart, gantt, pie, mindmap)\n\
             2. Use clear, descriptive node labels — never single letters\n\
             3. Show all meaningful relationships and data flows\n\
             4. Return ONLY a ```mermaid code block, no explanation before or after"
        );
        self.run_turn(&diagram_prompt)?;

        // Save the last assistant response as .mmd file if it contains mermaid
        let last_text = self.runtime.session().messages.iter().rev()
            .find(|m| m.role == MessageRole::Assistant)
            .map(|m| m.blocks.iter().filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            }).collect::<Vec<_>>().join(""))
            .unwrap_or_default();

        if last_text.contains("graph") || last_text.contains("Diagram") || last_text.contains("sequenceDiagram") || last_text.contains("classDiagram") {
            let mermaid_code = last_text
                .lines()
                .skip_while(|l| !l.starts_with("```"))
                .skip(1)
                .take_while(|l| !l.starts_with("```"))
                .collect::<Vec<_>>()
                .join("\n");
            if !mermaid_code.is_empty() {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                let filename = format!("diagram-{timestamp}.mmd");
                fs::write(&filename, &mermaid_code)?;
                println!("\n  Diagram saved: {filename}");
            }
        }
        Ok(())
    }

    fn run_translate(
        &mut self,
        language: Option<&str>,
        text: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(language) = language else {
            println!("Usage: /translate <language> <text>\n  Example: /translate Spanish Hello, how are you?");
            return Ok(());
        };
        let Some(text) = text else {
            println!("Usage: /translate {language} <text>");
            return Ok(());
        };
        let translate_prompt = format!(
            "Translate the following text to {language}.\n\n\
             Rules:\n\
             - Return ONLY the translated text, nothing else\n\
             - Preserve formatting, line breaks, markdown, and code blocks\n\
             - Use natural, fluent {language} — not literal word-by-word translation\n\
             - Keep technical terms, brand names, and code identifiers untranslated\n\n\
             Text to translate:\n{text}"
        );
        self.run_turn(&translate_prompt)
    }

    fn run_tokens(&self, target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let (content, source) = match target {
            Some(path) if Path::new(path).exists() => {
                (fs::read_to_string(path)?, path.to_string())
            }
            Some(text) => (text.to_string(), "(inline text)".to_string()),
            None => {
                println!("Usage: /tokens <file-path or text>\n  Example: /tokens src/main.rs\n  Example: /tokens \"Hello world\"");
                return Ok(());
            }
        };
        let char_count = content.len();
        let word_count = content.split_whitespace().count();
        let line_count = content.lines().count();
        // Heuristic: code ~3.5 chars/token, prose ~4.5 chars/token
        let code_ratio = content.chars().filter(|c| matches!(c, '{' | '}' | '(' | ')' | ';' | ':' | '.' | '#')).count();
        let is_code = code_ratio > char_count / 40;
        let chars_per_token = if is_code { 3.5_f64 } else { 4.5 };
        let estimated_tokens = (char_count as f64 / chars_per_token).ceil() as usize;
        println!(
            "Token estimate\n  Source           {source}\n  Characters       {char_count}\n  Words            {word_count}\n  Lines            {line_count}\n  Content type     {content_type}\n  Est. tokens      ~{estimated_tokens}\n  Model            {model}",
            content_type = if is_code { "code" } else { "prose" },
            model = self.model
        );
        Ok(())
    }

    fn run_diff_review(&mut self, file: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let diff = if let Some(file) = file {
            let output = std::process::Command::new("git")
                .args(["diff", "--", file])
                .output()?;
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            let output = std::process::Command::new("git")
                .args(["diff"])
                .output()?;
            String::from_utf8_lossy(&output.stdout).to_string()
        };
        if diff.trim().is_empty() {
            println!("  No changes to review (git diff is empty).");
            return Ok(());
        }
        let review_prompt = format!(
            "You are a senior code reviewer. Review this git diff with a critical eye.\n\n\
             Check for:\n\
             1. **Bugs** — logic errors, off-by-one, null/undefined access, race conditions\n\
             2. **Security** — injection, XSS, hardcoded secrets, unsafe deserialization\n\
             3. **Performance** — unnecessary allocations, O(n^2) loops, missing caching\n\
             4. **Error handling** — swallowed errors, missing edge cases, panic paths\n\
             5. **API contract** — breaking changes, missing validation, wrong HTTP methods\n\n\
             For each issue found: state the file, line, severity (critical/warning/info), and fix.\n\
             If the changes are clean, say \"LGTM\" with a brief summary of what was changed.\n\n\
             ```diff\n{diff}\n```"
        );
        self.run_turn(&review_prompt)
    }

    fn run_scrape(url: Option<&str>, selector: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(url) = url else {
            println!("Usage: /scrape <url> [css-selector]\n  Example: /scrape https://example.com h1");
            return Ok(());
        };
        println!("  Fetching {url}...");
        let rt = tokio::runtime::Runtime::new()?;
        let result = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent("openanalyst-cli/1.0")
                .redirect(reqwest::redirect::Policy::limited(10))
                .build()?;
            let resp = client.get(url).send().await?;
            let status = resp.status();
            let body = resp.text().await?;
            Ok::<_, Box<dyn std::error::Error>>((status, body))
        });

        match result {
            Ok((status, body)) => {
                println!("  Status: {status}");
                if let Some(sel) = selector {
                    // Simple CSS selector extraction (tag name matching)
                    let tag = sel.trim_start_matches('.');
                    let extracted: Vec<String> = body.split(&format!("<{tag}"))
                        .skip(1)
                        .filter_map(|part| {
                            let inner = part.split('>').nth(1)?;
                            let text = inner.split(&format!("</{tag}")).next()?;
                            let clean = strip_html_tags(text);
                            (!clean.trim().is_empty()).then(|| clean.trim().to_string())
                        })
                        .take(20)
                        .collect();
                    if extracted.is_empty() {
                        println!("  No matches for selector: {sel}");
                    } else {
                        for (i, item) in extracted.iter().enumerate() {
                            println!("  [{i}] {item}");
                        }
                    }
                } else {
                    // Full text extraction
                    let text = strip_html_tags(&body);
                    let preview: String = text.lines()
                        .filter(|l| !l.trim().is_empty())
                        .take(50)
                        .collect::<Vec<_>>()
                        .join("\n");
                    println!("{preview}");
                }
            }
            Err(e) => println!("  Fetch failed: {e}"),
        }
        Ok(())
    }

    fn run_json(url: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(url) = url else {
            println!("Usage: /json <url>\n  Example: /json https://api.github.com/repos/rust-lang/rust");
            return Ok(());
        };
        println!("  Fetching {url}...");
        let rt = tokio::runtime::Runtime::new()?;
        let result = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent("openanalyst-cli/1.0")
                .build()?;
            let resp = client.get(url)
                .header("Accept", "application/json")
                .send().await?;
            let status = resp.status();
            let body: serde_json::Value = resp.json().await?;
            Ok::<_, Box<dyn std::error::Error>>((status, body))
        });

        match result {
            Ok((status, body)) => {
                println!("  Status: {status}\n");
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
            Err(e) => println!("  Fetch failed: {e}"),
        }
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    //  /dev — Playwright Browser Automation (MCP > CLI > Direct)
    // ════════════════════════════════════════════════════════════════════

    fn run_dev(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            None => {
                println!(
                    "Usage: /dev <action> [target]\n\n\
                     \x1b[1mSetup:\x1b[0m\n\
                     \x1b[1m  install\x1b[0m              Install latest Playwright + Chromium + MCP server\n\
                     \x1b[1m  status\x1b[0m               Show Playwright & MCP server status\n\n\
                     \x1b[1mBrowser (via Playwright CLI):\x1b[0m\n\
                     \x1b[1m  open <url>\x1b[0m            Open URL in headed browser with inspector\n\
                     \x1b[1m  screenshot <url> [file]\x1b[0m  Capture screenshot of URL\n\
                     \x1b[1m  codegen [url]\x1b[0m         Record actions and generate test code\n\n\
                     \x1b[1mAI Testing:\x1b[0m\n\
                     \x1b[1m  test <description>\x1b[0m    AI generates & runs a Playwright test\n\n\
                     \x1b[1mMCP (accessibility tree automation):\x1b[0m\n\
                     Once installed, the Playwright MCP server provides 30 browser\n\
                     tools (browser_navigate, browser_click, browser_snapshot, etc.)\n\
                     that the AI agent can call directly using the accessibility tree.\n\
                     Configure via: /mcp add playwright npx @playwright/mcp@latest\n\n\
                     Aliases: /browser, /playwright"
                );
                Ok(())
            }
            Some("install") => Self::dev_install(),
            Some("status") => Self::dev_status(),
            Some("open") => Self::dev_open(target),
            Some("screenshot") => Self::dev_screenshot(target),
            Some("codegen") => Self::dev_codegen(target),
            Some("test") => self.dev_test(target),
            Some(other) => {
                if other.starts_with("http://") || other.starts_with("https://") {
                    Self::dev_open(Some(other))
                } else {
                    println!("Unknown /dev action: {other}\n  Run /dev for usage.");
                    Ok(())
                }
            }
        }
    }

    fn dev_install() -> Result<(), Box<dyn std::error::Error>> {
        println!("  \x1b[1mStep 1/3:\x1b[0m Installing Playwright (latest)...");
        let npm_result = Command::new("npm")
            .args(["install", "-g", "playwright@latest", "@playwright/mcp@latest"])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status();
        match npm_result {
            Ok(s) if s.success() => {}
            Ok(_) | Err(_) => {
                println!("  npm global install failed. Packages will be used via npx.");
            }
        }

        println!("  \x1b[1mStep 2/3:\x1b[0m Installing Chromium browser...");
        let _ = Command::new("npx")
            .args(["playwright", "install", "chromium"])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status();

        println!("  \x1b[1mStep 3/3:\x1b[0m Configuring Playwright MCP server...");
        Self::dev_configure_mcp()?;

        println!("\n  \x1b[38;5;46mPlaywright ready.\x1b[0m");
        println!("  The AI agent can now use browser tools via the accessibility tree.");
        println!("  Quick actions: /dev open <url>, /dev screenshot <url>, /dev codegen");
        println!("  AI automation: The agent calls browser_navigate, browser_click, etc.");
        println!("\n  \x1b[2mRestart the CLI to activate the MCP server.\x1b[0m");
        Ok(())
    }

    fn dev_configure_mcp() -> Result<(), Box<dyn std::error::Error>> {
        // Add @playwright/mcp to .openanalyst/settings.json
        let cwd = env::current_dir()?;
        let settings_dir = cwd.join(".openanalyst");
        fs::create_dir_all(&settings_dir)?;
        let settings_path = settings_dir.join("settings.json");

        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        // Add playwright MCP server config
        if settings.get("mcpServers").is_none() {
            settings["mcpServers"] = json!({});
        }
        settings["mcpServers"]["playwright"] = json!({
            "command": "npx",
            "args": ["@playwright/mcp@latest"]
        });

        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        println!("  MCP server configured in {}", settings_path.display());
        Ok(())
    }

    fn dev_status() -> Result<(), Box<dyn std::error::Error>> {
        // Playwright version
        let pw_version = Command::new("npx")
            .args(["playwright", "--version"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        // MCP server version
        let mcp_version = Command::new("npx")
            .args(["@playwright/mcp@latest", "--version"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        println!("  Playwright:  {}", pw_version.as_deref().unwrap_or("not installed"));
        println!("  MCP server:  {}", mcp_version.as_deref().unwrap_or("not installed"));

        // Check MCP config
        let cwd = env::current_dir()?;
        let settings_path = cwd.join(".openanalyst").join("settings.json");
        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            let settings: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
            if settings.pointer("/mcpServers/playwright").is_some() {
                println!("  MCP config:  \x1b[38;5;46mconfigured\x1b[0m");
            } else {
                println!("  MCP config:  not configured (run /dev install)");
            }
        } else {
            println!("  MCP config:  no settings.json found");
        }

        if pw_version.is_none() {
            println!("\n  Run \x1b[1m/dev install\x1b[0m to set up Playwright.");
        }
        Ok(())
    }

    fn dev_open(url: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(url) = url else {
            println!("Usage: /dev open <url>\n  Opens a headed browser with Playwright inspector.");
            return Ok(());
        };
        println!("  Opening {url} with Playwright inspector...");
        let _ = Command::new("npx")
            .args(["playwright", "open", url])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;
        Ok(())
    }

    fn dev_screenshot(args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(args) = args else {
            println!("Usage: /dev screenshot <url> [file]\n  Example: /dev screenshot https://example.com shot.png");
            return Ok(());
        };
        let mut parts = args.splitn(2, ' ');
        let url = parts.next().unwrap_or_default();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let file = parts.next().unwrap_or("").trim();
        let file = if file.is_empty() { format!("screenshot-{timestamp}.png") } else { file.to_string() };

        println!("  Capturing screenshot of {url}...");
        let result = Command::new("npx")
            .args(["playwright", "screenshot", "--full-page", url, &file])
            .output()?;
        if result.status.success() {
            println!("  Screenshot saved: {file}");
        } else {
            let err = String::from_utf8_lossy(&result.stderr);
            println!("  Screenshot failed: {err}");
        }
        Ok(())
    }

    fn dev_codegen(url: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let mut args = vec!["playwright", "codegen"];
        if let Some(url) = url {
            args.push(url);
        }
        println!("  Starting Playwright codegen (record & generate tests)...");
        let _ = Command::new("npx")
            .args(&args)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;
        Ok(())
    }

    fn dev_test(&mut self, description: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(description) = description else {
            println!("Usage: /dev test <description>\n  Example: /dev test login form validates email");
            return Ok(());
        };
        let test_prompt = format!(
            "Write a production-quality Playwright test for: {description}\n\n\
             Requirements:\n\
             1. Use `import {{ test, expect }} from '@playwright/test';`\n\
             2. Use `test.describe` for grouping related assertions\n\
             3. ALWAYS use accessibility locators (never CSS selectors):\n\
                - page.getByRole('button', {{ name: '...' }})\n\
                - page.getByText('...')\n\
                - page.getByLabel('...')\n\
                - page.getByPlaceholder('...')\n\
                - page.getByTestId('...')\n\
             4. Add meaningful assertions with expect()\n\
             5. Handle async properly — await all page interactions\n\
             6. Include setup (page.goto) and cleanup if needed\n\
             7. Test both happy path AND at least one error case\n\
             8. Return ONLY a ```javascript code block — no explanation"
        );
        self.run_turn(&test_prompt)?;

        // Extract and save test code
        let last_text = self.runtime.session().messages.iter().rev()
            .find(|m| m.role == MessageRole::Assistant)
            .map(|m| m.blocks.iter().filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            }).collect::<Vec<_>>().join(""))
            .unwrap_or_default();

        let test_code = last_text.lines()
            .skip_while(|l| !l.starts_with("```"))
            .skip(1)
            .take_while(|l| !l.starts_with("```"))
            .collect::<Vec<_>>()
            .join("\n");

        if !test_code.is_empty() {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            let filename = format!("test-{timestamp}.spec.js");
            fs::write(&filename, &test_code)?;
            println!("\n  Test saved: {filename}");
            print!("  Run now? [y/N]: ");
            io::stdout().flush()?;
            let mut response = String::new();
            io::stdin().read_line(&mut response)?;
            if response.trim().eq_ignore_ascii_case("y") {
                let _ = Command::new("npx")
                    .args(["playwright", "test", &filename, "--reporter=line"])
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .status();
            }
        }
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    //  /mcp — MCP Server Management
    // ════════════════════════════════════════════════════════════════════

    // ════════════════════════════════════════════════════════════════════
    //  /knowledge — OpenAnalyst Knowledge Base (stub — backend TBD)
    // ════════════════════════════════════════════════════════════════════

    fn run_knowledge(&mut self, query: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(query) = query else {
            println!("  \x1b[1mOpenAnalyst Knowledge Base\x1b[0m\n");
            println!("  Usage: /knowledge <query>");
            println!("  Example: /knowledge how to create Meta Ads strategy for D2C\n");
            println!("  Searches the hosted OpenAnalyst knowledge base for expert");
            println!("  strategies, course insights, and actionable guidance.\n");
            println!("  \x1b[2mRequires OPENANALYST_API_KEY environment variable.\x1b[0m");
            return Ok(());
        };

        // Check for API key — required for KB access
        let api_key = env::var("OPENANALYST_API_KEY").or_else(|_| env::var("OA_API_KEY"));
        let Ok(api_key) = api_key else {
            println!("  \x1b[33m[!]\x1b[0m OPENANALYST_API_KEY not set.\n");
            println!("  Set your key to access the knowledge base:");
            println!("    export OPENANALYST_API_KEY=oa_...");
            println!("  Or: set OPENANALYST_API_KEY=oa_...\n");
            println!("  Get your key at: https://openanalyst.com/keys");
            return Ok(());
        };

        let kb_endpoint = env::var("OPENANALYST_KB_URL")
            .unwrap_or_else(|_| "http://209.20.157.253:8000/v1/knowledge/query".to_string());

        println!("  \x1b[38;5;45m[>]\x1b[0m Searching knowledge base...");
        println!("  \x1b[2mQuery: {}\x1b[0m\n", truncate_for_prompt(query, 80));

        // Phase 1: metadata scan → Phase 2: auto-deep transcript search
        let rt = tokio::runtime::Runtime::new()?;
        let result = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .user_agent("openanalyst-cli/1.0")
                .build()?;

            let payload = serde_json::json!({
                "query": query,
                "mode": "progressive",
                "max_results": 10
            });

            let resp = client.post(&kb_endpoint)
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?;

            let status = resp.status();
            let body = resp.text().await?;
            Ok::<_, Box<dyn std::error::Error>>((status, body))
        });

        match result {
            Ok((status, body)) => {
                if status.is_success() {
                    // Parse the response and pass through LLM for synthesis
                    let prompt = format!(
                        "The user asked: \"{query}\"\n\n\
                         The knowledge base returned these results:\n\
                         ```json\n{body}\n```\n\n\
                         Synthesize a comprehensive, actionable answer from these results. \
                         Include source citations. Be specific and practical."
                    );
                    self.run_turn(&prompt)?;
                } else if status.as_u16() == 401 {
                    println!("  \x1b[31m[x]\x1b[0m Authentication failed. Check your OPENANALYST_API_KEY.");
                } else if status.as_u16() == 503 {
                    println!("  \x1b[33m[!]\x1b[0m Knowledge base is not yet available.");
                    println!("  \x1b[2mThe hosted KB backend is under development.\x1b[0m");
                    println!("  \x1b[2mFalling back to AI-only answer...\x1b[0m\n");
                    // Fallback: answer from LLM's own knowledge
                    let fallback = format!(
                        "Answer this query as an expert consultant. Be specific and actionable:\n\n{query}"
                    );
                    self.run_turn(&fallback)?;
                } else {
                    println!("  \x1b[33m[!]\x1b[0m Knowledge base returned an error. Falling back to AI...\n");
                    let fallback = format!(
                        "Answer this query as an expert consultant. Be specific and actionable:\n\n{query}"
                    );
                    self.run_turn(&fallback)?;
                }
            }
            Err(_) => {
                // Network error — fall back gracefully (no internal details exposed)
                println!("  \x1b[33m[!]\x1b[0m Knowledge base is temporarily unavailable.");
                println!("  \x1b[2mFalling back to AI-only answer...\x1b[0m\n");
                let fallback = format!(
                    "Answer this query as an expert consultant. Be specific and actionable:\n\n{query}"
                );
                self.run_turn(&fallback)?;
            }
        }
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    //  /explore — Smart GitHub Repo Explorer
    // ════════════════════════════════════════════════════════════════════

    fn run_explore(&mut self, target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(target) = target else {
            println!("  \x1b[1mSmart Repo Explorer\x1b[0m\n");
            println!("  Usage: /explore <github-url-or-local-path>");
            println!("  Examples:");
            println!("    /explore https://github.com/rust-lang/rust");
            println!("    /explore owner/repo");
            println!("    /explore .                     (current directory)\n");
            println!("  Analyzes a repository from its git history to produce:");
            println!("    - Architecture overview & tech stack");
            println!("    - Commit patterns & active areas");
            println!("    - Key contributors & development velocity");
            println!("    - File change heatmap & module structure");
            return Ok(());
        };

        let is_local = target == "." || Path::new(target).is_dir();
        let is_short_form = !target.contains("://") && target.contains('/') && !Path::new(target).exists();

        if is_local {
            self.explore_local(target)?;
        } else if is_short_form {
            // owner/repo shorthand → use gh api
            self.explore_github(target)?;
        } else if target.contains("github.com") {
            // Full URL — extract owner/repo
            let repo_slug = target
                .trim_end_matches('/')
                .trim_end_matches(".git")
                .rsplit("github.com/")
                .next()
                .unwrap_or(target);
            self.explore_github(repo_slug)?;
        } else {
            // Try as local path
            if Path::new(target).is_dir() {
                self.explore_local(target)?;
            } else {
                println!("  \x1b[31m[x]\x1b[0m Not a valid GitHub URL, owner/repo, or local path: {target}");
            }
        }
        Ok(())
    }

    fn explore_local(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("  \x1b[38;5;45m[>]\x1b[0m Exploring local repository: {path}\n");

        // Gather git log
        let log_output = Command::new("git")
            .args(["-C", path, "log", "--oneline", "--no-merges", "-50"])
            .output()?;
        let log = String::from_utf8_lossy(&log_output.stdout).to_string();

        // Gather file stats
        let stat_output = Command::new("git")
            .args(["-C", path, "log", "--oneline", "--stat", "--no-merges", "-30"])
            .output()?;
        let stats = String::from_utf8_lossy(&stat_output.stdout).to_string();

        // Gather contributor info
        let authors_output = Command::new("git")
            .args(["-C", path, "shortlog", "-sn", "--no-merges", "-20"])
            .output()?;
        let authors = String::from_utf8_lossy(&authors_output.stdout).to_string();

        // Get branch info
        let branch_output = Command::new("git")
            .args(["-C", path, "branch", "-a", "--no-color"])
            .output()?;
        let branches = String::from_utf8_lossy(&branch_output.stdout).to_string();

        // Detect tech stack from file extensions
        let ls_output = Command::new("git")
            .args(["-C", path, "ls-files"])
            .output()?;
        let files = String::from_utf8_lossy(&ls_output.stdout).to_string();
        let file_stats = Self::compute_file_type_stats(&files);

        // Get repo age and activity
        let first_commit = Command::new("git")
            .args(["-C", path, "log", "--reverse", "--format=%ci", "-1"])
            .output()?;
        let first_date = String::from_utf8_lossy(&first_commit.stdout).trim().to_string();

        let last_commit = Command::new("git")
            .args(["-C", path, "log", "--format=%ci", "-1"])
            .output()?;
        let last_date = String::from_utf8_lossy(&last_commit.stdout).trim().to_string();

        let total_commits = Command::new("git")
            .args(["-C", path, "rev-list", "--count", "HEAD"])
            .output()?;
        let commit_count = String::from_utf8_lossy(&total_commits.stdout).trim().to_string();

        // Print structured analysis
        println!("  \x1b[1;38;5;45m== Repository Analysis ==\x1b[0m\n");

        println!("  \x1b[1mTimeline:\x1b[0m");
        println!("    First commit:  {first_date}");
        println!("    Latest commit: {last_date}");
        println!("    Total commits: {commit_count}\n");

        println!("  \x1b[1mTech Stack (by file count):\x1b[0m");
        for (ext, count) in file_stats.iter().take(10) {
            let bar_len = (*count as usize).min(30);
            let bar: String = std::iter::repeat_n('#', bar_len).collect();
            println!("    {ext:>8}  {bar} ({count})");
        }
        println!();

        if !authors.trim().is_empty() {
            println!("  \x1b[1mTop Contributors:\x1b[0m");
            for line in authors.lines().take(5) {
                println!("    {}", line.trim());
            }
            println!();
        }

        let branch_count = branches.lines().count();
        println!("  \x1b[1mBranches:\x1b[0m {branch_count}");
        for line in branches.lines().take(5) {
            println!("    {}", line.trim());
        }
        if branch_count > 5 {
            println!("    ... and {} more", branch_count - 5);
        }
        println!();

        // Now pass everything through LLM for intelligent summary
        let prompt = format!(
            "You are analyzing a repository. Based on the following data, provide:\n\
             1. **Architecture Overview** — key modules, crate/package structure, entry points\n\
             2. **Tech Stack** — languages, frameworks, build tools detected\n\
             3. **Development Patterns** — what areas are most active, what's stable\n\
             4. **Key Features** — what this project does based on commit messages\n\
             5. **Health Assessment** — commit frequency, contributor diversity, code hygiene\n\n\
             Be concise and specific. No generic filler.\n\n\
             --- RECENT COMMITS (newest first) ---\n```\n{log}\n```\n\n\
             --- FILE CHANGE PATTERNS (last 30 commits) ---\n```\n{stats}\n```\n\n\
             --- CONTRIBUTORS ---\n```\n{authors}\n```"
        );
        self.run_turn(&prompt)?;
        Ok(())
    }

    fn explore_github(&mut self, repo_slug: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("  \x1b[38;5;45m[>]\x1b[0m Exploring GitHub repository: {repo_slug}\n");

        // Use gh CLI to fetch repo info
        let repo_info = Command::new("gh")
            .args(["api", &format!("repos/{repo_slug}"), "--jq",
                   "[.full_name, .description, .language, .stargazers_count, .forks_count, .open_issues_count, .created_at, .pushed_at, .default_branch, .topics] | @tsv"])
            .output();

        let repo_meta = match repo_info {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => {
                println!("  \x1b[33m[!]\x1b[0m gh CLI not available or repo not found. Trying git clone...");
                return self.explore_github_via_clone(repo_slug);
            }
        };

        if !repo_meta.is_empty() {
            println!("  \x1b[2mFetched repo metadata via GitHub API\x1b[0m");
        }

        // Fetch recent commits via API
        let commits_output = Command::new("gh")
            .args(["api", &format!("repos/{repo_slug}/commits?per_page=50"),
                   "--jq", ".[] | \"\\(.sha[0:7]) \\(.commit.message | split(\"\\n\") | .[0])\""])
            .output()?;
        let commits = String::from_utf8_lossy(&commits_output.stdout).to_string();

        // Fetch commit activity stats
        let stats_output = Command::new("gh")
            .args(["api", &format!("repos/{repo_slug}/stats/contributors"),
                   "--jq", ".[] | \"\\(.author.login) \\(.total)\""])
            .output()?;
        let contributor_stats = String::from_utf8_lossy(&stats_output.stdout).to_string();

        // Fetch languages
        let lang_output = Command::new("gh")
            .args(["api", &format!("repos/{repo_slug}/languages")])
            .output()?;
        let languages = String::from_utf8_lossy(&lang_output.stdout).to_string();

        // Fetch recent releases
        let release_output = Command::new("gh")
            .args(["api", &format!("repos/{repo_slug}/releases?per_page=5"),
                   "--jq", ".[] | \"\\(.tag_name) \\(.published_at[0:10]) \\(.name)\""])
            .output()?;
        let releases = String::from_utf8_lossy(&release_output.stdout).to_string();

        // Fetch directory structure (root level)
        let tree_output = Command::new("gh")
            .args(["api", &format!("repos/{repo_slug}/git/trees/HEAD"),
                   "--jq", ".tree[] | \"\\(.type) \\(.path)\""])
            .output()?;
        let tree = String::from_utf8_lossy(&tree_output.stdout).to_string();

        // Print quick stats
        println!("  \x1b[1;38;5;45m== Repository Analysis: {repo_slug} ==\x1b[0m\n");

        if !languages.trim().is_empty() && languages.trim() != "{}" {
            println!("  \x1b[1mLanguages:\x1b[0m");
            if let Ok(lang_map) = serde_json::from_str::<serde_json::Value>(&languages) {
                if let Some(obj) = lang_map.as_object() {
                    let total: f64 = obj.values().filter_map(|v| v.as_f64()).sum();
                    for (lang, bytes) in obj.iter().take(8) {
                        let pct = bytes.as_f64().unwrap_or(0.0) / total * 100.0;
                        let bar_len = (pct / 3.0) as usize;
                        let bar: String = std::iter::repeat_n('#', bar_len).collect();
                        println!("    {lang:>12}  {bar} ({pct:.1}%)");
                    }
                }
            }
            println!();
        }

        if !tree.trim().is_empty() {
            println!("  \x1b[1mRoot Structure:\x1b[0m");
            for line in tree.lines().take(20) {
                let icon = if line.starts_with("tree") { "+" } else { " " };
                let name = line.split_whitespace().nth(1).unwrap_or(line);
                println!("    {icon} {name}");
            }
            println!();
        }

        if !releases.trim().is_empty() {
            println!("  \x1b[1mRecent Releases:\x1b[0m");
            for line in releases.lines().take(5) {
                println!("    {line}");
            }
            println!();
        }

        // Pass through LLM for full analysis
        let prompt = format!(
            "You are analyzing the GitHub repository **{repo_slug}**. Based on the data below, provide:\n\
             1. **What This Project Does** — purpose, key features, target users\n\
             2. **Architecture** — module structure, entry points, key directories\n\
             3. **Tech Stack** — languages, frameworks, tools\n\
             4. **Development Activity** — active areas, commit patterns, release cadence\n\
             5. **Community** — contributors, stars/forks context, health\n\n\
             Be concise and specific. Focus on actionable insights.\n\n\
             --- REPO METADATA ---\n{repo_meta}\n\n\
             --- LANGUAGES ---\n{languages}\n\n\
             --- ROOT TREE ---\n{tree}\n\n\
             --- RECENT COMMITS (last 50) ---\n```\n{commits}\n```\n\n\
             --- CONTRIBUTORS ---\n```\n{contributor_stats}\n```\n\n\
             --- RELEASES ---\n{releases}"
        );
        self.run_turn(&prompt)?;
        Ok(())
    }

    fn explore_github_via_clone(&mut self, repo_slug: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Fallback: bare clone to temp dir, then analyze locally
        let temp_dir = env::temp_dir().join(format!("oa-explore-{}", repo_slug.replace('/', "-")));
        let url = format!("https://github.com/{repo_slug}.git");

        println!("  Cloning (bare) {url}...");
        let clone_result = Command::new("git")
            .args(["clone", "--bare", "--depth=50", &url, &temp_dir.display().to_string()])
            .output()?;

        if !clone_result.status.success() {
            let err = String::from_utf8_lossy(&clone_result.stderr);
            println!("  \x1b[31m[x]\x1b[0m Clone failed: {err}");
            return Ok(());
        }

        // Get log from bare repo
        let log_output = Command::new("git")
            .args(["--git-dir", &temp_dir.display().to_string(),
                   "log", "--oneline", "--no-merges", "-50"])
            .output()?;
        let log = String::from_utf8_lossy(&log_output.stdout).to_string();

        let authors_output = Command::new("git")
            .args(["--git-dir", &temp_dir.display().to_string(),
                   "shortlog", "-sn", "--no-merges", "-20"])
            .output()?;
        let authors = String::from_utf8_lossy(&authors_output.stdout).to_string();

        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&temp_dir);

        let prompt = format!(
            "You are analyzing the GitHub repository **{repo_slug}** from its commit history. Provide:\n\
             1. **What This Project Does** — purpose and key features\n\
             2. **Architecture** — module structure based on file paths in commits\n\
             3. **Development Patterns** — active areas, commit themes\n\
             4. **Key Features** — what capabilities the commits reveal\n\n\
             Be concise and specific.\n\n\
             --- RECENT COMMITS ---\n```\n{log}\n```\n\n\
             --- CONTRIBUTORS ---\n```\n{authors}\n```"
        );
        self.run_turn(&prompt)?;
        Ok(())
    }

    fn compute_file_type_stats(file_listing: &str) -> Vec<(String, u32)> {
        let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for line in file_listing.lines() {
            let ext = line.rsplit('.').next().unwrap_or("(none)");
            if ext != line && ext.len() < 10 {
                *counts.entry(ext.to_lowercase()).or_default() += 1;
            }
        }
        let mut sorted: Vec<(String, u32)> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted
    }

    // ════════════════════════════════════════════════════════════════════
    //  Claude Code Parity Commands
    // ════════════════════════════════════════════════════════════════════

    fn run_doctor(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("  \x1b[1mOpenAnalyst CLI Doctor\x1b[0m\n");
        // Check Node.js
        let node_ver = Command::new("node").arg("--version").output();
        println!(
            "  Node.js:       {}",
            node_ver.ok().filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|| "\x1b[38;5;196mnot found\x1b[0m".to_string())
        );
        // Check git
        let git_ver = Command::new("git").arg("--version").output();
        println!(
            "  Git:           {}",
            git_ver.ok().filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|| "\x1b[38;5;196mnot found\x1b[0m".to_string())
        );
        // Check shell
        let sh_ver = Command::new("sh").arg("--version").output();
        println!(
            "  Shell (sh):    {}",
            sh_ver.ok().filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("ok").to_string())
                .unwrap_or_else(|| "\x1b[38;5;196mnot found\x1b[0m".to_string())
        );
        // Check providers
        println!("\n  \x1b[1mProvider credentials:\x1b[0m");
        let providers: &[(&str, &[&str])] = &[
            ("Anthropic", &["ANTHROPIC_API_KEY"]),
            ("OpenAI", &["OPENAI_API_KEY"]),
            ("Google Gemini", &["GEMINI_API_KEY"]),
            ("xAI (Grok)", &["XAI_API_KEY"]),
            ("OpenRouter", &["OPENROUTER_API_KEY"]),
            ("Stability AI", &["STABILITY_API_KEY"]),
        ];
        // OpenAnalyst — check if API key is set
        {
            let has_oa = env::var("OPENANALYST_AUTH_TOKEN").ok().filter(|v| !v.is_empty()).is_some()
                || env::var("OPENANALYST_API_KEY").ok().filter(|v| !v.is_empty()).is_some();
            let mode = env::var("OPENANALYST_MODE").unwrap_or_else(|_| "api".to_string());
            let mode_label = if mode == "free" { "free model" } else { "API credits" };
            let icon = if has_oa { "\x1b[38;5;46m+\x1b[0m" } else { "\x1b[38;5;240m-\x1b[0m" };
            println!("  {icon} OpenAnalyst ({mode_label})");
        }
        for (name, keys) in providers {
            let has_key = keys.iter().any(|k| env::var(k).ok().filter(|v| !v.is_empty()).is_some());
            let icon = if has_key { "\x1b[38;5;46m+\x1b[0m" } else { "\x1b[38;5;240m-\x1b[0m" };
            println!("  {icon} {name}");
        }
        // Check MCP servers
        let cwd = env::current_dir()?;
        let loader = ConfigLoader::default_for(&cwd);
        let config = loader.load()?;
        let mcp_count = config.mcp().servers().len();
        println!("\n  MCP servers:   {mcp_count} configured");
        // Check model
        println!("  Active model:  {}", self.model);
        println!("  Permission:    {}", self.permission_mode.as_str());
        // Check workspace
        let has_oa_md = cwd.join("OPENANALYST.md").exists();
        let has_settings = cwd.join(".openanalyst").join("settings.json").exists();
        println!("\n  \x1b[1mWorkspace:\x1b[0m");
        println!(
            "  OPENANALYST.md {}",
            if has_oa_md { "\x1b[38;5;46mfound\x1b[0m" } else { "\x1b[38;5;240mmissing (run /init)\x1b[0m" }
        );
        println!(
            "  settings.json  {}",
            if has_settings { "\x1b[38;5;46mfound\x1b[0m" } else { "\x1b[38;5;240mmissing\x1b[0m" }
        );
        Ok(())
    }

    fn print_context(&self) {
        let estimated = self.runtime.estimated_tokens();
        let cumulative = self.runtime.usage().cumulative_usage();
        let turns = self.runtime.usage().turns();
        let messages = self.runtime.session().messages.len();
        println!(
            "Context\n\
             \x1b[2m  Messages\x1b[0m        {messages}\n\
             \x1b[2m  Turns\x1b[0m           {turns}\n\
             \x1b[2m  Est. tokens\x1b[0m     ~{estimated}\n\
             \x1b[2m  Input tokens\x1b[0m    {}\n\
             \x1b[2m  Output tokens\x1b[0m   {}\n\
             \x1b[2m  Model\x1b[0m           {}",
            cumulative.input_tokens,
            cumulative.output_tokens,
            self.model,
        );
    }

    fn run_changelog(&mut self, since: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let since_arg = since.unwrap_or("HEAD~20");
        let output = Command::new("git")
            .args(["log", "--oneline", "--no-merges", &format!("{since_arg}..HEAD")])
            .output()?;
        let log = String::from_utf8_lossy(&output.stdout).to_string();
        if log.trim().is_empty() {
            let output2 = Command::new("git")
                .args(["log", "--oneline", "--no-merges", "-20"])
                .output()?;
            let log2 = String::from_utf8_lossy(&output2.stdout).to_string();
            if log2.trim().is_empty() {
                println!("  No commits found.");
                return Ok(());
            }
            let prompt = format!(
                "Generate a changelog from these git commits. Group by category (Features, Fixes, Improvements). Be concise.\n\n```\n{log2}\n```"
            );
            self.run_turn(&prompt)?;
        } else {
            let prompt = format!(
                "Generate a changelog from these git commits since {since_arg}. Group by category (Features, Fixes, Improvements). Be concise.\n\n```\n{log}\n```"
            );
            self.run_turn(&prompt)?;
        }
        Ok(())
    }

    fn run_add_dir(&mut self, path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(dir_path) = path else {
            println!("Usage: /add-dir <directory-path>");
            return Ok(());
        };
        let dir = Path::new(dir_path);
        if !dir.is_dir() {
            println!("  Not a directory: {dir_path}");
            return Ok(());
        }
        // Collect file listing
        let mut files = Vec::new();
        let mut total_size = 0_u64;
        for entry in walkdir::WalkDir::new(dir).max_depth(3).into_iter().flatten() {
            if entry.file_type().is_file() {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                total_size += size;
                files.push(entry.path().display().to_string().replace('\\', "/"));
            }
        }
        let prompt = format!(
            "I'm adding the directory `{dir_path}` to our conversation context.\n\
             It contains {} files ({} bytes total).\n\n\
             Files:\n{}\n\n\
             Acknowledge this directory has been added to context.",
            files.len(),
            total_size,
            files.iter().take(100).map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n"),
        );
        self.run_turn(&prompt)?;
        Ok(())
    }

    fn run_mcp(action: Option<&str>, args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            None | Some("list") => Self::mcp_list(),
            Some("add") => Self::mcp_add(args),
            Some("remove") => Self::mcp_remove(args),
            Some(other) => {
                println!("Unknown /mcp action: {other}\n  Usage: /mcp [list|add <name> <command> [args...]|remove <name>]");
                Ok(())
            }
        }
    }

    fn mcp_list() -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        let loader = ConfigLoader::default_for(&cwd);
        let config = loader.load()?;
        let servers = config.mcp().servers();

        if servers.is_empty() {
            println!("  No MCP servers configured.");
            println!("\n  Add one with: /mcp add <name> <command> [args...]");
            println!("  Example:      /mcp add playwright npx @playwright/mcp@latest");
            return Ok(());
        }

        println!("  \x1b[1mConfigured MCP servers:\x1b[0m\n");
        for (name, scoped) in servers {
            let (transport, detail) = match &scoped.config {
                runtime::McpServerConfig::Stdio(s) => {
                    let cmd = format!("{} {}", s.command, s.args.join(" "));
                    ("stdio", cmd)
                }
                runtime::McpServerConfig::Sse(s) | runtime::McpServerConfig::Http(s) => {
                    ("http/sse", s.url.clone())
                }
                runtime::McpServerConfig::Ws(s) => ("websocket", s.url.clone()),
                runtime::McpServerConfig::Sdk(s) => ("sdk", s.name.clone()),
                runtime::McpServerConfig::ManagedProxy(s) => ("proxy", s.url.clone()),
            };
            let source = match scoped.scope {
                ConfigSource::User => "user",
                ConfigSource::Project => "project",
                ConfigSource::Local => "local",
            };
            println!("  \x1b[38;5;45m{name}\x1b[0m");
            println!("    Transport:  {transport}");
            println!("    Command:    {detail}");
            println!("    Source:     {source}");
            println!();
        }
        Ok(())
    }

    fn mcp_add(args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(args) = args else {
            println!("Usage: /mcp add <name> <command> [args...]\n  Example: /mcp add playwright npx @playwright/mcp@latest");
            return Ok(());
        };
        let mut parts = args.splitn(2, ' ');
        let name = parts.next().unwrap_or_default().trim();
        let command_str = parts.next().unwrap_or_default().trim();
        if name.is_empty() || command_str.is_empty() {
            println!("Usage: /mcp add <name> <command> [args...]");
            return Ok(());
        }

        let mut cmd_parts = command_str.split_whitespace();
        let command = cmd_parts.next().unwrap_or_default();
        let cmd_args: Vec<&str> = cmd_parts.collect();

        // Write to project .openanalyst/settings.json
        let cwd = env::current_dir()?;
        let settings_dir = cwd.join(".openanalyst");
        fs::create_dir_all(&settings_dir)?;
        let settings_path = settings_dir.join("settings.json");

        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        if settings.get("mcpServers").is_none() {
            settings["mcpServers"] = json!({});
        }
        settings["mcpServers"][name] = json!({
            "command": command,
            "args": cmd_args,
        });

        fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        println!("  MCP server \x1b[38;5;45m{name}\x1b[0m added.");
        println!("  Config: {}", settings_path.display());
        println!("\n  \x1b[2mRestart the CLI to activate the MCP server.\x1b[0m");
        Ok(())
    }

    fn mcp_remove(args: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(name) = args.map(str::trim).filter(|s| !s.is_empty()) else {
            println!("Usage: /mcp remove <name>");
            return Ok(());
        };

        let cwd = env::current_dir()?;
        let settings_path = cwd.join(".openanalyst").join("settings.json");
        if !settings_path.exists() {
            println!("  No settings.json found.");
            return Ok(());
        }

        let content = fs::read_to_string(&settings_path)?;
        let mut settings: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(servers) = settings.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
            if servers.remove(name).is_some() {
                fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
                println!("  MCP server \x1b[38;5;45m{name}\x1b[0m removed.");
                println!("\n  \x1b[2mRestart the CLI to apply changes.\x1b[0m");
            } else {
                println!("  MCP server '{name}' not found.");
            }
        } else {
            println!("  No MCP servers configured.");
        }
        Ok(())
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(input: &str) -> Vec<u8> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.decode(input).unwrap_or_default()
}

fn mime_from_extension(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".mp3") { "audio/mpeg" }
    else if lower.ends_with(".wav") { "audio/wav" }
    else if lower.ends_with(".m4a") { "audio/mp4" }
    else if lower.ends_with(".ogg") { "audio/ogg" }
    else if lower.ends_with(".flac") { "audio/flac" }
    else if lower.ends_with(".webm") { "audio/webm" }
    else if lower.ends_with(".mp4") { "video/mp4" }
    else if lower.ends_with(".png") { "image/png" }
    else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") { "image/jpeg" }
    else if lower.ends_with(".webp") { "image/webp" }
    else if lower.ends_with(".gif") { "image/gif" }
    else { "application/octet-stream" }
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
            result.push(' ');
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

fn sessions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let path = cwd.join(".openanalyst").join("sessions");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn create_managed_session_handle() -> Result<SessionHandle, Box<dyn std::error::Error>> {
    let id = generate_session_id();
    let path = sessions_dir()?.join(format!("{id}.json"));
    Ok(SessionHandle { id, path })
}

fn generate_session_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("session-{millis}")
}

fn resolve_session_reference(reference: &str) -> Result<SessionHandle, Box<dyn std::error::Error>> {
    let direct = PathBuf::from(reference);
    let path = if direct.exists() {
        direct
    } else {
        sessions_dir()?.join(format!("{reference}.json"))
    };
    if !path.exists() {
        return Err(format!("session not found: {reference}").into());
    }
    let id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(reference)
        .to_string();
    Ok(SessionHandle { id, path })
}

fn list_managed_sessions() -> Result<Vec<ManagedSessionSummary>, Box<dyn std::error::Error>> {
    let dir = sessions_dir()?;
    let index = SessionIndex::load(&dir);
    let mut sessions = Vec::new();
    let mut index_dirty = false;

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if file_name == "index.json" || file_name == "usage.json" {
            continue;
        }
        let metadata = entry.metadata()?;
        let modified_epoch_secs = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Use cached message count from index if modification time matches
        let message_count = if let Some(cached) = index.get(&id) {
            if cached.modified_epoch_secs == modified_epoch_secs {
                cached.message_count
            } else {
                index_dirty = true;
                Session::load_from_path(&path)
                    .map(|s| s.messages.len())
                    .unwrap_or_default()
            }
        } else {
            index_dirty = true;
            Session::load_from_path(&path)
                .map(|s| s.messages.len())
                .unwrap_or_default()
        };

        sessions.push(ManagedSessionSummary {
            id,
            path,
            modified_epoch_secs,
            message_count,
        });
    }
    sessions.sort_by(|left, right| right.modified_epoch_secs.cmp(&left.modified_epoch_secs));

    // Rebuild index if anything changed
    if index_dirty {
        let new_index = SessionIndex::from_sessions(&sessions);
        let _ = new_index.save(&dir);
    }

    Ok(sessions)
}

// ── Session Index (cached metadata for fast /session list) ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SessionIndexEntry {
    message_count: usize,
    modified_epoch_secs: u64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct SessionIndex {
    sessions: std::collections::BTreeMap<String, SessionIndexEntry>,
}

impl SessionIndex {
    fn load(sessions_dir: &Path) -> Self {
        let path = sessions_dir.join("index.json");
        fs::read_to_string(path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    fn save(&self, sessions_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let path = sessions_dir.join("index.json");
        fs::write(path, serde_json::to_string(self)?)?;
        Ok(())
    }

    fn get(&self, id: &str) -> Option<&SessionIndexEntry> {
        self.sessions.get(id)
    }

    fn from_sessions(sessions: &[ManagedSessionSummary]) -> Self {
        let mut index = Self::default();
        for s in sessions {
            index.sessions.insert(
                s.id.clone(),
                SessionIndexEntry {
                    message_count: s.message_count,
                    modified_epoch_secs: s.modified_epoch_secs,
                },
            );
        }
        index
    }
}

// ── Usage Aggregation (per-provider, per-day token tracking) ──

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct UsageLog {
    entries: Vec<UsageLogEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UsageLogEntry {
    date: String,
    model: String,
    input_tokens: u64,
    output_tokens: u64,
    sessions: u32,
}

impl UsageLog {
    fn path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let dir = sessions_dir()?;
        Ok(dir.join("usage.json"))
    }

    fn load() -> Self {
        Self::path()
            .ok()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::path()?;
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn record(&mut self, date: &str, model: &str, input_tokens: u32, output_tokens: u32) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.date == date && e.model == model) {
            entry.input_tokens += u64::from(input_tokens);
            entry.output_tokens += u64::from(output_tokens);
            entry.sessions += 1;
        } else {
            self.entries.push(UsageLogEntry {
                date: date.to_string(),
                model: model.to_string(),
                input_tokens: u64::from(input_tokens),
                output_tokens: u64::from(output_tokens),
                sessions: 1,
            });
        }
        // Keep only last 90 days of entries
        if self.entries.len() > 500 {
            self.entries.drain(..self.entries.len() - 500);
        }
    }
}

fn record_session_usage(model: &str, usage: &runtime::TokenUsage) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = now / 86400;
    // Simple date from epoch days
    let date = format!("{days}"); // epoch day number, compact
    let mut log = UsageLog::load();
    log.record(&date, model, usage.input_tokens, usage.output_tokens);
    let _ = log.save();
}

// ── Session Auto-Cleanup (prune old sessions on startup) ──

fn cleanup_old_sessions() {
    let Ok(dir) = sessions_dir() else { return };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let cutoff = now.saturating_sub(30 * 24 * 3600); // 30 days
    let max_sessions = 100;

    let Ok(entries) = fs::read_dir(&dir) else { return };
    let mut session_files: Vec<(PathBuf, u64)> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("session-") && name.ends_with(".json")
        })
        .filter_map(|e| {
            let modified = e.metadata().ok()?
                .modified().ok()?
                .duration_since(UNIX_EPOCH).ok()?
                .as_secs();
            Some((e.path(), modified))
        })
        .collect();

    // Sort oldest first
    session_files.sort_by_key(|(_, modified)| *modified);

    let mut removed = 0_usize;
    for (path, modified) in &session_files {
        // Remove empty sessions older than 30 days
        if *modified < cutoff {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(session) = serde_json::from_str::<serde_json::Value>(&content) {
                    let msgs = session.get("messages")
                        .and_then(|m| m.as_array())
                        .map_or(0, Vec::len);
                    if msgs == 0 {
                        let _ = fs::remove_file(path);
                        removed += 1;
                    }
                }
            }
        }
    }

    // If still over max, remove oldest empty sessions
    if session_files.len().saturating_sub(removed) > max_sessions {
        let excess = session_files.len() - removed - max_sessions;
        let mut pruned = 0;
        for (path, _) in &session_files {
            if pruned >= excess { break; }
            if !path.exists() { continue; }
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(session) = serde_json::from_str::<serde_json::Value>(&content) {
                    let msgs = session.get("messages")
                        .and_then(|m| m.as_array())
                        .map_or(0, Vec::len);
                    if msgs <= 2 {
                        let _ = fs::remove_file(path);
                        pruned += 1;
                    }
                }
            }
        }
    }
}

fn render_session_list(active_session_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let sessions = list_managed_sessions()?;
    let mut lines = vec![
        "Sessions".to_string(),
        format!("  Directory         {}", sessions_dir()?.display()),
    ];
    if sessions.is_empty() {
        lines.push("  No managed sessions saved yet.".to_string());
        return Ok(lines.join("\n"));
    }
    for session in sessions {
        let marker = if session.id == active_session_id {
            "● current"
        } else {
            "○ saved"
        };
        lines.push(format!(
            "  {id:<20} {marker:<10} msgs={msgs:<4} modified={modified} path={path}",
            id = session.id,
            msgs = session.message_count,
            modified = session.modified_epoch_secs,
            path = session.path.display(),
        ));
    }
    Ok(lines.join("\n"))
}

fn render_repl_help() -> String {
    [
        "REPL".to_string(),
        "  /exit                Quit the REPL".to_string(),
        "  /quit                Quit the REPL".to_string(),
        "  /vim                 Toggle Vim keybindings".to_string(),
        "  Up/Down              Navigate prompt history".to_string(),
        "  Tab                  Complete slash commands".to_string(),
        "  Ctrl-C               Clear input (or exit on empty prompt)".to_string(),
        "  Shift+Enter/Ctrl+J   Insert a newline".to_string(),
        String::new(),
        render_slash_command_help(),
    ]
    .join(
        "
",
    )
}

fn status_context(
    session_path: Option<&Path>,
) -> Result<StatusContext, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let discovered_config_files = loader.discover().len();
    let runtime_config = loader.load()?;
    let project_context = ProjectContext::discover_with_git(&cwd, DEFAULT_DATE)?;
    let (project_root, git_branch) =
        parse_git_status_metadata(project_context.git_status.as_deref());
    Ok(StatusContext {
        cwd,
        session_path: session_path.map(Path::to_path_buf),
        loaded_config_files: runtime_config.loaded_entries().len(),
        discovered_config_files,
        memory_file_count: project_context.instruction_files.len(),
        project_root,
        git_branch,
    })
}

fn format_status_report(
    model: &str,
    usage: StatusUsage,
    permission_mode: &str,
    context: &StatusContext,
) -> String {
    [
        format!(
            "Status
  Model            {model}
  Permission mode  {permission_mode}
  Messages         {}
  Turns            {}
  Estimated tokens {}",
            usage.message_count, usage.turns, usage.estimated_tokens,
        ),
        format!(
            "Usage
  Latest total     {}
  Cumulative input {}
  Cumulative output {}
  Cumulative total {}",
            usage.latest.total_tokens(),
            usage.cumulative.input_tokens,
            usage.cumulative.output_tokens,
            usage.cumulative.total_tokens(),
        ),
        format!(
            "Workspace
  Cwd              {}
  Project root     {}
  Git branch       {}
  Session          {}
  Config files     loaded {}/{}
  Memory files     {}",
            context.cwd.display(),
            context
                .project_root
                .as_ref()
                .map_or_else(|| "unknown".to_string(), |path| path.display().to_string()),
            context.git_branch.as_deref().unwrap_or("unknown"),
            context.session_path.as_ref().map_or_else(
                || "live-repl".to_string(),
                |path| path.display().to_string()
            ),
            context.loaded_config_files,
            context.discovered_config_files,
            context.memory_file_count,
        ),
    ]
    .join(
        "

",
    )
}

fn render_config_report(section: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let discovered = loader.discover();
    let runtime_config = loader.load()?;

    let mut lines = vec![
        format!(
            "Config
  Working directory {}
  Loaded files      {}
  Merged keys       {}",
            cwd.display(),
            runtime_config.loaded_entries().len(),
            runtime_config.merged().len()
        ),
        "Discovered files".to_string(),
    ];
    for entry in discovered {
        let source = match entry.source {
            ConfigSource::User => "user",
            ConfigSource::Project => "project",
            ConfigSource::Local => "local",
        };
        let status = if runtime_config
            .loaded_entries()
            .iter()
            .any(|loaded_entry| loaded_entry.path == entry.path)
        {
            "loaded"
        } else {
            "missing"
        };
        lines.push(format!(
            "  {source:<7} {status:<7} {}",
            entry.path.display()
        ));
    }

    if let Some(section) = section {
        lines.push(format!("Merged section: {section}"));
        let value = match section {
            "env" => runtime_config.get("env"),
            "hooks" => runtime_config.get("hooks"),
            "model" => runtime_config.get("model"),
            "plugins" => runtime_config
                .get("plugins")
                .or_else(|| runtime_config.get("enabledPlugins")),
            other => {
                lines.push(format!(
                    "  Unsupported config section '{other}'. Use env, hooks, model, or plugins."
                ));
                return Ok(lines.join(
                    "
",
                ));
            }
        };
        lines.push(format!(
            "  {}",
            match value {
                Some(value) => value.render(),
                None => "<unset>".to_string(),
            }
        ));
        return Ok(lines.join(
            "
",
        ));
    }

    lines.push("Merged JSON".to_string());
    lines.push(format!("  {}", runtime_config.as_json().render()));
    Ok(lines.join(
        "
",
    ))
}

fn render_memory_report() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let project_context = ProjectContext::discover(&cwd, DEFAULT_DATE)?;
    let mut lines = vec![format!(
        "Memory
  Working directory {}
  Instruction files {}",
        cwd.display(),
        project_context.instruction_files.len()
    )];
    if project_context.instruction_files.is_empty() {
        lines.push("Discovered files".to_string());
        lines.push(
            "  No OpenAnalyst instruction files discovered in the current directory ancestry."
                .to_string(),
        );
    } else {
        lines.push("Discovered files".to_string());
        for (index, file) in project_context.instruction_files.iter().enumerate() {
            let preview = file.content.lines().next().unwrap_or("").trim();
            let preview = if preview.is_empty() {
                "<empty>"
            } else {
                preview
            };
            lines.push(format!("  {}. {}", index + 1, file.path.display(),));
            lines.push(format!(
                "     lines={} preview={}",
                file.content.lines().count(),
                preview
            ));
        }
    }
    Ok(lines.join(
        "
",
    ))
}

fn init_openanalyst_md() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    Ok(initialize_repo(&cwd)?.render())
}

fn run_init() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", init_openanalyst_md()?);
    Ok(())
}

fn normalize_permission_mode(mode: &str) -> Option<&'static str> {
    match mode.trim() {
        "read-only" => Some("read-only"),
        "workspace-write" => Some("workspace-write"),
        "danger-full-access" => Some("danger-full-access"),
        _ => None,
    }
}

fn render_diff_report() -> Result<String, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--", ":(exclude).omx"])
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git diff failed: {stderr}").into());
    }
    let diff = String::from_utf8(output.stdout)?;
    if diff.trim().is_empty() {
        return Ok(
            "Diff\n  Result           clean working tree\n  Detail           no current changes"
                .to_string(),
        );
    }
    Ok(format!("Diff\n\n{}", diff.trim_end()))
}

fn render_teleport_report(target: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;

    let file_list = Command::new("rg")
        .args(["--files"])
        .current_dir(&cwd)
        .output()?;
    let file_matches = if file_list.status.success() {
        String::from_utf8(file_list.stdout)?
            .lines()
            .filter(|line| line.contains(target))
            .take(10)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let content_output = Command::new("rg")
        .args(["-n", "-S", "--color", "never", target, "."])
        .current_dir(&cwd)
        .output()?;

    let mut lines = vec![format!("Teleport\n  Target           {target}")];
    if !file_matches.is_empty() {
        lines.push(String::new());
        lines.push("File matches".to_string());
        lines.extend(file_matches.into_iter().map(|path| format!("  {path}")));
    }

    if content_output.status.success() {
        let matches = String::from_utf8(content_output.stdout)?;
        if !matches.trim().is_empty() {
            lines.push(String::new());
            lines.push("Content matches".to_string());
            lines.push(truncate_for_prompt(&matches, 4_000));
        }
    }

    if lines.len() == 1 {
        lines.push("  Result           no matches found".to_string());
    }

    Ok(lines.join("\n"))
}

fn render_last_tool_debug_report(session: &Session) -> Result<String, Box<dyn std::error::Error>> {
    let last_tool_use = session
        .messages
        .iter()
        .rev()
        .find_map(|message| {
            message.blocks.iter().rev().find_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.clone(), name.clone(), input.clone()))
                }
                _ => None,
            })
        })
        .ok_or_else(|| "no prior tool call found in session".to_string())?;

    let tool_result = session.messages.iter().rev().find_map(|message| {
        message.blocks.iter().rev().find_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } if tool_use_id == &last_tool_use.0 => {
                Some((tool_name.clone(), output.clone(), *is_error))
            }
            _ => None,
        })
    });

    let mut lines = vec![
        "Debug tool call".to_string(),
        format!("  Tool id          {}", last_tool_use.0),
        format!("  Tool name        {}", last_tool_use.1),
        "  Input".to_string(),
        indent_block(&last_tool_use.2, 4),
    ];

    match tool_result {
        Some((tool_name, output, is_error)) => {
            lines.push("  Result".to_string());
            lines.push(format!("    name           {tool_name}"));
            lines.push(format!(
                "    status         {}",
                if is_error { "error" } else { "ok" }
            ));
            lines.push(indent_block(&output, 4));
        }
        None => lines.push("  Result           missing tool result".to_string()),
    }

    Ok(lines.join("\n"))
}

fn indent_block(value: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn git_output(args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn git_status_ok(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(())
}

fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn write_temp_text_file(
    filename: &str,
    contents: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = env::temp_dir().join(filename);
    fs::write(&path, contents)?;
    Ok(path)
}

fn recent_user_context(session: &Session, limit: usize) -> String {
    let requests = session
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .filter_map(|message| {
            message.blocks.iter().find_map(|block| match block {
                ContentBlock::Text { text } => Some(text.trim().to_string()),
                _ => None,
            })
        })
        .rev()
        .take(limit)
        .collect::<Vec<_>>();

    if requests.is_empty() {
        "<no prior user messages>".to_string()
    } else {
        requests
            .into_iter()
            .rev()
            .enumerate()
            .map(|(index, text)| format!("{}. {}", index + 1, text))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn truncate_for_prompt(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.trim().to_string()
    } else {
        let truncated = value.chars().take(limit).collect::<String>();
        format!("{}\n…[truncated]", truncated.trim_end())
    }
}

fn sanitize_generated_message(value: &str) -> String {
    value.trim().trim_matches('`').trim().replace("\r\n", "\n")
}

fn parse_titled_body(value: &str) -> Option<(String, String)> {
    let normalized = sanitize_generated_message(value);
    let title = normalized
        .lines()
        .find_map(|line| line.strip_prefix("TITLE:").map(str::trim))?;
    let body_start = normalized.find("BODY:")?;
    let body = normalized[body_start + "BODY:".len()..].trim();
    Some((title.to_string(), body.to_string()))
}

fn render_version_report() -> String {
    let git_sha = GIT_SHA.unwrap_or("unknown");
    let target = BUILD_TARGET.unwrap_or("unknown");
    format!(
        "OpenAnalyst CLI\n  Version          {VERSION}\n  Git SHA          {git_sha}\n  Target           {target}\n  Build date       {DEFAULT_DATE}"
    )
}

fn render_export_text(session: &Session) -> String {
    let mut lines = vec!["# Conversation Export".to_string(), String::new()];
    for (index, message) in session.messages.iter().enumerate() {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        lines.push(format!("## {}. {role}", index + 1));
        for block in &message.blocks {
            match block {
                ContentBlock::Text { text } => lines.push(text.clone()),
                ContentBlock::ToolUse { id, name, input } => {
                    lines.push(format!("[tool_use id={id} name={name}] {input}"));
                }
                ContentBlock::ToolResult {
                    tool_use_id,
                    tool_name,
                    output,
                    is_error,
                } => {
                    lines.push(format!(
                        "[tool_result id={tool_use_id} name={tool_name} error={is_error}] {output}"
                    ));
                }
            }
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn default_export_filename(session: &Session) -> String {
    let stem = session
        .messages
        .iter()
        .find_map(|message| match message.role {
            MessageRole::User => message.blocks.iter().find_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            }),
            _ => None,
        })
        .map_or("conversation", |text| {
            text.lines().next().unwrap_or("conversation")
        })
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("-");
    let fallback = if stem.is_empty() {
        "conversation"
    } else {
        &stem
    };
    format!("{fallback}.txt")
}

fn resolve_export_path(
    requested_path: Option<&str>,
    session: &Session,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let file_name =
        requested_path.map_or_else(|| default_export_filename(session), ToOwned::to_owned);
    let final_name = if Path::new(&file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
    {
        file_name
    } else {
        format!("{file_name}.txt")
    };
    Ok(cwd.join(final_name))
}

fn build_system_prompt() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(load_system_prompt(
        env::current_dir()?,
        DEFAULT_DATE,
        env::consts::OS,
        "unknown",
    )?)
}

fn build_runtime_plugin_state(
) -> Result<(runtime::RuntimeFeatureConfig, GlobalToolRegistry), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load()?;
    let plugin_manager = build_plugin_manager(&cwd, &loader, &runtime_config);
    let tool_registry = GlobalToolRegistry::with_plugin_tools(plugin_manager.aggregated_tools()?)?;
    Ok((runtime_config.feature_config().clone(), tool_registry))
}

fn build_plugin_manager(
    cwd: &Path,
    loader: &ConfigLoader,
    runtime_config: &runtime::RuntimeConfig,
) -> PluginManager {
    let plugin_settings = runtime_config.plugins();
    let mut plugin_config = PluginManagerConfig::new(loader.config_home().to_path_buf());
    plugin_config.enabled_plugins = plugin_settings.enabled_plugins().clone();
    plugin_config.external_dirs = plugin_settings
        .external_directories()
        .iter()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path))
        .collect();
    plugin_config.install_root = plugin_settings
        .install_root()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    plugin_config.registry_path = plugin_settings
        .registry_path()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    plugin_config.bundled_root = plugin_settings
        .bundled_root()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    PluginManager::new(plugin_config)
}

fn resolve_plugin_path(cwd: &Path, config_home: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else if value.starts_with('.') {
        cwd.join(path)
    } else {
        config_home.join(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InternalPromptProgressState {
    command_label: &'static str,
    task_label: String,
    step: usize,
    phase: String,
    detail: Option<String>,
    saw_final_text: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InternalPromptProgressEvent {
    Started,
    Update,
    Heartbeat,
    Complete,
    Failed,
}

#[derive(Debug)]
struct InternalPromptProgressShared {
    state: Mutex<InternalPromptProgressState>,
    output_lock: Mutex<()>,
    started_at: Instant,
}

#[derive(Debug, Clone)]
struct InternalPromptProgressReporter {
    shared: Arc<InternalPromptProgressShared>,
}

#[derive(Debug)]
struct InternalPromptProgressRun {
    reporter: InternalPromptProgressReporter,
    heartbeat_stop: Option<mpsc::Sender<()>>,
    heartbeat_handle: Option<thread::JoinHandle<()>>,
}

impl InternalPromptProgressReporter {
    fn ultraplan(task: &str) -> Self {
        Self {
            shared: Arc::new(InternalPromptProgressShared {
                state: Mutex::new(InternalPromptProgressState {
                    command_label: "Ultraplan",
                    task_label: task.to_string(),
                    step: 0,
                    phase: "planning started".to_string(),
                    detail: Some(format!("task: {task}")),
                    saw_final_text: false,
                }),
                output_lock: Mutex::new(()),
                started_at: Instant::now(),
            }),
        }
    }

    fn emit(&self, event: InternalPromptProgressEvent, error: Option<&str>) {
        let snapshot = self.snapshot();
        let line = format_internal_prompt_progress_line(event, &snapshot, self.elapsed(), error);
        self.write_line(&line);
    }

    fn mark_model_phase(&self) {
        let snapshot = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("internal prompt progress state poisoned");
            state.step += 1;
            state.phase = if state.step == 1 {
                "analyzing request".to_string()
            } else {
                "reviewing findings".to_string()
            };
            state.detail = Some(format!("task: {}", state.task_label));
            state.clone()
        };
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Update,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn mark_tool_phase(&self, name: &str, input: &str) {
        let detail = describe_tool_progress(name, input);
        let snapshot = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("internal prompt progress state poisoned");
            state.step += 1;
            state.phase = format!("running {name}");
            state.detail = Some(detail);
            state.clone()
        };
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Update,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn mark_text_phase(&self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let detail = truncate_for_summary(first_visible_line(trimmed), 120);
        let snapshot = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("internal prompt progress state poisoned");
            if state.saw_final_text {
                return;
            }
            state.saw_final_text = true;
            state.step += 1;
            state.phase = "drafting final plan".to_string();
            state.detail = (!detail.is_empty()).then_some(detail);
            state.clone()
        };
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Update,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn emit_heartbeat(&self) {
        let snapshot = self.snapshot();
        self.write_line(&format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Heartbeat,
            &snapshot,
            self.elapsed(),
            None,
        ));
    }

    fn snapshot(&self) -> InternalPromptProgressState {
        self.shared
            .state
            .lock()
            .expect("internal prompt progress state poisoned")
            .clone()
    }

    fn elapsed(&self) -> Duration {
        self.shared.started_at.elapsed()
    }

    fn write_line(&self, line: &str) {
        let _guard = self
            .shared
            .output_lock
            .lock()
            .expect("internal prompt progress output lock poisoned");
        let mut stdout = io::stdout();
        let _ = writeln!(stdout, "{line}");
        let _ = stdout.flush();
    }
}

impl InternalPromptProgressRun {
    fn start_ultraplan(task: &str) -> Self {
        let reporter = InternalPromptProgressReporter::ultraplan(task);
        reporter.emit(InternalPromptProgressEvent::Started, None);

        let (heartbeat_stop, heartbeat_rx) = mpsc::channel();
        let heartbeat_reporter = reporter.clone();
        let heartbeat_handle = thread::spawn(move || loop {
            match heartbeat_rx.recv_timeout(INTERNAL_PROGRESS_HEARTBEAT_INTERVAL) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => heartbeat_reporter.emit_heartbeat(),
            }
        });

        Self {
            reporter,
            heartbeat_stop: Some(heartbeat_stop),
            heartbeat_handle: Some(heartbeat_handle),
        }
    }

    fn reporter(&self) -> InternalPromptProgressReporter {
        self.reporter.clone()
    }

    fn finish_success(&mut self) {
        self.stop_heartbeat();
        self.reporter
            .emit(InternalPromptProgressEvent::Complete, None);
    }

    fn finish_failure(&mut self, error: &str) {
        self.stop_heartbeat();
        self.reporter
            .emit(InternalPromptProgressEvent::Failed, Some(error));
    }

    fn stop_heartbeat(&mut self) {
        if let Some(sender) = self.heartbeat_stop.take() {
            let _ = sender.send(());
        }
        if let Some(handle) = self.heartbeat_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for InternalPromptProgressRun {
    fn drop(&mut self) {
        self.stop_heartbeat();
    }
}

fn format_internal_prompt_progress_line(
    event: InternalPromptProgressEvent,
    snapshot: &InternalPromptProgressState,
    elapsed: Duration,
    error: Option<&str>,
) -> String {
    let elapsed_seconds = elapsed.as_secs();
    let step_label = if snapshot.step == 0 {
        "current step pending".to_string()
    } else {
        format!("current step {}", snapshot.step)
    };
    let mut status_bits = vec![step_label, format!("phase {}", snapshot.phase)];
    if let Some(detail) = snapshot
        .detail
        .as_deref()
        .filter(|detail| !detail.is_empty())
    {
        status_bits.push(detail.to_string());
    }
    let status = status_bits.join(" · ");
    match event {
        InternalPromptProgressEvent::Started => {
            format!(
                "🧭 {} status · planning started · {status}",
                snapshot.command_label
            )
        }
        InternalPromptProgressEvent::Update => {
            format!("… {} status · {status}", snapshot.command_label)
        }
        InternalPromptProgressEvent::Heartbeat => format!(
            "… {} heartbeat · {elapsed_seconds}s elapsed · {status}",
            snapshot.command_label
        ),
        InternalPromptProgressEvent::Complete => format!(
            "✔ {} status · completed · {elapsed_seconds}s elapsed · {} steps total",
            snapshot.command_label, snapshot.step
        ),
        InternalPromptProgressEvent::Failed => format!(
            "✘ {} status · failed · {elapsed_seconds}s elapsed · {}",
            snapshot.command_label,
            error.unwrap_or("unknown error")
        ),
    }
}

fn describe_tool_progress(name: &str, input: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string()));
    match name {
        "bash" | "Bash" => {
            let command = parsed
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if command.is_empty() {
                "running shell command".to_string()
            } else {
                format!("command {}", truncate_for_summary(command.trim(), 100))
            }
        }
        "read_file" | "Read" => format!("reading {}", extract_tool_path(&parsed)),
        "write_file" | "Write" => format!("writing {}", extract_tool_path(&parsed)),
        "edit_file" | "Edit" => format!("editing {}", extract_tool_path(&parsed)),
        "glob_search" | "Glob" => {
            let pattern = parsed
                .get("pattern")
                .and_then(|value| value.as_str())
                .unwrap_or("?");
            let scope = parsed
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or(".");
            format!("glob `{pattern}` in {scope}")
        }
        "grep_search" | "Grep" => {
            let pattern = parsed
                .get("pattern")
                .and_then(|value| value.as_str())
                .unwrap_or("?");
            let scope = parsed
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or(".");
            format!("grep `{pattern}` in {scope}")
        }
        "web_search" | "WebSearch" => parsed
            .get("query")
            .and_then(|value| value.as_str())
            .map_or_else(
                || "running web search".to_string(),
                |query| format!("query {}", truncate_for_summary(query, 100)),
            ),
        _ => {
            let summary = summarize_tool_payload(input);
            if summary.is_empty() {
                format!("running {name}")
            } else {
                format!("{name}: {summary}")
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn build_runtime(
    session: Session,
    model: String,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    progress_reporter: Option<InternalPromptProgressReporter>,
) -> Result<ConversationRuntime<DefaultRuntimeClient, CliToolExecutor>, Box<dyn std::error::Error>> {
    let (feature_config, tool_registry) = build_runtime_plugin_state()?;
    Ok(ConversationRuntime::new_with_features(
        session,
        DefaultRuntimeClient::new(
            model,
            enable_tools,
            emit_output,
            allowed_tools.clone(),
            tool_registry.clone(),
            progress_reporter,
        )?,
        CliToolExecutor::new(allowed_tools.clone(), emit_output, tool_registry.clone()),
        permission_policy(permission_mode, &tool_registry),
        system_prompt,
        feature_config,
    ))
}

struct CliPermissionPrompter {
    current_mode: PermissionMode,
}

impl CliPermissionPrompter {
    fn new(current_mode: PermissionMode) -> Self {
        Self { current_mode }
    }
}

impl runtime::PermissionPrompter for CliPermissionPrompter {
    fn decide(
        &mut self,
        request: &runtime::PermissionRequest,
    ) -> runtime::PermissionPromptDecision {
        println!();
        println!("Permission approval required");
        println!("  Tool             {}", request.tool_name);
        println!("  Current mode     {}", self.current_mode.as_str());
        println!("  Required mode    {}", request.required_mode.as_str());
        println!("  Input            {}", request.input);
        print!("Approve this tool call? [y/N]: ");
        let _ = io::stdout().flush();

        let mut response = String::new();
        match io::stdin().read_line(&mut response) {
            Ok(_) => {
                let normalized = response.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "y" | "yes") {
                    runtime::PermissionPromptDecision::Allow
                } else {
                    runtime::PermissionPromptDecision::Deny {
                        reason: format!(
                            "tool '{}' denied by user approval prompt",
                            request.tool_name
                        ),
                    }
                }
            }
            Err(error) => runtime::PermissionPromptDecision::Deny {
                reason: format!("permission approval failed: {error}"),
            },
        }
    }
}

struct DefaultRuntimeClient {
    runtime: tokio::runtime::Runtime,
    client: api::ProviderClient,
    model: String,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    progress_reporter: Option<InternalPromptProgressReporter>,
}

impl DefaultRuntimeClient {
    fn new(
        model: String,
        enable_tools: bool,
        emit_output: bool,
        allowed_tools: Option<AllowedToolSet>,
        tool_registry: GlobalToolRegistry,
        progress_reporter: Option<InternalPromptProgressReporter>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Build the correct provider client based on model name.
        // This is what enables cross-provider /model switching:
        // - "openanalyst-beta" or "sonnet" → OpenAnalyst/Anthropic API
        // - "gpt-4o" → OpenAI API
        // - "grok" → xAI API
        // - "openrouter/*" → OpenRouter API
        // The session and conversation persist across provider switches.
        let default_auth = resolve_cli_auth_source().ok();
        let client = api::ProviderClient::from_model_with_default_auth(
            &model,
            default_auth,
        )?;
        Ok(Self {
            runtime: tokio::runtime::Runtime::new()?,
            client,
            model,
            enable_tools,
            emit_output,
            allowed_tools,
            tool_registry,
            progress_reporter,
        })
    }
}

// ── Startup account info fetched from the active API ──

struct StartupAccountInfo {
    display_name: String,
    model_display: String,
    provider_name: String,
    user_name: Option<String>,
    user_email: Option<String>,
    subscription: Option<String>,
    organization: Option<String>,
}

fn fetch_startup_account_info(model: &str) -> StartupAccountInfo {
    use api::{detect_provider_kind, ProviderKind};

    let provider = detect_provider_kind(model);
    let provider_name = provider.display_name().to_string();

    // Resolve the auth env vars and base URL for this provider
    let (auth_token, api_key, base_url) = match provider {
        ProviderKind::OpenAnalystApi => (
            env::var("OPENANALYST_AUTH_TOKEN").ok().filter(|v| !v.is_empty())
                .or_else(|| env::var("ANTHROPIC_AUTH_TOKEN").ok().filter(|v| !v.is_empty())),
            env::var("OPENANALYST_API_KEY").ok().filter(|v| !v.is_empty())
                .or_else(|| env::var("ANTHROPIC_API_KEY").ok().filter(|v| !v.is_empty())),
            env::var("OPENANALYST_BASE_URL")
                .unwrap_or_else(|_| "https://api.openanalyst.com/api".to_string()),
        ),
        ProviderKind::Anthropic => (
            env::var("ANTHROPIC_AUTH_TOKEN").ok().filter(|v| !v.is_empty()),
            env::var("ANTHROPIC_API_KEY").ok().filter(|v| !v.is_empty()),
            env::var("ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
        ),
        ProviderKind::OpenAi => (
            None,
            env::var("OPENAI_API_KEY").ok().filter(|v| !v.is_empty()),
            env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
        ),
        ProviderKind::Xai => (
            None,
            env::var("XAI_API_KEY").ok().filter(|v| !v.is_empty()),
            env::var("XAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.x.ai/v1".to_string()),
        ),
        ProviderKind::OpenRouter => (
            None,
            env::var("OPENROUTER_API_KEY").ok().filter(|v| !v.is_empty()),
            env::var("OPENROUTER_BASE_URL")
                .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string()),
        ),
        ProviderKind::Bedrock => (
            None,
            env::var("BEDROCK_API_KEY").ok().filter(|v| !v.is_empty()),
            env::var("BEDROCK_BASE_URL")
                .unwrap_or_else(|_| "https://bedrock-runtime.us-east-1.amazonaws.com/v1".to_string()),
        ),
        ProviderKind::Gemini => (
            None,
            env::var("GEMINI_API_KEY").ok().filter(|v| !v.is_empty()),
            env::var("GEMINI_BASE_URL")
                .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/openai".to_string()),
        ),
    };

    // Try to fetch account info from API
    let fetched = try_fetch_account(&base_url, auth_token.as_deref(), api_key.as_deref());

    // Fallback display name from OS
    let os_user = env::var("USER").or_else(|_| env::var("USERNAME")).ok()
        .filter(|v| !v.is_empty());

    let display_name = fetched.as_ref()
        .and_then(|f| f.name.clone())
        .or_else(|| os_user.clone())
        .unwrap_or_else(|| provider_name.clone());

    StartupAccountInfo {
        display_name,
        model_display: model.to_string(),
        provider_name,
        user_name: fetched.as_ref().and_then(|f| f.name.clone()).or(os_user),
        user_email: fetched.as_ref().and_then(|f| f.email.clone()),
        subscription: fetched.as_ref().and_then(|f| f.subscription.clone()),
        organization: fetched.as_ref().and_then(|f| f.organization.clone()),
    }
}

#[derive(Debug)]
struct FetchedAccount {
    name: Option<String>,
    email: Option<String>,
    subscription: Option<String>,
    organization: Option<String>,
}

fn try_fetch_account(
    base_url: &str,
    auth_token: Option<&str>,
    api_key: Option<&str>,
) -> Option<FetchedAccount> {
    let rt = tokio::runtime::Runtime::new().ok()?;
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .ok()?;

        let base = base_url.trim_end_matches('/');

        // Try multiple known endpoints for user info:
        // 1. /v1/me (standard)
        // 2. /keys (OpenAnalyst JWT-based)
        // 3. /credits/balance (OpenAnalyst - has user context)
        let endpoints = [
            format!("{base}/v1/me"),
            format!("{base}/health"),
        ];

        for url in &endpoints {
            let mut req = client.get(url)
                .header("anthropic-version", "2023-06-01");
            if let Some(token) = auth_token {
                req = req.bearer_auth(token);
            }
            if let Some(key) = api_key {
                req = req.header("x-api-key", key);
            }

            let resp = match req.send().await {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };

            let body: serde_json::Value = match resp.json().await {
                Ok(b) => b,
                Err(_) => continue,
            };

            let account = FetchedAccount {
                name: body.get("name").and_then(|v| v.as_str()).map(ToOwned::to_owned)
                    .or_else(|| body.get("display_name").and_then(|v| v.as_str()).map(ToOwned::to_owned))
                    .or_else(|| body.get("user").and_then(|u| u.get("name")).and_then(|v| v.as_str()).map(ToOwned::to_owned)),
                email: body.get("email").and_then(|v| v.as_str()).map(ToOwned::to_owned)
                    .or_else(|| body.get("user").and_then(|u| u.get("email")).and_then(|v| v.as_str()).map(ToOwned::to_owned)),
                subscription: body.get("subscription").and_then(|v| v.as_str()).map(ToOwned::to_owned)
                    .or_else(|| body.get("plan").and_then(|v| v.as_str()).map(ToOwned::to_owned))
                    .or_else(|| body.get("tier").and_then(|v| v.as_str()).map(ToOwned::to_owned)),
                organization: body.get("organization").and_then(|v| {
                        v.as_str().map(ToOwned::to_owned)
                            .or_else(|| v.get("name").and_then(|n| n.as_str()).map(ToOwned::to_owned))
                    })
                    .or_else(|| body.get("org_name").and_then(|v| v.as_str()).map(ToOwned::to_owned)),
            };

            // Only return if we got at least one useful field
            if account.name.is_some() || account.email.is_some() {
                return Some(account);
            }
        }
        None
    })
}

fn resolve_cli_auth_source() -> Result<AuthSource, Box<dyn std::error::Error>> {
    Ok(resolve_startup_auth_source(|| {
        let cwd = env::current_dir().map_err(api::ApiError::from)?;
        let config = ConfigLoader::default_for(&cwd).load().map_err(|error| {
            api::ApiError::Auth(format!("failed to load runtime OAuth config: {error}"))
        })?;
        Ok(config.oauth().cloned())
    })?)
}

impl ApiClient for DefaultRuntimeClient {
    #[allow(clippy::too_many_lines)]
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        if let Some(progress_reporter) = &self.progress_reporter {
            progress_reporter.mark_model_phase();
        }
        let message_request = MessageRequest {
            model: self.model.clone(),
            max_tokens: max_tokens_for_model(&self.model),
            messages: convert_messages(&request.messages),
            system: (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n")),
            tools: self
                .enable_tools
                .then(|| filter_tool_specs(&self.tool_registry, self.allowed_tools.as_ref())),
            tool_choice: self.enable_tools.then_some(ToolChoice::Auto),
            stream: true,
            thinking: None,
        };

        self.runtime.block_on(async {
            let mut stream = self
                .client
                .stream_message(&message_request)
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            let mut stdout = io::stdout();
            let mut sink = io::sink();
            let out: &mut dyn Write = if self.emit_output {
                &mut stdout
            } else {
                &mut sink
            };
            let renderer = TerminalRenderer::new();
            let mut markdown_stream = MarkdownStreamState::default();
            let mut events = Vec::new();
            let mut pending_tool: Option<(String, String, String)> = None;
            let mut saw_stop = false;

            while let Some(event) = stream
                .next_event()
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?
            {
                match event {
                    ApiStreamEvent::MessageStart(start) => {
                        for block in start.message.content {
                            push_output_block(block, out, &mut events, &mut pending_tool, true)?;
                        }
                    }
                    ApiStreamEvent::ContentBlockStart(start) => {
                        push_output_block(
                            start.content_block,
                            out,
                            &mut events,
                            &mut pending_tool,
                            true,
                        )?;
                    }
                    ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                let text = runtime::scrub_model_identity(&text);
                                if let Some(progress_reporter) = &self.progress_reporter {
                                    progress_reporter.mark_text_phase(&text);
                                }
                                if let Some(rendered) = markdown_stream.push(&renderer, &text) {
                                    write!(out, "{rendered}")
                                        .and_then(|()| out.flush())
                                        .map_err(|error| RuntimeError::new(error.to_string()))?;
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
                        if let Some(rendered) = markdown_stream.flush(&renderer) {
                            write!(out, "{rendered}")
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                        }
                        if let Some((id, name, input)) = pending_tool.take() {
                            if let Some(progress_reporter) = &self.progress_reporter {
                                progress_reporter.mark_tool_phase(&name, &input);
                            }
                            // Display tool call now that input is fully accumulated
                            writeln!(out, "\n{}", format_tool_call_start(&name, &input))
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
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
                        if let Some(rendered) = markdown_stream.flush(&renderer) {
                            write!(out, "{rendered}")
                                .and_then(|()| out.flush())
                                .map_err(|error| RuntimeError::new(error.to_string()))?;
                        }
                        events.push(AssistantEvent::MessageStop);
                    }
                }
            }

            if !saw_stop
                && events.iter().any(|event| {
                    matches!(event, AssistantEvent::TextDelta(text) if !text.is_empty())
                        || matches!(event, AssistantEvent::ToolUse { .. })
                })
            {
                events.push(AssistantEvent::MessageStop);
            }

            if events
                .iter()
                .any(|event| matches!(event, AssistantEvent::MessageStop))
            {
                return Ok(events);
            }

            let response = self
                .client
                .send_message(&MessageRequest {
                    stream: false,
            thinking: None,
                    ..message_request.clone()
                })
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            response_to_events(response, out)
        })
    }
}

fn final_assistant_text(summary: &runtime::TurnSummary) -> String {
    summary
        .assistant_messages
        .last()
        .map(|message| {
            message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn collect_tool_uses(summary: &runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .assistant_messages
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some(json!({
                "id": id,
                "name": name,
                "input": input,
            })),
            _ => None,
        })
        .collect()
}

fn collect_tool_results(summary: &runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .tool_results
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => Some(json!({
                "tool_use_id": tool_use_id,
                "tool_name": tool_name,
                "output": output,
                "is_error": is_error,
            })),
            _ => None,
        })
        .collect()
}

fn slash_command_completion_candidates() -> Vec<String> {
    let mut candidates = slash_command_specs()
        .iter()
        .flat_map(|spec| {
            std::iter::once(spec.name)
                .chain(spec.aliases.iter().copied())
                .map(|name| format!("/{name}"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    candidates.push("/vim".to_string());
    candidates
}

fn format_tool_call_start(name: &str, input: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string()));

    let detail = match name {
        "bash" | "Bash" => format_bash_call(&parsed),
        "read_file" | "Read" => {
            let path = extract_tool_path(&parsed);
            format!("\x1b[2m📄 Reading {path}…\x1b[0m")
        }
        "write_file" | "Write" => {
            let path = extract_tool_path(&parsed);
            let lines = parsed
                .get("content")
                .and_then(|value| value.as_str())
                .map_or(0, |content| content.lines().count());
            format!("\x1b[1;32m✏️ Writing {path}\x1b[0m \x1b[2m({lines} lines)\x1b[0m")
        }
        "edit_file" | "Edit" => {
            let path = extract_tool_path(&parsed);
            let old_value = parsed
                .get("old_string")
                .or_else(|| parsed.get("oldString"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let new_value = parsed
                .get("new_string")
                .or_else(|| parsed.get("newString"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            format!(
                "\x1b[1;33m📝 Editing {path}\x1b[0m{}",
                format_patch_preview(old_value, new_value)
                    .map(|preview| format!("\n{preview}"))
                    .unwrap_or_default()
            )
        }
        "glob_search" | "Glob" => format_search_start("🔎 Glob", &parsed),
        "grep_search" | "Grep" => format_search_start("🔎 Grep", &parsed),
        "web_search" | "WebSearch" => parsed
            .get("query")
            .and_then(|value| value.as_str())
            .unwrap_or("?")
            .to_string(),
        _ => summarize_tool_payload(input),
    };

    let border = "─".repeat(name.len() + 8);
    format!(
        "\x1b[38;5;245m╭─ \x1b[1;36m{name}\x1b[0;38;5;245m ─╮\x1b[0m\n\x1b[38;5;245m│\x1b[0m {detail}\n\x1b[38;5;245m╰{border}╯\x1b[0m"
    )
}

fn format_tool_result(name: &str, output: &str, is_error: bool) -> String {
    let icon = if is_error {
        "\x1b[1;31m✗\x1b[0m"
    } else {
        "\x1b[1;32m✓\x1b[0m"
    };
    if is_error {
        let summary = truncate_for_summary(output.trim(), 160);
        return if summary.is_empty() {
            format!("{icon} \x1b[38;5;245m{name}\x1b[0m")
        } else {
            format!("{icon} \x1b[38;5;245m{name}\x1b[0m\n\x1b[38;5;203m{summary}\x1b[0m")
        };
    }

    let parsed: serde_json::Value =
        serde_json::from_str(output).unwrap_or(serde_json::Value::String(output.to_string()));
    match name {
        "bash" | "Bash" => format_bash_result(icon, &parsed),
        "read_file" | "Read" => format_read_result(icon, &parsed),
        "write_file" | "Write" => format_write_result(icon, &parsed),
        "edit_file" | "Edit" => format_edit_result(icon, &parsed),
        "glob_search" | "Glob" => format_glob_result(icon, &parsed),
        "grep_search" | "Grep" => format_grep_result(icon, &parsed),
        _ => format_generic_tool_result(icon, name, &parsed),
    }
}

const DISPLAY_TRUNCATION_NOTICE: &str =
    "\x1b[2m… output truncated for display; full result preserved in session.\x1b[0m";
const READ_DISPLAY_MAX_LINES: usize = 80;
const READ_DISPLAY_MAX_CHARS: usize = 6_000;
const TOOL_OUTPUT_DISPLAY_MAX_LINES: usize = 60;
const TOOL_OUTPUT_DISPLAY_MAX_CHARS: usize = 4_000;

fn extract_tool_path(parsed: &serde_json::Value) -> String {
    parsed
        .get("file_path")
        .or_else(|| parsed.get("filePath"))
        .or_else(|| parsed.get("path"))
        .and_then(|value| value.as_str())
        .unwrap_or("?")
        .to_string()
}

fn format_search_start(label: &str, parsed: &serde_json::Value) -> String {
    let pattern = parsed
        .get("pattern")
        .and_then(|value| value.as_str())
        .unwrap_or("?");
    let scope = parsed
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or(".");
    format!("{label} {pattern}\n\x1b[2min {scope}\x1b[0m")
}

fn format_patch_preview(old_value: &str, new_value: &str) -> Option<String> {
    if old_value.is_empty() && new_value.is_empty() {
        return None;
    }
    Some(format!(
        "\x1b[38;5;203m- {}\x1b[0m\n\x1b[38;5;70m+ {}\x1b[0m",
        truncate_for_summary(first_visible_line(old_value), 72),
        truncate_for_summary(first_visible_line(new_value), 72)
    ))
}

fn format_bash_call(parsed: &serde_json::Value) -> String {
    let command = parsed
        .get("command")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if command.is_empty() {
        String::new()
    } else {
        format!(
            "\x1b[48;5;236;38;5;255m $ {} \x1b[0m",
            truncate_for_summary(command, 160)
        )
    }
}

fn first_visible_line(text: &str) -> &str {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(text)
}

fn format_bash_result(icon: &str, parsed: &serde_json::Value) -> String {
    let mut lines = vec![format!("{icon} \x1b[38;5;245mbash\x1b[0m")];
    if let Some(task_id) = parsed
        .get("backgroundTaskId")
        .and_then(|value| value.as_str())
    {
        write!(&mut lines[0], " backgrounded ({task_id})").expect("write to string");
    } else if let Some(status) = parsed
        .get("returnCodeInterpretation")
        .and_then(|value| value.as_str())
        .filter(|status| !status.is_empty())
    {
        write!(&mut lines[0], " {status}").expect("write to string");
    }

    if let Some(stdout) = parsed.get("stdout").and_then(|value| value.as_str()) {
        if !stdout.trim().is_empty() {
            lines.push(truncate_output_for_display(
                stdout,
                TOOL_OUTPUT_DISPLAY_MAX_LINES,
                TOOL_OUTPUT_DISPLAY_MAX_CHARS,
            ));
        }
    }
    if let Some(stderr) = parsed.get("stderr").and_then(|value| value.as_str()) {
        if !stderr.trim().is_empty() {
            lines.push(format!(
                "\x1b[38;5;203m{}\x1b[0m",
                truncate_output_for_display(
                    stderr,
                    TOOL_OUTPUT_DISPLAY_MAX_LINES,
                    TOOL_OUTPUT_DISPLAY_MAX_CHARS,
                )
            ));
        }
    }

    lines.join("\n\n")
}

fn format_read_result(icon: &str, parsed: &serde_json::Value) -> String {
    let file = parsed.get("file").unwrap_or(parsed);
    let path = extract_tool_path(file);
    let start_line = file
        .get("startLine")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);
    let num_lines = file
        .get("numLines")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let total_lines = file
        .get("totalLines")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(num_lines);
    let content = file
        .get("content")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let end_line = start_line.saturating_add(num_lines.saturating_sub(1));

    format!(
        "{icon} \x1b[2m📄 Read {path} (lines {}-{} of {})\x1b[0m\n{}",
        start_line,
        end_line.max(start_line),
        total_lines,
        truncate_output_for_display(content, READ_DISPLAY_MAX_LINES, READ_DISPLAY_MAX_CHARS)
    )
}

fn format_write_result(icon: &str, parsed: &serde_json::Value) -> String {
    let path = extract_tool_path(parsed);
    let kind = parsed
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("write");
    let line_count = parsed
        .get("content")
        .and_then(|value| value.as_str())
        .map_or(0, |content| content.lines().count());
    format!(
        "{icon} \x1b[1;32m✏️ {} {path}\x1b[0m \x1b[2m({line_count} lines)\x1b[0m",
        if kind == "create" { "Wrote" } else { "Updated" },
    )
}

fn format_structured_patch_preview(parsed: &serde_json::Value) -> Option<String> {
    let hunks = parsed.get("structuredPatch")?.as_array()?;
    let mut preview = Vec::new();
    for hunk in hunks.iter().take(2) {
        let lines = hunk.get("lines")?.as_array()?;
        for line in lines.iter().filter_map(|value| value.as_str()).take(6) {
            match line.chars().next() {
                Some('+') => preview.push(format!("\x1b[38;5;70m{line}\x1b[0m")),
                Some('-') => preview.push(format!("\x1b[38;5;203m{line}\x1b[0m")),
                _ => preview.push(line.to_string()),
            }
        }
    }
    if preview.is_empty() {
        None
    } else {
        Some(preview.join("\n"))
    }
}

fn format_edit_result(icon: &str, parsed: &serde_json::Value) -> String {
    let path = extract_tool_path(parsed);
    let suffix = if parsed
        .get("replaceAll")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        " (replace all)"
    } else {
        ""
    };
    let preview = format_structured_patch_preview(parsed).or_else(|| {
        let old_value = parsed
            .get("oldString")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let new_value = parsed
            .get("newString")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        format_patch_preview(old_value, new_value)
    });

    match preview {
        Some(preview) => format!("{icon} \x1b[1;33m📝 Edited {path}{suffix}\x1b[0m\n{preview}"),
        None => format!("{icon} \x1b[1;33m📝 Edited {path}{suffix}\x1b[0m"),
    }
}

fn format_glob_result(icon: &str, parsed: &serde_json::Value) -> String {
    let num_files = parsed
        .get("numFiles")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let filenames = parsed
        .get("filenames")
        .and_then(|value| value.as_array())
        .map(|files| {
            files
                .iter()
                .filter_map(|value| value.as_str())
                .take(8)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if filenames.is_empty() {
        format!("{icon} \x1b[38;5;245mglob_search\x1b[0m matched {num_files} files")
    } else {
        format!("{icon} \x1b[38;5;245mglob_search\x1b[0m matched {num_files} files\n{filenames}")
    }
}

fn format_grep_result(icon: &str, parsed: &serde_json::Value) -> String {
    let num_matches = parsed
        .get("numMatches")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let num_files = parsed
        .get("numFiles")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let content = parsed
        .get("content")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let filenames = parsed
        .get("filenames")
        .and_then(|value| value.as_array())
        .map(|files| {
            files
                .iter()
                .filter_map(|value| value.as_str())
                .take(8)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let summary = format!(
        "{icon} \x1b[38;5;245mgrep_search\x1b[0m {num_matches} matches across {num_files} files"
    );
    if !content.trim().is_empty() {
        format!(
            "{summary}\n{}",
            truncate_output_for_display(
                content,
                TOOL_OUTPUT_DISPLAY_MAX_LINES,
                TOOL_OUTPUT_DISPLAY_MAX_CHARS,
            )
        )
    } else if !filenames.is_empty() {
        format!("{summary}\n{filenames}")
    } else {
        summary
    }
}

fn format_generic_tool_result(icon: &str, name: &str, parsed: &serde_json::Value) -> String {
    let rendered_output = match parsed {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
            serde_json::to_string_pretty(parsed).unwrap_or_else(|_| parsed.to_string())
        }
        _ => parsed.to_string(),
    };
    let preview = truncate_output_for_display(
        &rendered_output,
        TOOL_OUTPUT_DISPLAY_MAX_LINES,
        TOOL_OUTPUT_DISPLAY_MAX_CHARS,
    );

    if preview.is_empty() {
        format!("{icon} \x1b[38;5;245m{name}\x1b[0m")
    } else if preview.contains('\n') {
        format!("{icon} \x1b[38;5;245m{name}\x1b[0m\n{preview}")
    } else {
        format!("{icon} \x1b[38;5;245m{name}:\x1b[0m {preview}")
    }
}

fn summarize_tool_payload(payload: &str) -> String {
    let compact = match serde_json::from_str::<serde_json::Value>(payload) {
        Ok(value) => value.to_string(),
        Err(_) => payload.trim().to_string(),
    };
    truncate_for_summary(&compact, 96)
}

fn truncate_for_summary(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn truncate_output_for_display(content: &str, max_lines: usize, max_chars: usize) -> String {
    let original = content.trim_end_matches('\n');
    if original.is_empty() {
        return String::new();
    }

    let mut preview_lines = Vec::new();
    let mut used_chars = 0usize;
    let mut truncated = false;

    for (index, line) in original.lines().enumerate() {
        if index >= max_lines {
            truncated = true;
            break;
        }

        let newline_cost = usize::from(!preview_lines.is_empty());
        let available = max_chars.saturating_sub(used_chars + newline_cost);
        if available == 0 {
            truncated = true;
            break;
        }

        let line_chars = line.chars().count();
        if line_chars > available {
            preview_lines.push(line.chars().take(available).collect::<String>());
            truncated = true;
            break;
        }

        preview_lines.push(line.to_string());
        used_chars += newline_cost + line_chars;
    }

    let mut preview = preview_lines.join("\n");
    if truncated {
        if !preview.is_empty() {
            preview.push('\n');
        }
        preview.push_str(DISPLAY_TRUNCATION_NOTICE);
    }
    preview
}

fn push_output_block(
    block: OutputContentBlock,
    out: &mut (impl Write + ?Sized),
    events: &mut Vec<AssistantEvent>,
    pending_tool: &mut Option<(String, String, String)>,
    streaming_tool_input: bool,
) -> Result<(), RuntimeError> {
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
                let rendered = TerminalRenderer::new().markdown_to_ansi(&text);
                write!(out, "{rendered}")
                    .and_then(|()| out.flush())
                    .map_err(|error| RuntimeError::new(error.to_string()))?;
                events.push(AssistantEvent::TextDelta(text));
            }
        }
        OutputContentBlock::ToolUse { id, name, input } => {
            // During streaming, the initial content_block_start has an empty input ({}).
            // The real input arrives via input_json_delta events. In
            // non-streaming responses, preserve a legitimate empty object.
            let initial_input = if streaming_tool_input
                && input.is_object()
                && input.as_object().is_some_and(serde_json::Map::is_empty)
            {
                String::new()
            } else {
                input.to_string()
            };
            *pending_tool = Some((id, name, initial_input));
        }
        OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
    }
    Ok(())
}

fn response_to_events(
    response: MessageResponse,
    out: &mut (impl Write + ?Sized),
) -> Result<Vec<AssistantEvent>, RuntimeError> {
    let mut events = Vec::new();
    let mut pending_tool = None;

    for block in response.content {
        push_output_block(block, out, &mut events, &mut pending_tool, false)?;
        if let Some((id, name, input)) = pending_tool.take() {
            events.push(AssistantEvent::ToolUse { id, name, input });
        }
    }

    events.push(AssistantEvent::Usage(TokenUsage {
        input_tokens: response.usage.input_tokens,
        output_tokens: response.usage.output_tokens,
        cache_creation_input_tokens: response.usage.cache_creation_input_tokens,
        cache_read_input_tokens: response.usage.cache_read_input_tokens,
    }));
    events.push(AssistantEvent::MessageStop);
    Ok(events)
}

struct CliToolExecutor {
    renderer: TerminalRenderer,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
}

impl CliToolExecutor {
    fn new(
        allowed_tools: Option<AllowedToolSet>,
        emit_output: bool,
        tool_registry: GlobalToolRegistry,
    ) -> Self {
        Self {
            renderer: TerminalRenderer::new(),
            emit_output,
            allowed_tools,
            tool_registry,
        }
    }
}

impl ToolExecutor for CliToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        if self
            .allowed_tools
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(tool_name))
        {
            return Err(ToolError::new(format!(
                "tool `{tool_name}` is not enabled by the current --allowedTools setting"
            )));
        }
        let value = serde_json::from_str(input)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
        match self.tool_registry.execute(tool_name, &value) {
            Ok(output) => {
                if self.emit_output {
                    let markdown = format_tool_result(tool_name, &output, false);
                    self.renderer
                        .stream_markdown(&markdown, &mut io::stdout())
                        .map_err(|error| ToolError::new(error.to_string()))?;
                }
                Ok(output)
            }
            Err(error) => {
                if self.emit_output {
                    let markdown = format_tool_result(tool_name, &error, true);
                    self.renderer
                        .stream_markdown(&markdown, &mut io::stdout())
                        .map_err(|stream_error| ToolError::new(stream_error.to_string()))?;
                }
                Err(ToolError::new(error))
            }
        }
    }
}

fn permission_policy(mode: PermissionMode, tool_registry: &GlobalToolRegistry) -> PermissionPolicy {
    tool_registry.permission_specs(None).into_iter().fold(
        PermissionPolicy::new(mode),
        |policy, (name, required_permission)| {
            policy.with_tool_requirement(name, required_permission)
        },
    )
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

fn print_help_to(out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "openanalyst v{VERSION}")?;
    writeln!(out)?;
    writeln!(out, "Usage:")?;
    writeln!(
        out,
        "  openanalyst [--model MODEL] [--allowedTools TOOL[,TOOL...]]"
    )?;
    writeln!(out, "      Start the interactive REPL")?;
    writeln!(
        out,
        "  openanalyst [--model MODEL] [--output-format text|json] prompt TEXT"
    )?;
    writeln!(out, "      Send one prompt and exit")?;
    writeln!(
        out,
        "  openanalyst [--model MODEL] [--output-format text|json] TEXT"
    )?;
    writeln!(out, "      Shorthand non-interactive prompt mode")?;
    writeln!(
        out,
        "  openanalyst --resume SESSION.json [/status] [/compact] [...]"
    )?;
    writeln!(
        out,
        "      Inspect or maintain a saved session without entering the REPL"
    )?;
    writeln!(out, "  openanalyst dump-manifests")?;
    writeln!(out, "  openanalyst bootstrap-plan")?;
    writeln!(out, "  openanalyst agents")?;
    writeln!(out, "  openanalyst skills")?;
    writeln!(out, "  openanalyst system-prompt [--cwd PATH] [--date YYYY-MM-DD]")?;
    writeln!(out, "  openanalyst agent run [--max-turns N] [--verbose] TASK")?;
    writeln!(
        out,
        "      Run an autonomous agent that completes the task without interaction"
    )?;
    writeln!(out, "  openanalyst login")?;
    writeln!(out, "  openanalyst logout")?;
    writeln!(out, "  openanalyst whoami")?;
    writeln!(out, "  openanalyst update")?;
    writeln!(out, "  openanalyst uninstall")?;
    writeln!(out, "  openanalyst init")?;
    writeln!(out)?;
    writeln!(out, "Flags:")?;
    writeln!(
        out,
        "  --model MODEL              Override the active model"
    )?;
    writeln!(
        out,
        "  --output-format FORMAT     Non-interactive output format: text or json"
    )?;
    writeln!(
        out,
        "  --permission-mode MODE     Set read-only, workspace-write, or danger-full-access"
    )?;
    writeln!(
        out,
        "  --dangerously-skip-permissions  Skip all permission checks"
    )?;
    writeln!(out, "  --allowedTools TOOLS       Restrict enabled tools (repeatable; comma-separated aliases supported)")?;
    writeln!(
        out,
        "  --version, -V              Print version and build information locally"
    )?;
    writeln!(out)?;
    writeln!(out, "Interactive slash commands:")?;
    writeln!(out, "{}", render_slash_command_help())?;
    writeln!(out)?;
    let resume_commands = resume_supported_slash_commands()
        .into_iter()
        .map(|spec| match spec.argument_hint {
            Some(argument_hint) => format!("/{} {}", spec.name, argument_hint),
            None => format!("/{}", spec.name),
        })
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "Resume-safe commands: {resume_commands}")?;
    writeln!(out, "Examples:")?;
    writeln!(out, "  openanalyst --model opus \"summarize this repo\"")?;
    writeln!(
        out,
        "  openanalyst --output-format json prompt \"explain src/main.rs\""
    )?;
    writeln!(
        out,
        "  openanalyst --allowedTools read,glob \"summarize Cargo.toml\""
    )?;
    writeln!(
        out,
        "  openanalyst --resume session.json /status /diff /export notes.txt"
    )?;
    writeln!(out, "  openanalyst agents")?;
    writeln!(out, "  openanalyst /skills")?;
    writeln!(out, "  openanalyst agent run --model gemini-2.5-pro \"fix the failing tests\"")?;
    writeln!(out, "  openanalyst login")?;
    writeln!(out, "  openanalyst init")?;
    Ok(())
}

fn print_help() {
    let _ = print_help_to(&mut io::stdout());
}

#[cfg(test)]
mod tests {
    use super::{
        describe_tool_progress, filter_tool_specs, format_compact_report, format_cost_report,
        format_internal_prompt_progress_line, format_model_report, format_model_switch_report,
        format_permissions_report, format_permissions_switch_report, format_resume_report,
        format_status_report, format_tool_call_start, format_tool_result,
        normalize_permission_mode, parse_args, parse_git_status_metadata, permission_policy,
        print_help_to, push_output_block, render_config_report, render_memory_report,
        render_repl_help, resolve_model_alias, response_to_events, resume_supported_slash_commands,
        status_context, CliAction, CliOutputFormat, InternalPromptProgressEvent,
        InternalPromptProgressState, SlashCommand, StatusUsage, DEFAULT_MODEL,
    };
    use api::{MessageResponse, OutputContentBlock, Usage};
    use plugins::{PluginTool, PluginToolDefinition, PluginToolPermission};
    use runtime::{AssistantEvent, ContentBlock, ConversationMessage, MessageRole, PermissionMode};
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::Duration;
    use tools::GlobalToolRegistry;

    fn registry_with_plugin_tool() -> GlobalToolRegistry {
        GlobalToolRegistry::with_plugin_tools(vec![PluginTool::new(
            "plugin-demo@external",
            "plugin-demo",
            PluginToolDefinition {
                name: "plugin_echo".to_string(),
                description: Some("Echo plugin payload".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" }
                    },
                    "required": ["message"],
                    "additionalProperties": false
                }),
            },
            "echo".to_string(),
            Vec::new(),
            PluginToolPermission::WorkspaceWrite,
            None,
        )])
        .expect("plugin tool registry should build")
    }

    #[test]
    fn defaults_to_repl_when_no_args() {
        assert_eq!(
            parse_args(&[]).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
                use_tui: true,
            }
        );
    }

    #[test]
    fn parses_prompt_subcommand() {
        let args = vec![
            "prompt".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "hello world".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn parses_bare_prompt_and_json_output_flag() {
        let args = vec![
            "--output-format=json".to_string(),
            "--model".to_string(),
            "custom-opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "custom-opus".to_string(),
                output_format: CliOutputFormat::Json,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn resolves_model_aliases_in_args() {
        let args = vec![
            "--model".to_string(),
            "opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "claude-opus-4-6".to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
            }
        );
    }

    #[test]
    fn resolves_known_model_aliases() {
        assert_eq!(resolve_model_alias("opus"), "claude-opus-4-6");
        assert_eq!(resolve_model_alias("sonnet"), "claude-sonnet-4-6");
        assert_eq!(resolve_model_alias("haiku"), "claude-haiku-4-5-20251213");
        assert_eq!(resolve_model_alias("custom-opus"), "custom-opus");
    }

    #[test]
    fn parses_version_flags_without_initializing_prompt_mode() {
        assert_eq!(
            parse_args(&["--version".to_string()]).expect("args should parse"),
            CliAction::Version
        );
        assert_eq!(
            parse_args(&["-V".to_string()]).expect("args should parse"),
            CliAction::Version
        );
    }

    #[test]
    fn parses_permission_mode_flag() {
        let args = vec!["--permission-mode=read-only".to_string()];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                allowed_tools: None,
                permission_mode: PermissionMode::ReadOnly,
                use_tui: true,
            }
        );
    }

    #[test]
    fn parses_allowed_tools_flags_with_aliases_and_lists() {
        let args = vec![
            "--allowedTools".to_string(),
            "read,glob".to_string(),
            "--allowed-tools=write_file".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                allowed_tools: Some(
                    ["glob_search", "read_file", "write_file"]
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                ),
                permission_mode: PermissionMode::DangerFullAccess,
                use_tui: true,
            }
        );
    }

    #[test]
    fn rejects_unknown_allowed_tools() {
        let error = parse_args(&["--allowedTools".to_string(), "teleport".to_string()])
            .expect_err("tool should be rejected");
        assert!(error.contains("unsupported tool in --allowedTools: teleport"));
    }

    #[test]
    fn parses_system_prompt_options() {
        let args = vec![
            "system-prompt".to_string(),
            "--cwd".to_string(),
            "/tmp/project".to_string(),
            "--date".to_string(),
            "2026-04-01".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::PrintSystemPrompt {
                cwd: PathBuf::from("/tmp/project"),
                date: "2026-04-01".to_string(),
            }
        );
    }

    #[test]
    fn parses_login_and_logout_subcommands() {
        assert_eq!(
            parse_args(&["login".to_string()]).expect("login should parse"),
            CliAction::Login
        );
        assert_eq!(
            parse_args(&["logout".to_string()]).expect("logout should parse"),
            CliAction::Logout
        );
        assert_eq!(
            parse_args(&["whoami".to_string()]).expect("whoami should parse"),
            CliAction::WhoAmI
        );
        assert_eq!(
            parse_args(&["init".to_string()]).expect("init should parse"),
            CliAction::Init
        );
        assert_eq!(
            parse_args(&["agents".to_string()]).expect("agents should parse"),
            CliAction::Agents { args: None }
        );
        assert_eq!(
            parse_args(&["skills".to_string()]).expect("skills should parse"),
            CliAction::Skills { args: None }
        );
        assert_eq!(
            parse_args(&["agents".to_string(), "--help".to_string()])
                .expect("agents help should parse"),
            CliAction::Agents {
                args: Some("--help".to_string())
            }
        );
    }

    #[test]
    fn parses_agent_run_subcommand() {
        let result = parse_args(&[
            "agent".to_string(),
            "run".to_string(),
            "fix".to_string(),
            "the".to_string(),
            "bug".to_string(),
        ])
        .expect("agent run should parse");
        match result {
            CliAction::Agent {
                task,
                max_turns,
                verbose,
                ..
            } => {
                assert_eq!(task, "fix the bug");
                assert_eq!(max_turns, 30);
                assert!(!verbose);
            }
            other => panic!("expected Agent, got {other:?}"),
        }
    }

    #[test]
    fn parses_agent_run_with_flags() {
        let result = parse_args(&[
            "agent".to_string(),
            "run".to_string(),
            "--max-turns".to_string(),
            "10".to_string(),
            "--verbose".to_string(),
            "--model".to_string(),
            "gemini-2.5-pro".to_string(),
            "deploy".to_string(),
        ])
        .expect("agent run with flags should parse");
        match result {
            CliAction::Agent {
                task,
                model,
                max_turns,
                verbose,
                ..
            } => {
                assert_eq!(task, "deploy");
                assert_eq!(model, "gemini-2.5-pro");
                assert_eq!(max_turns, 10);
                assert!(verbose);
            }
            other => panic!("expected Agent, got {other:?}"),
        }
    }

    #[test]
    fn agent_without_run_shows_error() {
        assert!(parse_args(&["agent".to_string()]).is_err());
    }

    #[test]
    fn agent_run_without_task_shows_error() {
        assert!(parse_args(&["agent".to_string(), "run".to_string()]).is_err());
    }

    #[test]
    fn parses_direct_agents_and_skills_slash_commands() {
        assert_eq!(
            parse_args(&["/agents".to_string()]).expect("/agents should parse"),
            CliAction::Agents { args: None }
        );
        assert_eq!(
            parse_args(&["/skills".to_string()]).expect("/skills should parse"),
            CliAction::Skills { args: None }
        );
        assert_eq!(
            parse_args(&["/skills".to_string(), "help".to_string()])
                .expect("/skills help should parse"),
            CliAction::Skills {
                args: Some("help".to_string())
            }
        );
        let error = parse_args(&["/status".to_string()])
            .expect_err("/status should remain REPL-only when invoked directly");
        assert!(error.contains("unsupported direct slash command"));
    }

    #[test]
    fn parses_resume_flag_with_slash_command() {
        let args = vec![
            "--resume".to_string(),
            "session.json".to_string(),
            "/compact".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.json"),
                commands: vec!["/compact".to_string()],
            }
        );
    }

    #[test]
    fn parses_resume_flag_with_multiple_slash_commands() {
        let args = vec![
            "--resume".to_string(),
            "session.json".to_string(),
            "/status".to_string(),
            "/compact".to_string(),
            "/cost".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.json"),
                commands: vec![
                    "/status".to_string(),
                    "/compact".to_string(),
                    "/cost".to_string(),
                ],
            }
        );
    }

    #[test]
    fn filtered_tool_specs_respect_allowlist() {
        let allowed = ["read_file", "grep_search"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let filtered = filter_tool_specs(&GlobalToolRegistry::builtin(), Some(&allowed));
        let names = filtered
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["read_file", "grep_search"]);
    }

    #[test]
    fn filtered_tool_specs_include_plugin_tools() {
        let filtered = filter_tool_specs(&registry_with_plugin_tool(), None);
        let names = filtered
            .into_iter()
            .map(|definition| definition.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"bash".to_string()));
        assert!(names.contains(&"plugin_echo".to_string()));
    }

    #[test]
    fn permission_policy_uses_plugin_tool_permissions() {
        let policy = permission_policy(PermissionMode::ReadOnly, &registry_with_plugin_tool());
        let required = policy.required_mode_for("plugin_echo");
        assert_eq!(required, PermissionMode::WorkspaceWrite);
    }

    #[test]
    fn shared_help_uses_resume_annotation_copy() {
        let help = commands::render_slash_command_help();
        assert!(help.contains("Slash commands"));
        assert!(help.contains("works with --resume SESSION.json"));
    }

    #[test]
    fn repl_help_includes_shared_commands_and_exit() {
        let help = render_repl_help();
        assert!(help.contains("REPL"));
        assert!(help.contains("/help"));
        assert!(help.contains("/status"));
        assert!(help.contains("/model [model]"));
        assert!(help.contains("/permissions [read-only|workspace-write|danger-full-access]"));
        assert!(help.contains("/clear [--confirm]"));
        assert!(help.contains("/cost"));
        assert!(help.contains("/resume <session-path>"));
        assert!(help.contains("/config [env|hooks|model|plugins]"));
        assert!(help.contains("/memory"));
        assert!(help.contains("/init"));
        assert!(help.contains("/diff"));
        assert!(help.contains("/version"));
        assert!(help.contains("/export [file]"));
        assert!(help.contains("/session [list|switch <session-id>]"));
        assert!(help.contains(
            "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]"
        ));
        assert!(help.contains("aliases: /plugins, /marketplace"));
        assert!(help.contains("/agents"));
        assert!(help.contains("/skills"));
        assert!(help.contains("/exit"));
    }

    #[test]
    fn resume_supported_command_list_matches_expected_surface() {
        let names = resume_supported_slash_commands()
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "help", "status", "compact", "clear", "cost", "config", "memory", "init", "diff",
                "version", "export", "agents", "skills", "context",
            ]
        );
    }

    #[test]
    fn resume_report_uses_sectioned_layout() {
        let report = format_resume_report("session.json", 14, 6);
        assert!(report.contains("Session resumed"));
        assert!(report.contains("Session file     session.json"));
        assert!(report.contains("Messages         14"));
        assert!(report.contains("Turns            6"));
    }

    #[test]
    fn compact_report_uses_structured_output() {
        let compacted = format_compact_report(8, 5, false);
        assert!(compacted.contains("Compact"));
        assert!(compacted.contains("Result           compacted"));
        assert!(compacted.contains("Messages removed 8"));
        let skipped = format_compact_report(0, 3, true);
        assert!(skipped.contains("Result           skipped"));
    }

    #[test]
    fn cost_report_uses_sectioned_layout() {
        let report = format_cost_report(runtime::TokenUsage {
            input_tokens: 20,
            output_tokens: 8,
            cache_creation_input_tokens: 3,
            cache_read_input_tokens: 1,
        });
        assert!(report.contains("Cost"));
        assert!(report.contains("Input tokens     20"));
        assert!(report.contains("Output tokens    8"));
        assert!(report.contains("Cache create     3"));
        assert!(report.contains("Cache read       1"));
        assert!(report.contains("Total tokens     32"));
    }

    #[test]
    fn permissions_report_uses_sectioned_layout() {
        let report = format_permissions_report("workspace-write");
        assert!(report.contains("Permissions"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Modes"));
        assert!(report.contains("read-only          ○ available Read/search tools only"));
        assert!(report.contains("workspace-write    ● current   Edit files inside the workspace"));
        assert!(report.contains("danger-full-access ○ available Unrestricted tool access"));
    }

    #[test]
    fn permissions_switch_report_is_structured() {
        let report = format_permissions_switch_report("read-only", "workspace-write");
        assert!(report.contains("Permissions updated"));
        assert!(report.contains("Result           mode switched"));
        assert!(report.contains("Previous mode    read-only"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Applies to       subsequent tool calls"));
    }

    #[test]
    fn init_help_mentions_direct_subcommand() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        assert!(help.contains("openanalyst init"));
        assert!(help.contains("openanalyst agents"));
        assert!(help.contains("openanalyst skills"));
        assert!(help.contains("openanalyst /skills"));
    }

    #[test]
    fn model_report_uses_sectioned_layout() {
        let report = format_model_report("sonnet", 12, 4);
        assert!(report.contains("Model"));
        assert!(report.contains("Current model    sonnet"));
        assert!(report.contains("Session messages 12"));
        assert!(report.contains("Switch models with /model <name>"));
    }

    #[test]
    fn model_switch_report_preserves_context_summary() {
        let report = format_model_switch_report("sonnet", "opus", 9);
        assert!(report.contains("Model updated"));
        assert!(report.contains("Previous         sonnet"));
        assert!(report.contains("Current          opus"));
        assert!(report.contains("Preserved msgs   9"));
    }

    #[test]
    fn status_line_reports_model_and_token_totals() {
        let status = format_status_report(
            "sonnet",
            StatusUsage {
                message_count: 7,
                turns: 3,
                latest: runtime::TokenUsage {
                    input_tokens: 5,
                    output_tokens: 4,
                    cache_creation_input_tokens: 1,
                    cache_read_input_tokens: 0,
                },
                cumulative: runtime::TokenUsage {
                    input_tokens: 20,
                    output_tokens: 8,
                    cache_creation_input_tokens: 2,
                    cache_read_input_tokens: 1,
                },
                estimated_tokens: 128,
            },
            "workspace-write",
            &super::StatusContext {
                cwd: PathBuf::from("/tmp/project"),
                session_path: Some(PathBuf::from("session.json")),
                loaded_config_files: 2,
                discovered_config_files: 3,
                memory_file_count: 4,
                project_root: Some(PathBuf::from("/tmp")),
                git_branch: Some("main".to_string()),
            },
        );
        assert!(status.contains("Status"));
        assert!(status.contains("Model            sonnet"));
        assert!(status.contains("Permission mode  workspace-write"));
        assert!(status.contains("Messages         7"));
        assert!(status.contains("Latest total     10"));
        assert!(status.contains("Cumulative total 31"));
        assert!(status.contains("Cwd              /tmp/project"));
        assert!(status.contains("Project root     /tmp"));
        assert!(status.contains("Git branch       main"));
        assert!(status.contains("Session          session.json"));
        assert!(status.contains("Config files     loaded 2/3"));
        assert!(status.contains("Memory files     4"));
    }

    #[test]
    fn config_report_supports_section_views() {
        let report = render_config_report(Some("env")).expect("config report should render");
        assert!(report.contains("Merged section: env"));
        let plugins_report =
            render_config_report(Some("plugins")).expect("plugins config report should render");
        assert!(plugins_report.contains("Merged section: plugins"));
    }

    #[test]
    fn memory_report_uses_sectioned_layout() {
        let report = render_memory_report().expect("memory report should render");
        assert!(report.contains("Memory"));
        assert!(report.contains("Working directory"));
        assert!(report.contains("Instruction files"));
        assert!(report.contains("Discovered files"));
    }

    #[test]
    fn config_report_uses_sectioned_layout() {
        let report = render_config_report(None).expect("config report should render");
        assert!(report.contains("Config"));
        assert!(report.contains("Discovered files"));
        assert!(report.contains("Merged JSON"));
    }

    #[test]
    fn parses_git_status_metadata() {
        let (root, branch) = parse_git_status_metadata(Some(
            "## rcc/cli...origin/rcc/cli
 M src/main.rs",
        ));
        assert_eq!(branch.as_deref(), Some("rcc/cli"));
        let _ = root;
    }

    #[test]
    fn status_context_reads_real_workspace_metadata() {
        let context = status_context(None).expect("status context should load");
        assert!(context.cwd.is_absolute());
        assert_eq!(context.discovered_config_files, 5);
        assert!(context.loaded_config_files <= context.discovered_config_files);
    }

    #[test]
    fn normalizes_supported_permission_modes() {
        assert_eq!(normalize_permission_mode("read-only"), Some("read-only"));
        assert_eq!(
            normalize_permission_mode("workspace-write"),
            Some("workspace-write")
        );
        assert_eq!(
            normalize_permission_mode("danger-full-access"),
            Some("danger-full-access")
        );
        assert_eq!(normalize_permission_mode("unknown"), None);
    }

    #[test]
    fn clear_command_requires_explicit_confirmation_flag() {
        assert_eq!(
            SlashCommand::parse("/clear"),
            Some(SlashCommand::Clear { confirm: false })
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Some(SlashCommand::Clear { confirm: true })
        );
    }

    #[test]
    fn parses_resume_and_config_slash_commands() {
        assert_eq!(
            SlashCommand::parse("/resume saved-session.json"),
            Some(SlashCommand::Resume {
                session_path: Some("saved-session.json".to_string())
            })
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Some(SlashCommand::Clear { confirm: true })
        );
        assert_eq!(
            SlashCommand::parse("/config"),
            Some(SlashCommand::Config { section: None })
        );
        assert_eq!(
            SlashCommand::parse("/config env"),
            Some(SlashCommand::Config {
                section: Some("env".to_string())
            })
        );
        assert_eq!(SlashCommand::parse("/memory"), Some(SlashCommand::Memory));
        assert_eq!(SlashCommand::parse("/init"), Some(SlashCommand::Init));
    }

    #[test]
    fn init_template_mentions_detected_rust_workspace() {
        let rendered = crate::init::render_init_openanalyst_md(std::path::Path::new("."));
        assert!(rendered.contains("# OPENANALYST.md"));
        assert!(rendered.contains("cargo clippy --workspace --all-targets -- -D warnings"));
    }

    #[test]
    fn converts_tool_roundtrip_messages() {
        let messages = vec![
            ConversationMessage::user_text("hello"),
            ConversationMessage::assistant(vec![ContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "bash".to_string(),
                input: "{\"command\":\"pwd\"}".to_string(),
            }]),
            ConversationMessage {
                role: MessageRole::Tool,
                blocks: vec![ContentBlock::ToolResult {
                    tool_use_id: "tool-1".to_string(),
                    tool_name: "bash".to_string(),
                    output: "ok".to_string(),
                    is_error: false,
                }],
                usage: None,
            },
        ];

        let converted = super::convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[1].role, "assistant");
        assert_eq!(converted[2].role, "user");
    }
    #[test]
    fn repl_help_mentions_history_completion_and_multiline() {
        let help = render_repl_help();
        assert!(help.contains("Up/Down"));
        assert!(help.contains("Tab"));
        assert!(help.contains("Shift+Enter/Ctrl+J"));
    }

    #[test]
    fn tool_rendering_helpers_compact_output() {
        let start = format_tool_call_start("read_file", r#"{"path":"src/main.rs"}"#);
        assert!(start.contains("read_file"));
        assert!(start.contains("src/main.rs"));

        let done = format_tool_result(
            "read_file",
            r#"{"file":{"filePath":"src/main.rs","content":"hello","numLines":1,"startLine":1,"totalLines":1}}"#,
            false,
        );
        assert!(done.contains("📄 Read src/main.rs"));
        assert!(done.contains("hello"));
    }

    #[test]
    fn tool_rendering_truncates_large_read_output_for_display_only() {
        let content = (0..200)
            .map(|index| format!("line {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");
        let output = json!({
            "file": {
                "filePath": "src/main.rs",
                "content": content,
                "numLines": 200,
                "startLine": 1,
                "totalLines": 200
            }
        })
        .to_string();

        let rendered = format_tool_result("read_file", &output, false);

        assert!(rendered.contains("line 000"));
        assert!(rendered.contains("line 079"));
        assert!(!rendered.contains("line 199"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("line 199"));
    }

    #[test]
    fn tool_rendering_truncates_large_bash_output_for_display_only() {
        let stdout = (0..120)
            .map(|index| format!("stdout {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");
        let output = json!({
            "stdout": stdout,
            "stderr": "",
            "returnCodeInterpretation": "completed successfully"
        })
        .to_string();

        let rendered = format_tool_result("bash", &output, false);

        assert!(rendered.contains("stdout 000"));
        assert!(rendered.contains("stdout 059"));
        assert!(!rendered.contains("stdout 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("stdout 119"));
    }

    #[test]
    fn tool_rendering_truncates_generic_long_output_for_display_only() {
        let items = (0..120)
            .map(|index| format!("payload {index:03}"))
            .collect::<Vec<_>>();
        let output = json!({
            "summary": "plugin payload",
            "items": items,
        })
        .to_string();

        let rendered = format_tool_result("plugin_echo", &output, false);

        assert!(rendered.contains("plugin_echo"));
        assert!(rendered.contains("payload 000"));
        assert!(rendered.contains("payload 040"));
        assert!(!rendered.contains("payload 080"));
        assert!(!rendered.contains("payload 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("payload 119"));
    }

    #[test]
    fn tool_rendering_truncates_raw_generic_output_for_display_only() {
        let output = (0..120)
            .map(|index| format!("raw {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");

        let rendered = format_tool_result("plugin_echo", &output, false);

        assert!(rendered.contains("plugin_echo"));
        assert!(rendered.contains("raw 000"));
        assert!(rendered.contains("raw 059"));
        assert!(!rendered.contains("raw 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("raw 119"));
    }

    #[test]
    fn ultraplan_progress_lines_include_phase_step_and_elapsed_status() {
        let snapshot = InternalPromptProgressState {
            command_label: "Ultraplan",
            task_label: "ship plugin progress".to_string(),
            step: 3,
            phase: "running read_file".to_string(),
            detail: Some("reading rust/crates/openanalyst-cli/src/main.rs".to_string()),
            saw_final_text: false,
        };

        let started = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Started,
            &snapshot,
            Duration::from_secs(0),
            None,
        );
        let heartbeat = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Heartbeat,
            &snapshot,
            Duration::from_secs(9),
            None,
        );
        let completed = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Complete,
            &snapshot,
            Duration::from_secs(12),
            None,
        );
        let failed = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Failed,
            &snapshot,
            Duration::from_secs(12),
            Some("network timeout"),
        );

        assert!(started.contains("planning started"));
        assert!(started.contains("current step 3"));
        assert!(heartbeat.contains("heartbeat"));
        assert!(heartbeat.contains("9s elapsed"));
        assert!(heartbeat.contains("phase running read_file"));
        assert!(completed.contains("completed"));
        assert!(completed.contains("3 steps total"));
        assert!(failed.contains("failed"));
        assert!(failed.contains("network timeout"));
    }

    #[test]
    fn describe_tool_progress_summarizes_known_tools() {
        assert_eq!(
            describe_tool_progress("read_file", r#"{"path":"src/main.rs"}"#),
            "reading src/main.rs"
        );
        assert!(
            describe_tool_progress("bash", r#"{"command":"cargo test -p openanalyst-cli"}"#)
                .contains("cargo test -p openanalyst-cli")
        );
        assert_eq!(
            describe_tool_progress("grep_search", r#"{"pattern":"ultraplan","path":"rust"}"#),
            "grep `ultraplan` in rust"
        );
    }

    #[test]
    fn push_output_block_renders_markdown_text() {
        let mut out = Vec::new();
        let mut events = Vec::new();
        let mut pending_tool = None;

        push_output_block(
            OutputContentBlock::Text {
                text: "# Heading".to_string(),
            },
            &mut out,
            &mut events,
            &mut pending_tool,
            false,
        )
        .expect("text block should render");

        let rendered = String::from_utf8(out).expect("utf8");
        assert!(rendered.contains("Heading"));
        assert!(rendered.contains('\u{1b}'));
    }

    #[test]
    fn push_output_block_skips_empty_object_prefix_for_tool_streams() {
        let mut out = Vec::new();
        let mut events = Vec::new();
        let mut pending_tool = None;

        push_output_block(
            OutputContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "read_file".to_string(),
                input: json!({}),
            },
            &mut out,
            &mut events,
            &mut pending_tool,
            true,
        )
        .expect("tool block should accumulate");

        assert!(events.is_empty());
        assert_eq!(
            pending_tool,
            Some(("tool-1".to_string(), "read_file".to_string(), String::new(),))
        );
    }

    #[test]
    fn response_to_events_preserves_empty_object_json_input_outside_streaming() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-1".to_string(),
                kind: "message".to_string(),
                model: "claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![OutputContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "read_file".to_string(),
                    input: json!({}),
                }],
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::ToolUse { name, input, .. }
                if name == "read_file" && input == "{}"
        ));
    }

    #[test]
    fn response_to_events_preserves_non_empty_json_input_outside_streaming() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-2".to_string(),
                kind: "message".to_string(),
                model: "claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![OutputContentBlock::ToolUse {
                    id: "tool-2".to_string(),
                    name: "read_file".to_string(),
                    input: json!({ "path": "rust/Cargo.toml" }),
                }],
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::ToolUse { name, input, .. }
                if name == "read_file" && input == "{\"path\":\"rust/Cargo.toml\"}"
        ));
    }

    #[test]
    fn response_to_events_ignores_thinking_blocks() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-3".to_string(),
                kind: "message".to_string(),
                model: "claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![
                    OutputContentBlock::Thinking {
                        thinking: "step 1".to_string(),
                        signature: Some("sig_123".to_string()),
                    },
                    OutputContentBlock::Text {
                        text: "Final answer".to_string(),
                    },
                ],
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::TextDelta(text) if text == "Final answer"
        ));
        assert!(!String::from_utf8(out).expect("utf8").contains("step 1"));
    }
}
