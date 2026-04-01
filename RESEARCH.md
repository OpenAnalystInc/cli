# OpenAnalyst CLI — Deep Research & Architecture Documentation

> **Repository:** [github.com/instructkr/openanalyst-cli](https://github.com/instructkr/openanalyst-cli)
> **Author:** Sigrid Jin (@instructkr) — Korean AI community leader, featured in Wall Street Journal
> **Stars:** 50K+ (fastest repo to reach 50K — achieved in 2 hours)
> **Date researched:** 2026-04-01
> **License:** MIT (Rust workspace)

---

## 1. What Is OpenAnalyst CLI?

OpenAnalyst CLI is a **clean-room open-source rewrite** of OpenAnalyst's agent harness. It reverse-engineers and re-implements the core architecture of OpenAnalyst CLI agent tool — the tool wiring, session management, agentic loop, permission system, hooks, plugins, MCP orchestration, and system prompt construction — without copying any proprietary source code.

The project exists in two parallel implementations:
1. **Python workspace** (`src/`) — the original porting effort, metadata-oriented, focused on cataloging and mirroring the command/tool surface
2. **Rust implementation** (`rust/`) — the primary active implementation, a fully functional CLI agent (~20K production lines / ~33K with tests) across 9 crates

**Key distinction:** This is NOT a wrapper around OpenAnalyst. It is a **from-scratch reimplementation** of OpenAnalyst's agent harness architecture, built to be faster (Rust), open-source, and extensible.

---

## 2. Origin Story & Context

On March 31, 2026, OpenAnalyst's source was accidentally exposed. The developer community scrambled to study it. Sigrid Jin — already one of the most active OpenAnalyst power users (25 billion tokens consumed, featured in WSJ) — did a clean-room port overnight:

1. Studied the exposed harness structure (tool wiring, agent loops, runtime)
2. Ported core features to Python from scratch before sunrise
3. Used **oh-my-codex (OmX)** for orchestrated AI-assisted porting (`$team` mode for parallel review, `$ralph` mode for persistent execution)
4. Later extended to a full Rust implementation for performance

The repo intentionally avoids including any of the original leaked source. It focuses on architectural patterns, not code copying.

---

## 3. Tech Stack

### Rust Implementation (Primary — `rust/`)
| Technology | Usage |
|-----------|-------|
| **Rust 2021** | Core language, memory-safe, high-performance |
| **reqwest** | HTTP client for API calls |
| **serde / serde_json** | JSON serialization throughout |
| **crossterm** | Terminal rendering (colors, cursor, clearing) |
| **pulldown-cmark** | Markdown parsing for terminal rendering |
| **syntect** | Syntax highlighting in code blocks |
| **rustyline** | Interactive REPL line editing |
| **lsp-types** | LSP protocol type definitions |
| **tokio** | Async runtime (for LSP, server crate) |
| **axum** | HTTP/SSE server (server crate) |
| **sha2** | PKCE challenge hashing (OAuth) |
| **glob** | File pattern matching |
| **regex** | Grep search patterns |
| **walkdir** | Recursive directory traversal |
| **async-stream** | SSE event streaming (server crate) |

### Python Implementation (Metadata/Porting — `src/`)
| Technology | Usage |
|-----------|-------|
| **Python 3.10+** | Porting workspace |
| **dataclasses** | Data models |
| **argparse** | CLI |
| **unittest** | Verification |

---

## 4. Architecture — Rust Workspace

The Rust workspace (`rust/Cargo.toml`) contains **9 crates** organized as a modular agent system:

```
rust/crates/
├── api/              # API client + SSE streaming + provider abstraction
├── openanalyst-cli/         # Main CLI binary (REPL, one-shot, rendering)
├── commands/         # Slash command registry + execution
├── compat-harness/   # TS manifest extraction (reads upstream source)
├── lsp/              # LSP client integration (diagnostics, go-to-def, references)
├── plugins/          # Plugin system (manifests, hooks, lifecycle, tools)
├── runtime/          # Core engine (conversation loop, config, sessions, MCP, permissions)
├── server/           # HTTP/SSE server (axum-based, session management)
└── tools/            # Built-in tool registry + execution
```

### 4.1 Core Agentic Loop (`runtime/src/conversation.rs`)

The heart of the system. This is a generic `ConversationRuntime<C, T>` that is parameterized over:
- `C: ApiClient` — sends requests to the LLM and receives streaming events
- `T: ToolExecutor` — executes tool calls locally

**The loop works like OpenAnalyst's:**

```
User input → API call → Assistant response
  ↓
If response contains ToolUse blocks:
  For each tool:
    1. Check permissions (PermissionPolicy.authorize())
    2. Run PreToolUse hooks
    3. If hook denies → return error to LLM
    4. Execute tool via ToolExecutor
    5. Run PostToolUse hooks
    6. Merge hook feedback into output
    7. Push ToolResult back to session
  Loop back to API call
  ↓
If no tool calls → conversation turn complete
```

Key design decisions matching OpenAnalyst:
- **Unlimited iterations** by default (`usize::MAX`)
- **Hook pipeline** with exit-code-based deny/allow (exit 0 = allow, exit 2 = deny)
- **Permission escalation prompting** (workspace-write → danger-full-access requires user approval)
- **Usage tracking** reconstructed from session history on resume

### 4.2 API Client (`api/`)

Multi-provider client supporting:
- **OpenAnalyst API** — native SSE streaming, API key + OAuth bearer auth
- **xAI (Grok)** — OpenAI-compatible endpoint
- **OpenAI** — OpenAI-compatible endpoint

Provider auto-detection based on model name. Model aliases:
- `opus` → `claude-opus-4-6`
- `sonnet` → `claude-sonnet-4-6`
- `haiku` → `claude-haiku-4-5-20251213`
- `grok` → `grok-3`
- `grok-mini` → `grok-3-mini`

**Auth sources** (can coexist):
- `ANTHROPIC_API_KEY` → sent as `x-api-key` header
- `ANTHROPIC_AUTH_TOKEN` → sent as `Authorization: Bearer` header
- Both present → `ApiKeyAndBearer` mode (both headers sent)
- Neither present → error with helpful message listing expected env vars
- OAuth credentials → persisted to disk, auto-refreshed when expired

### 4.3 Tool System (`tools/`)

~45K tokens of tool implementation. The `GlobalToolRegistry` manages:

**20 Built-in tools** (via `mvp_tool_specs()`), with mixed naming convention:

| Tool | Permission | Description |
|------|-----------|-------------|
| `bash` | DangerFullAccess | Shell execution with timeout, background, sandbox disable |
| `PowerShell` | DangerFullAccess | Windows PowerShell execution |
| `read_file` | ReadOnly | Read file with offset/limit |
| `write_file` | WorkspaceWrite | Write file content |
| `edit_file` | WorkspaceWrite | Replace text in file (old_string → new_string, replace_all) |
| `glob_search` | ReadOnly | Find files by glob pattern |
| `grep_search` | ReadOnly | Regex content search with context lines, output modes |
| `WebFetch` | ReadOnly | Fetch URL, convert to text, answer prompt about it |
| `WebSearch` | ReadOnly | Web search with domain allow/block lists |
| `TodoWrite` | WorkspaceWrite | Structured task list (pending/in_progress/completed) |
| `Skill` | ReadOnly | Load local SKILL.md files |
| `Agent` | DangerFullAccess | Sub-agent orchestration with model/type selection |
| `ToolSearch` | ReadOnly | Search for deferred/specialized tools |
| `NotebookEdit` | WorkspaceWrite | Jupyter cell replace/insert/delete |
| `Sleep` | ReadOnly | Wait for duration without holding a process |
| `SendUserMessage` | ReadOnly | Send message to user (aliased as `Brief`) |
| `Config` | WorkspaceWrite | Get/set OpenAnalyst CLI settings |
| `StructuredOutput` | ReadOnly | Return structured output in requested format |
| `REPL` | DangerFullAccess | Execute code in language REPL subprocess |

**Note on naming:** Snake_case tools (`bash`, `read_file`, `write_file`, `edit_file`, `glob_search`, `grep_search`) are the core file/shell tools. PascalCase tools (`WebFetch`, `WebSearch`, `Agent`, `Skill`, etc.) are higher-level orchestration tools. This mirrors the exact naming from OpenAnalyst's tool definitions.

**Plugin tools** are loaded dynamically via `GlobalToolRegistry::with_plugin_tools()` and validated against builtin names (no collisions or duplicates allowed).

**Tool name normalization** with aliases for `--allowedTools` flag:
- `read` → `read_file`, `write` → `write_file`, `edit` → `edit_file`
- `glob` → `glob_search`, `grep` → `grep_search`

### 4.4 Permission System (`runtime/src/permissions.rs`)

Five permission modes (matching OpenAnalyst):
1. **ReadOnly** — only read tools allowed
2. **WorkspaceWrite** — read + write to project files
3. **DangerFullAccess** — everything (default in this project)
4. **Prompt** — ask user for every tool call
5. **Allow** — bypass all permission checks entirely

Modes are ordered (`ReadOnly < WorkspaceWrite < DangerFullAccess`). Each tool declares its `required_permission` in the `ToolSpec`. The policy compares `active_mode >= required_mode`:
- If satisfied → allow
- If `active_mode == Prompt` or escalation needed (e.g., WorkspaceWrite → DangerFullAccess) → delegates to `PermissionPrompter` trait for interactive approval
- If `active_mode == Allow` → always allow (skips all checks)
- Otherwise → hard deny with explanation message

Read-only cannot escalate to write without prompting. The system generates descriptive denial reasons like `"tool 'bash' requires danger-full-access permission; current mode is read-only"`.

### 4.5 Hook System (`runtime/src/hooks.rs`)

Shell-based hooks matching OpenAnalyst's:
- **PreToolUse** — runs before tool execution, can deny (exit code 2)
- **PostToolUse** — runs after, can flag errors

Hooks receive environment variables:
- `HOOK_EVENT`, `HOOK_TOOL_NAME`, `HOOK_TOOL_INPUT`, `HOOK_TOOL_IS_ERROR`, `HOOK_TOOL_OUTPUT`
- JSON payload via stdin

Exit code semantics: 0 = allow, 2 = deny, anything else = warn + allow

### 4.6 Config System (`runtime/src/config.rs`)

Three-tier config hierarchy (matching OpenAnalyst):
1. **User** — `~/.openanalyst/settings.json` (or legacy `~/.openanalyst.json`)
2. **Project** — `.openanalyst/settings.json` in project root
3. **Local** — `.openanalyst/settings.local.json` (gitignored)

Config merges hooks, MCP servers, plugins, OAuth, model, permission mode, and sandbox settings.

**MCP Config supports 6 transport types:**
- Stdio, SSE, HTTP, WebSocket, SDK, ManagedProxy

### 4.7 Session Management (`runtime/src/session.rs`)

Sessions are JSON-serializable with:
- Version field for forward compatibility
- Messages array with role-based typing (System, User, Assistant, Tool)
- Content blocks: Text, ToolUse (id/name/input), ToolResult (output/is_error)
- Per-message usage tracking

Sessions can be saved/loaded from disk for resume functionality.

### 4.8 Compaction (`runtime/src/compact.rs`)

When sessions grow too large:
- Configurable `preserve_recent_messages` (default 4)
- `max_estimated_tokens` threshold (default 10,000)
- Generates a summary, strips `<analysis>` tags, preserves `<summary>` content
- Inserts continuation preamble for seamless resume

### 4.9 System Prompt (`runtime/src/prompt.rs`)

`SystemPromptBuilder` constructs the system prompt with sections:
- Intro, system behavior, task instructions, action guidelines
- Environment context (model family, CWD, date, platform)
- Project context (git status, git diff)
- OPENANALYST.md instruction files (max 4KB per file, 12KB total)
- Runtime config section
- LSP context enrichment (diagnostics, symbols)

Instruction file discovery walks up from CWD looking for `OPENANALYST.md` files (project + global).

### 4.10 Plugin System (`plugins/`)

Full plugin architecture:
- **Manifest format:** `.openanalyst-plugin/plugin.json` with name, version, description, permissions, hooks, lifecycle, tools, commands
- **Plugin kinds:** Builtin, Bundled (shipped with CLI), External (user-installed)
- **Plugin permissions:** Read, Write, Execute
- **Plugin lifecycle:** Init and Shutdown commands
- **Plugin tools:** custom tools with JSON schemas, validated against builtins
- **Plugin hooks:** PreToolUse/PostToolUse merged with global hooks
- **Plugin manager:** install, uninstall, enable/disable, list

### 4.11 LSP Integration (`lsp/`)

Full Language Server Protocol client:
- Connects to LSP servers via stdio
- Workspace diagnostics collection
- Go-to-definition
- Find references
- Document open/change/save notifications
- Context enrichment for system prompt (injects diagnostics + symbol info)

### 4.12 CLI (`openanalyst-cli/`)

The main binary (`openanalyst`). ~47K tokens across 5 source files:
- `main.rs` — CLI entry, argument parsing (manual, NOT clap), REPL loop, API integration, OAuth login/logout
- `app.rs` — Session config/state, slash command parsing
- `args.rs` — CLI argument definitions, permission mode parsing
- `input.rs` — Custom line editor built on crossterm (NOT rustyline for main editing — rustyline is used for simpler prompts). Implements full vim keybinding support with Normal/Insert/Visual/Command modes, yank buffer, visual selection, and command-line `:` commands
- `render.rs` — Terminal markdown renderer with pulldown-cmark + syntect syntax highlighting, color themes, spinners (⠋⠙⠹... braille animation), table/code-block/heading rendering, ANSI colors
- `init.rs` — Project initialization (`openanalyst init` creates starter OPENANALYST.md)

Features:
- **Interactive REPL** with custom crossterm-based editor
- **One-shot prompt mode** with text or JSON output format
- **Streaming display** with progressive markdown rendering and tool call spinners
- **OAuth login/logout** flow with PKCE (loopback redirect on port 4545)
- **Session resume** from saved JSON files
- **Vim keybinding mode** with full operator-pending, visual selection, yank/paste
- **Git slash commands** (/branch, /commit, /commit-push-pr, /worktree)
- **Plugin loading** on startup via `PluginManager`
- **Heartbeat interval** (3s) during long operations

### 4.13 Commands (`commands/`)

Slash command registry with specs:
- `/help`, `/status`, `/compact`, `/model`, `/permissions`, `/clear`, `/cost`
- `/resume`, `/config`, `/memory`, `/init`, `/diff`, `/version`
- `/bughunter` — codebase bug inspection
- `/agents` — agent management
- `/skills` — skill discovery
- `/plugins` — plugin management
- `/export`, `/session`

### 4.14 Compat Harness (`compat-harness/`)

Reads the **original OpenAnalyst TypeScript source** (when available locally) to extract:
- Command registry (from `src/commands.ts`)
- Tool registry (from `src/tools.ts`)
- Bootstrap plan (from `src/entrypoints/cli.tsx`)

**Upstream source discovery** — searches multiple candidate paths in order:
1. Parent of the Rust workspace directory
2. `OPENANALYST_CLI_UPSTREAM` environment variable
3. Ancestor directories (up to 4 levels) + `/openanalyst-cli`
4. `reference-source/openanalyst-cli` relative to workspace
5. `vendor/openanalyst-cli` relative to workspace

Validates by checking if `src/commands.ts` exists at the candidate root. Returns `ExtractedManifest` containing `CommandRegistry`, `ToolRegistry`, and `BootstrapPlan`. This is strictly for parity auditing — not used at runtime.

### 4.15 HTTP Server (`server/`)

Axum-based HTTP/SSE server for remote/headless use. Single-file crate (`lib.rs`):

**State management:**
- `AppState` with `SessionStore` (HashMap behind `Arc<RwLock<>>`)
- Atomic session ID allocation (`session-1`, `session-2`, ...)
- Per-session `broadcast::channel` for SSE event fan-out (capacity: 64)

**Session events** (JSON-tagged enum):
- `Snapshot` — full session state
- Event streaming via SSE with `KeepAlive`

**Routes:** REST endpoints for session CRUD + SSE streaming. Enables integration with editors, web UIs, and remote agent orchestration.

### 4.16 Bootstrap Plan (`runtime/src/bootstrap.rs`)

Mirrors OpenAnalyst's 12-phase startup sequence:
1. `CliEntry` → `FastPathVersion` → `StartupProfiler`
2. `SystemPromptFastPath` → `ChromeMcpFastPath` → `DaemonWorkerFastPath`
3. `BridgeFastPath` → `DaemonFastPath` → `BackgroundSessionFastPath`
4. `TemplateFastPath` → `EnvironmentRunnerFastPath` → `MainRuntime`

This reveals OpenAnalyst's internal startup optimization — fast-path checks before expensive initialization, with separate paths for Chrome MCP, daemon workers, bridges, and background sessions.

### 4.17 Sandbox System (`runtime/src/sandbox.rs`)

Filesystem isolation for tool execution:

| Setting | Options |
|---------|---------|
| `filesystem_mode` | `Off`, `WorkspaceOnly` (default), `AllowList` |
| `namespace_restrictions` | Boolean — OS namespace isolation |
| `network_isolation` | Boolean — network access control |
| `allowed_mounts` | List of permitted mount paths |

`SandboxStatus` tracks whether sandboxing is supported, active, and which features (namespace, network) are actually running. Container detection via `ContainerEnvironment` with marker-based detection.

### 4.18 Remote/Upstream Proxy (`runtime/src/remote.rs`)

Infrastructure for running behind corporate proxies or in remote environments:

- **CCR (OpenAnalyst Remote)** session management — `RemoteSessionContext` with session ID and base URL
- **Upstream proxy bootstrap** — token path (`/run/ccr/session_token`), CA bundle discovery, system CA path
- **Proxy environment** — reads `HTTPS_PROXY`, `NO_PROXY`, `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS`, `REQUESTS_CA_BUNDLE`, `CURL_CA_BUNDLE`
- **No-proxy list** — 16 hosts including localhost, private ranges, anthropic.com, github.com, npm/crates registries
- **WebSocket URL resolution** for upstream proxy connections

### 4.19 Usage & Cost Tracking (`runtime/src/usage.rs`)

Per-model pricing with 4-dimensional cost tracking:

| Model Tier | Input/M | Output/M | Cache Create/M | Cache Read/M |
|-----------|---------|----------|----------------|--------------|
| **Opus** | $15.00 | $75.00 | $18.75 | $1.50 |
| **Sonnet** (default) | $15.00 | $75.00 | $18.75 | $1.50 |
| **Haiku** | $1.00 | $5.00 | $1.25 | $0.10 |

`TokenUsage` tracks: `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`. `UsageTracker` accumulates across turns and reconstructs from restored sessions.

### 4.20 MCP Client (`runtime/src/mcp_client.rs`, `mcp_stdio.rs`)

Full Model Context Protocol implementation:

**Transport types:**
- `McpStdioTransport` — spawns child process, communicates via JSON-RPC over stdin/stdout
- `McpRemoteTransport` — SSE/HTTP endpoints
- `McpSdkTransport` — SDK-level integration
- `McpManagedProxyTransport` — proxied connections

**JSON-RPC layer** (`mcp_stdio.rs`):
- `McpStdioProcess` — manages child process lifecycle
- `McpServerManager` — orchestrates multiple MCP servers
- Full request/response/error types (`JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`)
- Tool discovery: `McpListToolsParams` → `McpListToolsResult` → `McpTool` with input schemas
- Resource discovery: `McpListResourcesParams` → `McpResource` → `McpResourceContents`
- Tool execution: `McpToolCallParams` → `McpToolCallResult` → `McpToolCallContent`
- Initialize handshake with client info and capability negotiation

**MCP naming** (`mcp.rs`):
- Tool names: `mcp__{server}__{tool}` (double-underscore separated)
- Server name normalization (alphanumeric + underscore/hyphen only)
- CCR proxy URL unwrapping (extracts `mcp_url` query parameter)
- Config hashing for change detection

---

## 5. Python Workspace Architecture (`src/`)

The Python side is more of a **porting audit tool** than a runtime. It has evolved into a substantial workspace (~60+ files) with:

### Core Modules
| Module | Purpose |
|--------|---------|
| `main.py` | CLI entrypoint with 20+ subcommands |
| `models.py` | Dataclasses: Subsystem, PortingModule, PortingBacklog, UsageSummary |
| `commands.py` | Command surface metadata (loaded from `reference_data/commands_snapshot.json`) |
| `tools.py` | Tool surface metadata (loaded from `reference_data/tools_snapshot.json`) |
| `query_engine.py` | Session simulation with turn limits, budget, compaction, streaming |
| `port_manifest.py` | Workspace introspection (counts files, lists modules) |
| `runtime.py` | Prompt routing across command/tool inventories |
| `parity_audit.py` | Compares Python workspace against archived TypeScript source |
| `permissions.py` | Tool permission context with deny lists |
| `session_store.py` | Session save/load |
| `transcript.py` | Transcript store with compaction |
| `setup.py` | Startup/prefetch setup reporting |
| `bootstrap_graph.py` | Bootstrap/runtime phase graph |
| `command_graph.py` | Command graph segmentation |
| `tool_pool.py` | Tool pool assembly |
| `remote_runtime.py` | Remote/SSH/teleport runtime branching simulation |
| `direct_modes.py` | Direct-connect and deep-link modes |

### Extended Modules (66 Python files across 27 subdirectories)
The `src/` directory contains subdirectories that mirror OpenAnalyst's actual internal module structure. This is significant — it reveals the full internal organization of OpenAnalyst:

| Subdirectory | Inferred OpenAnalyst Subsystem |
|-------------|-------------------------------|
| `assistant/` | Session history, agent orchestration |
| `bootstrap/` | Startup/initialization phases |
| `bridge/` | Transport bridge (editor ↔ agent) |
| `buddy/` | Pair programming / buddy mode |
| `cli/` | CLI handlers, structured/remote IO, transports |
| `components/` | UI components (terminal rendering) |
| `constants/` | Shared constants |
| `coordinator/` | Task/agent coordination |
| `entrypoints/` | CLI entry (cli.tsx), web, etc. |
| `hooks/` | PreToolUse/PostToolUse hook infrastructure |
| `keybindings/` | Keyboard shortcut configuration |
| `memdir/` | Memory directory management |
| `migrations/` | Config/data migration scripts |
| `moreright/` | Unknown — possibly a feature module |
| `native_ts/` | Native TypeScript interop |
| `outputStyles/` | Output style configurations |
| `plugins/` | Plugin system (builtins, bundled, lifecycle) |
| `reference_data/` | JSON snapshots of command/tool inventories |
| `remote/` | Remote session transport |
| `schemas/` | JSON schemas for validation |
| `screens/` | Terminal screen management |
| `server/` | HTTP server mode |
| `services/` | API, OAuth, MCP, analytics, policy, sync |
| `skills/` | Skill loading, registry, bundled skills |
| `state/` | Application state management |
| `types/` | Shared TypeScript-origin type definitions |
| `upstreamproxy/` | Upstream proxy configuration |
| `utils/` | Utility functions |
| `vim/` | Vim keybinding mode |
| `voice/` | Voice input/output features |

This directory structure is a **map of OpenAnalyst's internal architecture** and represents one of the most complete public records of how the system is organized internally.

---

## 6. Feature Parity Analysis (vs OpenAnalyst)

Based on the PARITY.md document:

| Feature | Status | Notes |
|---------|--------|-------|
| API client + streaming | ✅ Complete | Multi-provider (Anthropic, xAI, OpenAI) |
| OAuth login/logout | ✅ Complete | PKCE flow, credential persistence |
| Agentic tool loop | ✅ Complete | Generic runtime with hook integration |
| Tool system (bash/file/search/web) | ✅ Complete | 20 built-in tools |
| Sub-agent orchestration | ✅ Complete | Agent tool for spawning sub-tasks |
| OPENANALYST.md discovery | ✅ Complete | Project + global instruction files |
| Config hierarchy | ✅ Complete | User/Project/Local with merge |
| Permission system | ✅ Complete | 5 modes (ReadOnly/WorkspaceWrite/DangerFullAccess/Prompt/Allow) |
| MCP lifecycle (stdio) | ✅ Complete | Full MCP client with tool discovery |
| Session persistence + resume | ✅ Complete | JSON serialization |
| Compaction | ✅ Complete | Summary + recent message preservation |
| Extended thinking | ✅ Complete | Thinking blocks support |
| Cost tracking | ✅ Complete | Per-model pricing |
| Markdown terminal rendering | ✅ Complete | Syntax highlighting, ANSI colors |
| Slash commands | ✅ Complete | 18+ commands including git integration |
| LSP integration | ✅ Complete | Diagnostics, definitions, references |
| Vim keybinding mode | ✅ Complete | Normal/insert/visual/command |
| HTTP/SSE server | ✅ Complete | Axum-based remote access |
| Sandbox/filesystem isolation | ✅ Complete | WorkspaceOnly/AllowList/Off modes, namespace + network isolation |
| Remote/upstream proxy | ✅ Complete | CCR sessions, CA bundles, proxy env vars |
| Bootstrap phases | ✅ Complete | 12-phase startup mirroring OpenAnalyst |
| Hooks (PreToolUse/PostToolUse) | 🔧 Partial | Config parsed, shell execution works, no TS-style full pipeline |
| Plugin system | 🔧 Partial | Manifest/manager exists, no marketplace |
| Skills registry | 📋 Planned | Local SKILL.md only, no bundled registry |
| Structured/remote IO | 📋 Missing | No TS-style transport layers |
| Analytics/telemetry | 📋 Missing | No TS-equivalent |
| Team memory sync | 📋 Missing | No multi-user features |

---

## 7. What Makes This Unique / Interesting

### 7.1 Architecture Insights
This project is essentially a **public specification of OpenAnalyst's internal architecture**. By studying it, you learn:
- How OpenAnalyst's agentic loop works (API call → tool use → permission check → hook pipeline → tool result → loop)
- The system prompt structure and section ordering
- The config hierarchy and merge strategy
- The permission model with escalation
- The hook system with exit-code-based control flow
- The compaction strategy for long conversations
- The MCP integration patterns

### 7.2 Multi-Provider Support
Unlike OpenAnalyst (Anthropic-only), OpenAnalyst CLI supports multiple providers through a `Provider` trait abstraction. This includes xAI's Grok models and OpenAI, with automatic provider detection from model names.

### 7.3 Clean-Room Approach
The project carefully avoids including any leaked source. The `compat-harness` crate can read the original TS source *if* the user has it locally, but only for parity auditing — not for runtime use.

### 7.4 Rust Performance
~20K lines of production Rust (~33K including tests) with `unsafe_code = "forbid"` and strict clippy lints (`pedantic` level). The binary is native and fast compared to OpenAnalyst's Node.js/TypeScript runtime.

### 7.5 Plugin Architecture
The plugin system is more complete than many expect — full manifest format, tool registration with schema validation, hook integration, lifecycle management, and permission scoping.

### 7.6 AI-Assisted Development
The entire project was built using AI orchestration tools (OmX, OmO), making it a meta-example of AI building AI tools.

---

## 8. Branch Strategy

The repo has extensive branching:

| Branch Pattern | Purpose |
|----------------|---------|
| `main` | Stable, Python + Rust |
| `dev/rust` | Active Rust development |
| `rcc/*` | Feature branches (api, cache, cli, cost, git, hooks, plugins, render, runtime, sandbox, tools, etc.) |
| `feat/*` | Feature branches (release, UI, redesign) |
| `integration/*` | External collaboration |

---

## 9. Crate Dependency Graph

```
                    openanalyst-cli (binary)
                   /    |    \     \
                  /     |     \     \
               tools  commands  compat-harness  plugins
              / |  \     |          |
             /  |   \    |          |
           api  |  plugins         runtime
            |   |                 /  |   \
            |  runtime           /   |    \
            |  /   \           lsp  plugins  (sha2, glob, regex, walkdir)
            | /     \
          runtime   plugins
           |
          (lsp, plugins, serde, tokio, sha2, glob, regex, walkdir)

  server (standalone)
    |
  runtime
```

Simplified: `runtime` is the foundational crate used by everything. `api` depends on `runtime` for OAuth types. `tools` depends on `api`, `runtime`, and `plugins`. `openanalyst-cli` depends on all other crates.

---

## 10. Key Design Patterns

### Trait-Based Abstractions
- `ApiClient` trait — swappable API backends
- `ToolExecutor` trait — swappable tool implementations
- `PermissionPrompter` trait — swappable permission UIs
- `Provider` trait — multi-provider API support

### Builder Pattern
- `SystemPromptBuilder` — fluent API for prompt construction
- `PermissionPolicy` — chainable `with_tool_requirement()`
- `ConversationRuntime` — `with_max_iterations()`

### Registry Pattern
- `GlobalToolRegistry` — builtin + plugin tools, name normalization, allowed-tool filtering
- `CommandRegistry` — slash commands with specs
- `PluginManager` — plugin discovery, install, enable/disable

### Event-Driven Streaming
- `AssistantEvent` enum: TextDelta, ToolUse, Usage, MessageStop
- `StreamEvent` for API-level SSE events
- Progressive rendering with spinner and markdown state machine

---

## 11. Relevance to Upfyn Code

Several architectural patterns from OpenAnalyst CLI directly parallel or could inform Upfyn Code's own agent layer:

| OpenAnalyst CLI | Upfyn Code Equivalent | Insight |
|-----------|----------------------|---------|
| `ConversationRuntime<C,T>` loop | `agent_loop.rs` native agent | Trait-based generics allow testing with mock clients |
| `PermissionPolicy` (5 modes) | Canvas/CLI permission modes | `Allow` mode = Upfyn's "dontAsk", escalation prompting = per-tool approval |
| Hook pipeline (Pre/PostToolUse) | Settings hooks system | Exit-code-based deny (2) is simpler than callback-based |
| Config hierarchy (User/Project/Local) | Settings scopes | Three-tier merge with legacy fallback paths |
| Session JSON save/load | Session management per tab | Version field for forward compatibility |
| Compaction with recent-message preservation | Context window management | 4 recent messages + summary — same pattern as OpenAnalyst |
| MCP stdio/SSE/HTTP client | MCP Global Connector | `McpServerManager` manages multiple servers, tool name = `mcp__{server}__{tool}` |
| `SystemPromptBuilder` | Bridge prompt construction | Sections: intro → system → tasks → actions → DYNAMIC_BOUNDARY → environment → project → config → LSP |
| Plugin manifest (`.openanalyst-plugin/`) | MCP + Composio integrations | Plugins can provide tools, hooks, and commands — richer than MCP alone |
| `GlobalToolRegistry` | Native agent tool routing | Builtin + plugin tools with collision detection and `--allowedTools` filtering |
| Multi-provider `ProviderClient` | BYOK provider system | Auto-detection from model name, Provider trait abstraction |
| `SandboxConfig` | Sandbox-first mode | FilesystemIsolation modes match Upfyn's worktree approach |
| Remote/CCR proxy | Desktop↔Web relay | Session tokens, CA bundles, WebSocket URLs |
| Bootstrap 12-phase plan | Bridge startup | Fast-path optimizations before heavy initialization |
| Vim keybinding mode | N/A (potential feature) | Full normal/insert/visual/command with yank buffer |
| `server` crate (axum SSE) | Upfyn-Code web UI relay | Session management + event streaming for remote clients |

---

## 12. Statistics

| Metric | Value |
|--------|-------|
| Rust source files | 48 |
| Rust lines of code | ~33,000 (including tests) / ~20,000 (per README, excluding tests) |
| Rust crates | 9 (api, openanalyst-cli, commands, compat-harness, lsp, plugins, runtime, server, tools) |
| Built-in tools | 20 |
| Slash commands | 18+ |
| Bootstrap phases | 12 |
| MCP transport types | 6 (Stdio, SSE, HTTP, WebSocket, SDK, ManagedProxy) |
| Python source directories | 30+ (mirroring OpenAnalyst's module structure) |
| GitHub stars | 50,000+ |
| Time to 50K stars | 2 hours |
| Binary name | `openanalyst` |
| Default model | `claude-opus-4-6` |
| Default permissions | `danger-full-access` |
| Default OAuth port | 4545 |
| CLI argument parsing | Manual (no clap) |
| Line editor | Custom crossterm-based (vim modes) + rustyline fallback |

---

*Document generated by deep codebase exploration on 2026-04-01.*
