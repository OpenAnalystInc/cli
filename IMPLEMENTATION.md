# OpenAnalyst CLI — Rebranding Implementation Log

## Overview

Complete rebranding of the "Claw Code" CLI agent harness to **OpenAnalyst CLI**, including API endpoint configuration, authentication, visual identity, and all source code references.

---

## 1. Binary & Package Naming

| Item | Before | After |
|------|--------|-------|
| CLI binary name | `claw` | `openanalyst` |
| CLI crate name | `claw-cli` | `openanalyst-cli` |
| Crate directory | `rust/crates/claw-cli/` | `rust/crates/openanalyst-cli/` |
| Workspace version | `0.1.0` | `1.0.1` |

**Files changed:**
- `rust/crates/openanalyst-cli/Cargo.toml` — `name = "openanalyst-cli"`, `[[bin]] name = "openanalyst"`
- `rust/Cargo.toml` — `version = "1.0.1"`

---

## 2. API Provider & Authentication

| Item | Before | After |
|------|--------|-------|
| Default base URL | `https://api.anthropic.com` | `https://api.openanalyst.com/api` |
| Default model | `claude-opus-4-6` | `openanalyst-beta` |
| Provider file | `claw_provider.rs` | `openanalyst_provider.rs` |
| Client struct | `ClawApiClient` | `OpenAnalystApiClient` |
| Provider enum | `ProviderKind::ClawApi` | `ProviderKind::OpenAnalystApi` |
| Provider client enum | `ProviderClient::ClawApi` | `ProviderClient::OpenAnalystApi` |
| API version header | `2023-06-01` (unchanged) | `2023-06-01` (unchanged) |

**Dual authentication support added:**
- `OPENANALYST_API_KEY` / `OPENANALYST_AUTH_TOKEN` / `OPENANALYST_BASE_URL` (primary)
- `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN` / `ANTHROPIC_BASE_URL` (fallback)
- Auth resolution: tries OpenAnalyst env vars first, falls back to Anthropic env vars

**Model registry additions:**
- Added `openanalyst-beta` and `openanalyst` as model aliases (resolve to `openanalyst-beta`)
- Kept all existing Claude and Grok model aliases

**Files changed:**
- `rust/crates/api/src/providers/openanalyst_provider.rs` — base URL, auth, client naming, dual auth fallback
- `rust/crates/api/src/providers/mod.rs` — provider enum, model registry, env var names
- `rust/crates/api/src/lib.rs` — re-exports updated
- `rust/crates/api/src/client.rs` — provider enum variants renamed
- `rust/crates/api/tests/client_integration.rs` — test strings updated
- `rust/crates/api/tests/provider_client_integration.rs` — test strings updated

---

## 3. ASCII Banner & Visual Identity

| Item | Before | After |
|------|--------|-------|
| Banner text | `CLAW` (block letters) | `OA` (block letters) + `OpenAnalyst CLI` |
| Banner color | Red (`\x1b[38;5;196m`) | Blue (`\x1b[38;5;39m`) |
| Accent color | Orange (`\x1b[38;5;208m`) | Turquoise (`\x1b[38;5;45m`) |
| Emoji mascot | 🦞 (lobster) | 📊 (chart) |
| Spinner text | `🦀 Thinking...` | `📊 Thinking...` |

**File changed:**
- `rust/crates/openanalyst-cli/src/main.rs` — `startup_banner()` function

---

## 4. Environment Variables

| Before | After |
|--------|-------|
| `ANTHROPIC_API_KEY` | `OPENANALYST_API_KEY` (+ `ANTHROPIC_API_KEY` fallback) |
| `ANTHROPIC_AUTH_TOKEN` | `OPENANALYST_AUTH_TOKEN` (+ `ANTHROPIC_AUTH_TOKEN` fallback) |
| `ANTHROPIC_BASE_URL` | `OPENANALYST_BASE_URL` (+ `ANTHROPIC_BASE_URL` fallback) |
| `CLAW_PERMISSION_MODE` | `OPENANALYST_PERMISSION_MODE` |
| `CLAW_CONFIG_HOME` | `OPENANALYST_CONFIG_HOME` |
| `CLAW_MODEL` | `OPENANALYST_MODEL` |
| `CLAW_SANDBOX_FILESYSTEM_MODE` | `OPENANALYST_SANDBOX_FILESYSTEM_MODE` |
| `CLAW_SANDBOX_ALLOWED_MOUNTS` | `OPENANALYST_SANDBOX_ALLOWED_MOUNTS` |
| `CLAW_CODE_REMOTE` | `OPENANALYST_CODE_REMOTE` |
| `CLAW_CODE_REMOTE_SESSION_ID` | `OPENANALYST_CODE_REMOTE_SESSION_ID` |
| `CLAW_WEB_SEARCH_BASE_URL` | `OPENANALYST_WEB_SEARCH_BASE_URL` |
| `CLAW_TODO_STORE` | `OPENANALYST_TODO_STORE` |
| `CLAW_AGENT_STORE` | `OPENANALYST_AGENT_STORE` |
| `CLAW_CODE_UPSTREAM` | `OPENANALYST_CODE_UPSTREAM` |
| `CLAW_PLUGIN_ID` | `OPENANALYST_PLUGIN_ID` |
| `CLAW_PLUGIN_NAME` | `OPENANALYST_PLUGIN_NAME` |
| `CLAW_TOOL_NAME` | `OPENANALYST_TOOL_NAME` |
| `CLAW_TOOL_INPUT` | `OPENANALYST_TOOL_INPUT` |
| `CLAW_PLUGIN_ROOT` | `OPENANALYST_PLUGIN_ROOT` |

---

## 5. Configuration Directories & Files

| Before | After |
|--------|-------|
| `.claw/` | `.openanalyst/` |
| `.claw.json` | `.openanalyst.json` |
| `.claw/settings.json` | `.openanalyst/settings.json` |
| `.claw/settings.local.json` | `.openanalyst/settings.local.json` |
| `.claw/sessions/` | `.openanalyst/sessions/` |
| `.claw-plugin/plugin.json` | `.openanalyst-plugin/plugin.json` |
| `.claw-agents/` | `.openanalyst-agents/` |
| `.claw-todos.json` | `.openanalyst-todos.json` |
| `CLAW.md` | `OPENANALYST.md` |
| `CLAW.local.md` | `OPENANALYST.local.md` |

**Files changed:**
- `rust/crates/runtime/src/config.rs` — all config path references
- `rust/crates/runtime/src/prompt.rs` — instruction file discovery paths
- `rust/crates/runtime/src/oauth.rs` — credential storage path
- `rust/crates/openanalyst-cli/src/init.rs` — project initialization paths
- `rust/crates/commands/src/lib.rs` — slash command path references
- `rust/crates/tools/src/lib.rs` — tool storage paths

---

## 6. OAuth Configuration

| Item | Before | After |
|------|--------|-------|
| Authorize URL | `https://platform.claw.dev/oauth/authorize` | `https://api.openanalyst.com/oauth/authorize` |
| Token URL | `https://platform.claw.dev/v1/oauth/token` | `https://api.openanalyst.com/v1/oauth/token` |
| Scope | `user:sessions:claw_code` | `user:sessions:openanalyst` |
| Login message | `Starting Claw OAuth login...` | `Starting OpenAnalyst OAuth login...` |
| Success message | `Claw OAuth login succeeded.` | `OpenAnalyst login succeeded.` |

**File changed:**
- `rust/crates/openanalyst-cli/src/main.rs` — `default_oauth_config()` and login/logout flow

---

## 7. Rust Source Code Identifiers

| Before | After | Locations |
|--------|-------|-----------|
| `ClawApiClient` | `OpenAnalystApiClient` | api crate, cli crate |
| `ProviderKind::ClawApi` | `ProviderKind::OpenAnalystApi` | api providers |
| `ProviderClient::ClawApi` | `ProviderClient::OpenAnalystApi` | api client |
| `MessageStream::ClawApi` | `MessageStream::OpenAnalystApi` | api client |
| `CLAW_SETTINGS_SCHEMA_NAME` | `OPENANALYST_SETTINGS_SCHEMA_NAME` | runtime config |
| `claw_default()` | `openanalyst_default()` | runtime bootstrap |
| `init_claw_md()` | `init_openanalyst_md()` | cli init |
| `render_init_claw_md()` | `render_init_openanalyst_md()` | cli init |
| `ProjectClaw` | `ProjectOpenAnalyst` | commands |
| `UserClaw` | `UserOpenAnalyst` | commands |
| `STARTER_CLAW_JSON` | `STARTER_OPENANALYST_JSON` | cli init |

---

## 8. CLI Help Text & User-Facing Strings

All `claw` command references in help text → `openanalyst`:
- `claw v{VERSION}` → `openanalyst v{VERSION}`
- `claw --help` → `openanalyst --help`
- `claw init` → `openanalyst init`
- `claw login` → `openanalyst login`
- `claw agents` → `openanalyst agents`
- etc.

Version report: `Claw Code` → `OpenAnalyst CLI`

**Files changed:**
- `rust/crates/openanalyst-cli/src/main.rs` — `print_help_to()`, `render_version_report()`, error messages
- `rust/crates/openanalyst-cli/src/args.rs` — clap command name and about text
- `rust/crates/openanalyst-cli/src/app.rs` — interactive mode message

---

## 9. Commands Crate

| Before | After |
|--------|-------|
| `"Inspect Claw config files..."` | `"Inspect OpenAnalyst config files..."` |
| `"Inspect loaded Claw instruction..."` | `"Inspect loaded OpenAnalyst instruction..."` |
| `"Create a starter CLAW.md..."` | `"Create a starter OPENANALYST.md..."` |
| `"Manage Claw Code plugins"` | `"Manage OpenAnalyst plugins"` |
| `"Project (.claw)"` | `"Project (.openanalyst)"` |
| `"User (~/.claw)"` | `"User (~/.openanalyst)"` |
| `"Direct CLI       claw agents"` | `"Direct CLI       openanalyst agents"` |
| `".claw/agents"` | `".openanalyst/agents"` |
| `".claw/skills"` | `".openanalyst/skills"` |
| `".claw/commands"` | `".openanalyst/commands"` |
| Temp files: `claw-commit-message`, `claw-pr-body` | `openanalyst-commit-message`, `openanalyst-pr-body` |

**File changed:** `rust/crates/commands/src/lib.rs`

---

## 10. Tools Crate

| Before | After |
|--------|-------|
| `"Get or set Claw Code settings."` | `"Get or set OpenAnalyst settings."` |
| User agent: `claw-rust-tools/0.1` | `openanalyst-tools/0.1` |
| Thread naming: `claw-agent-{id}` | `openanalyst-agent-{id}` |
| Agent guide: `claw-guide` | `openanalyst-guide` |
| Temp files: `claw-tools-*`, `claw-brief-*` | `openanalyst-tools-*`, `openanalyst-brief-*` |

**File changed:** `rust/crates/tools/src/lib.rs`

---

## 11. Plugins Crate

| Before | After |
|--------|-------|
| Manifest path: `.claw-plugin/plugin.json` | `.openanalyst-plugin/plugin.json` |
| Env vars: `CLAW_PLUGIN_*` | `OPENANALYST_PLUGIN_*` |
| Env vars: `CLAW_TOOL_*` | `OPENANALYST_TOOL_*` |

**Files changed:** `rust/crates/plugins/src/lib.rs`, `rust/crates/plugins/src/hooks.rs`

---

## 12. Runtime Crate

| Before | After |
|--------|-------|
| Config schema: `CLAW_SETTINGS_SCHEMA_NAME` | `OPENANALYST_SETTINGS_SCHEMA_NAME` |
| Bootstrap: `claw_default()` | `openanalyst_default()` |
| Sandbox env vars: `CLAW_SANDBOX_*` | `OPENANALYST_SANDBOX_*` |
| Remote env vars: `CLAW_CODE_REMOTE*` | `OPENANALYST_CODE_REMOTE*` |
| Config home: `CLAW_CONFIG_HOME` | `OPENANALYST_CONFIG_HOME` |
| Instruction files: `CLAW.md`, `CLAW.local.md` | `OPENANALYST.md`, `OPENANALYST.local.md` |
| Prompt heading: `# Claw instructions` | `# OpenAnalyst instructions` |
| Settings message: `No Claw Code settings files loaded.` | `No OpenAnalyst settings files loaded.` |
| Temp files: `claw-native-*` | `openanalyst-native-*` |

**Files changed:** `config.rs`, `prompt.rs`, `oauth.rs`, `lib.rs`, `remote.rs`, `sandbox.rs`, `bootstrap.rs`, `file_ops.rs`

---

## 13. Compat-Harness Crate

| Before | After |
|--------|-------|
| `CLAW_CODE_UPSTREAM` | `OPENANALYST_CODE_UPSTREAM` |
| Path references: `claw-code` | `openanalyst-code` |

**File changed:** `rust/crates/compat-harness/src/lib.rs`

---

## 14. Documentation Files

| Before | After |
|--------|-------|
| `CLAW.md` (file) | `OPENANALYST.md` (renamed) |
| `README.md` | Completely rewritten for OpenAnalyst branding |
| `rust/README.md` | Completely rewritten for OpenAnalyst branding |
| `rust/CONTRIBUTING.md` | Updated branding |
| `PARITY.md` | Updated all Claw/Claude references |
| `RESEARCH.md` | Updated all Claw/Claude references |

---

## 15. Git Configuration

| Before | After |
|--------|-------|
| `.gitignore`: `.clawd-agents/` | `.openanalyst-agents/` |
| `.gitignore`: `# Claude Code local artifacts` | `# OpenAnalyst CLI local artifacts` |
| `.gitignore`: `.claude/settings.local.json` | `.openanalyst/settings.local.json` |
| `.gitignore`: `.claude/sessions/` | `.openanalyst/sessions/` |

---

## 16. Python Source Files

| File | Change |
|------|--------|
| `src/__init__.py` | Docstring: `Claw Code` → `OpenAnalyst CLI` |
| `src/main.py` | Argparse description: `Claw Code` → `OpenAnalyst CLI` |
| `src/context.py` | Archive path: `claw_code_ts_snapshot` → `openanalyst_cli_ts_snapshot` |
| `src/parity_audit.py` | Archive path: `claw_code_ts_snapshot` → `openanalyst_cli_ts_snapshot` |
| `src/reference_data/archive_surface_snapshot.json` | Archive root path updated |

---

## 17. Test Updates

All test assertions across the codebase were updated to match new branding:
- Help text assertions: `claw init` → `openanalyst init`, etc.
- Test data: `"Claw Tests"` → `"OpenAnalyst Tests"`, `claw@example.com` → `openanalyst@example.com`
- Provider assertions: `ProviderKind::ClawApi` → `ProviderKind::OpenAnalystApi`
- Model tests: Added `openanalyst-beta` detection test
- Integration tests: Updated mock responses and env var names

---

## Build Verification

```
$ cd rust && cargo check
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 07s
```

All crates compile successfully. Zero remaining `claw`/`Claw`/`CLAW` references in Rust source or documentation.
