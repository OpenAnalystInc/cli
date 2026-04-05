# OpenAnalyst CLI

<p align="center">
  <strong>The Universal AI Agent for Your Terminal</strong><br>
  <em>One CLI. Every LLM Provider. Persistent Conversations Across Models.</em>
</p>

---

## What Is OpenAnalyst CLI?

OpenAnalyst CLI is an **independent, open-source AI coding agent** that connects to every major LLM provider through a single, unified terminal interface. It is built from the ground up in Rust with its own multi-provider architecture, Ratatui-based TUI, multi-agent orchestrator, and a full tool execution framework.

**OpenAnalyst is not a fork or copy of any other product.** It is an original work by OpenAnalyst Inc that implements industry-standard agent patterns — agentic loops, tool calling, permission systems, and session management — which are common across every major AI CLI tool (Codex CLI, Gemini CLI, Aider, Goose, aichat, and many others). These are established software engineering patterns, not proprietary to any single company.

---

## Why OpenAnalyst?

| Capability | OpenAnalyst CLI | Other CLI Tools |
|-----------|----------------|-----------------|
| **Providers** | 7 (OpenAnalyst, Anthropic, OpenAI, xAI, Gemini, OpenRouter, Bedrock) | Typically 1-2 |
| **OAuth login** | Browser login for Claude, Codex, Gemini — no API key needed | API key only |
| **Mid-conversation model switching** | Session persists across providers | Not supported |
| **TUI** | Full Ratatui-based terminal UI with blue-branded OA banner | Most use basic REPL |
| **Multi-agent orchestrator** | Built-in parallel agents, swarms, autonomous loops, MOE | Limited or none |
| **Multimedia** | /image, /voice, /speak, /vision, /diagram | Rarely supported |
| **51+ slash commands** | Git, AI planning, multimedia, web scraping, Playwright | 5-15 typical |
| **19 built-in tools** | Bash, file ops, search, web, agents, REPL, PowerShell | 5-10 typical |
| **MCP support** | Full Model Context Protocol — unlimited external tools | Partial or none |
| **Single binary** | Native Rust, no runtime dependencies (18 MB) | Often needs Node/Python |

---

## Quick Start

### 1. Install

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/openanalyst-cli/main/install.sh | bash
```

**Windows PowerShell:**
```powershell
irm https://raw.githubusercontent.com/OpenAnalystInc/openanalyst-cli/main/install.ps1 | iex
```

**Build from source:**
```bash
cd rust && cargo build --release
```

### 2. Login

```bash
openanalyst login
```

Interactive provider picker — select your LLM provider, authenticate via browser OAuth or API key, credentials saved automatically.

**Direct provider login:** For **Claude** (Anthropic), **Codex** (OpenAI), and **Gemini** (Google) you can login directly with your provider account via browser OAuth — no API key needed. Credentials are stored securely with PKCE and auto-refresh.

Alternatively, edit `~/.openanalyst/.env` (created during install) and add your API keys directly.

```bash
openanalyst whoami              # See all logged-in providers
```

### 3. Start

```bash
openanalyst                                    # Launch TUI (default)
openanalyst --no-tui                          # Legacy REPL mode
openanalyst "explain this codebase"           # One-shot prompt
openanalyst --model gpt-4o "summarize this"   # Use specific model
openanalyst --model gemini-2.5-pro "review"   # Google Gemini
openanalyst --model grok "fix the bug"        # xAI Grok
```

---

## Supported Providers

All providers are first-class citizens with live model discovery, streaming, and full tool support.

| Provider | Auth | Models |
|----------|------|--------|
| **OpenAnalyst** (default) | `OPENANALYST_AUTH_TOKEN` | Fetched live from API |
| **Anthropic / Claude** | `openanalyst login` (OAuth) or `ANTHROPIC_API_KEY` | opus, sonnet, haiku |
| **OpenAI / Codex** | `openanalyst login` (OAuth) or `OPENAI_API_KEY` | gpt-4o, o3, codex-mini |
| **Google Gemini** | `openanalyst login` (OAuth) or `GEMINI_API_KEY` | gemini-2.5-pro, flash |
| **xAI / Grok** | `XAI_API_KEY` | grok-3, grok-mini |
| **OpenRouter** | `OPENROUTER_API_KEY` | 350+ models from any provider |
| **Amazon Bedrock** | `BEDROCK_API_KEY` | Fetched live from gateway |

**All credits and trademarks belong to their respective providers.** Claude is a trademark of Anthropic. GPT is a trademark of OpenAI. Gemini is a trademark of Google. Grok is a trademark of xAI. OpenAnalyst CLI is an independent tool that connects to these providers' public APIs.

---

## Features

### Full Terminal UI (Ratatui-based)
- **Blue-branded OA banner** with version inline, rounded-corner box, account info, tips
- Scrollable chat with inline tool call cards and rich diff rendering
- **Type immediately** — no vim Normal mode gate, just start typing
- Up/Down arrow keys for prompt history navigation
- Status line with animated spinner, elapsed time, token count, and current model
- Blue thick borders on input box, sidebar, and autocomplete popup
- Permission dialogs as modal overlays
- Mouse scroll and keyboard navigation
- Vim mode available via `/vim` toggle
- Shift+Enter for multi-line input
- Double Ctrl+C to quit (first press cancels, second quits)

### Authentication
- **Browser OAuth** for Claude, Codex, and Gemini — sign in with your provider account
- **API key** support for all 7 providers
- **PKCE security** with automatic token refresh
- Credentials stored in `~/.openanalyst/credentials.json`
- Interactive provider picker with arrow-key navigation

### Multi-Agent Orchestrator
- Spawn sub-agents for parallel tasks (Explore, Plan, General)
- **Agent Swarm** — `/swarm <task>` decomposes work across parallel agents
- **Autonomous loops** — `/openanalyst <task>` runs think→act→observe→verify cycles
- **Mixture of Experts (MOE)** — routes to specialized models per task type
- **Smart model routing** — `/route` to view/edit per-category model assignments
- **Effort budgets** — `/effort` to control thinking depth (low/medium/high/max)
- Each agent has its own conversation runtime and tool permissions
- Agent lifecycle events displayed in real-time in TUI
- Channel-based async bridge (sync runtime ↔ async TUI)

### 51+ Slash Commands

**Session & Config:** /help, /status, /cost, /model, /clear, /compact, /session, /export, /resume, /version, /login, /logout, /context, /vim, /config, /memory, /init, /exit, /sidebar, /permissions

**Code & Git:** /diff, /commit, /commit-push-pr, /pr, /issue, /branch, /worktree, /teleport, /diff-review, /changelog

**Analysis & Planning:** /bughunter, /ultraplan, /debug-tool-call, /think, /doctor

**Multimedia:** /image, /voice, /speak, /vision, /diagram

**Web & Data:** /scrape, /json

**AI & Translation:** /translate, /tokens

**Agent Control:** /swarm, /openanalyst, /ask, /user-prompt, /effort, /route, /agents, /skills

**Dev Tools:** /dev (install, open, screenshot, snap, click, type, test, codegen, stop)

**Advanced:** /mcp, /knowledge, /explore, /plugins, /hooks, /add-dir

### 19 Built-in Tools

| Tool | Permission | Description |
|------|-----------|-------------|
| `bash` | Full Access | Execute shell commands with sandboxing |
| `read_file` | Read Only | Read file contents with line numbers |
| `write_file` | Workspace | Create or overwrite files |
| `edit_file` | Workspace | Modify files with exact string replacement |
| `glob_search` | Read Only | Find files by glob patterns |
| `grep_search` | Read Only | Search file contents with regex |
| `web_search` | Read Only | Search the internet |
| `web_fetch` | Read Only | Fetch and parse URL content |
| `agent` | Varies | Spawn sub-agents for parallel tasks |
| `todo_write` | Workspace | Create and manage task lists |
| `notebook_edit` | Workspace | Edit Jupyter notebook cells |
| `skill` | Varies | Invoke custom skills |
| `tool_search` | Read Only | Discover available tools |
| `config` | Read Only | Read configuration and settings |
| `repl` | Varies | Run code in Python or Node REPL |
| `structured_output` | Read Only | Validate output against JSON schema |
| `sleep` | Read Only | Pause execution |
| `send_user_message` | Read Only | Send notification to user |
| `powershell` | Varies | Execute PowerShell commands (Windows) |

Additional tools can be registered via **MCP servers** (unlimited).

### MCP (Model Context Protocol)

Full MCP client with 6 transport types:
- **stdio** — Local processes, npm packages
- **SSE** — Remote servers over HTTP
- **WebSocket** — Bidirectional real-time
- **HTTP** — REST API wrappers
- **SDK** — Direct in-process integration
- **Managed proxy** — Enterprise environments

```bash
/mcp                        # List connected servers
/mcp add my-server stdio npx -y @my/mcp-server
/mcp remove my-server
```

### Knowledge Base (`/knowledge`)

Agentic RAG powered by real BGE-M3 1024-dim embeddings, PostgreSQL pgvector, and Neo4j knowledge graph.

```bash
/knowledge best Meta Ads strategy for scaling D2C brands
```

**Pipeline:**
1. **Local intent classification** — Rust-side MOE classifies query intent (strategic, procedural, factual, etc.)
2. **API call** to hosted AgenticRAG server with intent hint
3. **Hybrid search** — pgvector cosine + PostgreSQL FTS + Neo4j graph expansion
4. **RRF fusion** — Reciprocal Rank Fusion merges results from all sources
5. **KnowledgeCard** — Tabbed, collapsible results with abstracted category labels
6. **Feedback** — Inline 👍/👎 buttons + `/feedback` command for corrections
7. **Local cache** — Results cached in `.openanalyst/knowledge/` for instant replay

**MOE Intent Types:** factual, conceptual, procedural, comparative, strategic, example_seeking, diagnostic, general

**No raw course names exposed** — results show abstracted labels like "Ads Strategy", "AI & Machine Learning".

Set `OPENANALYST_API_KEY=oa_your_key` to access the knowledge base.

### Permission Mode Switching (`Ctrl+P`)

Cycle through permission modes directly from the input box:

| Mode | Icon | Border | Behavior |
|------|------|--------|----------|
| Default | ❯ | Blue | Ask before running tools |
| Plan | ◈ | Yellow | Read-only tools only |
| Accept Edits | ✎ | Green | Auto-approve file write/edit |
| Danger | ⚡ | Red | Everything auto-approved |

Right-aligned badges on input box show: `[mode] [model] [agent] [branch]`

### Agent Selection from Sidebar

- Load agents from `.openanalyst/agents/*.md` (project + user level)
- Select agent in sidebar → changes input box title + system prompt
- Purple badge shows active agent name
- Dynamic system prompt switching without leaving the conversation

### Permission System

| Mode | Bash | Write | Edit | Install | Delete |
|------|------|-------|------|---------|--------|
| `read-only` | ✗ | ✗ | ✗ | ✗ | ✗ |
| `workspace-write` | ✓ | ✓ | ✓ | ✗ | ✗ |
| `danger-full-access` | ✓ | ✓ | ✓ | ✓ | ✓ |

Modal permission dialogs appear when a tool requires elevated access. Cycle with `Ctrl+P`.

### Hooks System

| Event | Fires When |
|-------|-----------|
| `PreToolUse` | Before a tool is executed |
| `PostToolUse` | After a tool completes |
| `SessionStart` | When a new session begins |
| `SessionEnd` | When a session ends |
| `CwdChanged` | When the working directory changes |
| `FileChanged` | When a file is modified |
| `TaskCreated` | When a new task is created |

### Plugin System
- Install, enable, disable plugins from `~/.openanalyst/plugins/`
- Full lifecycle management with `/plugins` command

---

## Architecture

OpenAnalyst CLI is a **14-crate Rust workspace**:

```text
rust/crates/
├── api/                   # Multi-provider API client (7 providers)
├── commands/              # 51+ slash commands
├── events/                # Shared TUI ↔ backend event types
├── orchestrator/          # Multi-agent lifecycle, MOE, model routing
├── tui/                   # Ratatui full-screen TUI application
├── tui-widgets/           # Widgets (markdown, tool cards, input, spinner)
├── runtime/               # Conversation engine, session, permissions, MCP
├── tools/                 # 19 built-in tool implementations
├── plugins/               # Plugin system (install, enable, hooks)
├── openanalyst-cli/       # Binary entry point
├── openanalyst-agent/     # Headless autonomous agent runner
├── server/                # HTTP/SSE server (axum)
├── lsp/                   # Language Server Protocol integration
└── compat-harness/        # Upstream manifest extraction
```

### Key Technologies
- **Ratatui 0.30** — Terminal UI framework
- **Tokio 1.x** — Async runtime (multi-threaded)
- **Crossterm 0.29** — Terminal backend
- **Reqwest 0.12** — HTTP client
- **Syntect 5.x** — Syntax highlighting
- **Tiktoken-rs** — Token counting
- **edtui** — Text editor widget (vim mode optional)
- **tui-markdown** — Markdown rendering (Ratatui team)
- **Axum** — HTTP/SSE server

### Ecosystem Crates Used (not reinvented)
- **tui-markdown** — Markdown rendering (by Ratatui core team)
- **edtui** — Vim-mode text editor widget
- **tui-tree-widget** — File tree sidebar
- **throbber-widgets-tui** — Animated spinners
- **syntect-tui** — Syntax highlighting bridge

---

## Configuration

| File | Purpose |
|------|---------|
| `OPENANALYST.md` | Project-specific AI instructions (auto-detected) |
| `.openanalyst.json` | Shared project defaults |
| `.openanalyst/settings.json` | Project settings (hooks, plugins, MCP, model, permissions) |
| `.openanalyst/settings.local.json` | Machine-local overrides (gitignored) |
| `~/.openanalyst/.env` | API keys and base URLs |
| `~/.openanalyst/credentials.json` | Saved OAuth tokens (auto-managed) |

### Environment Variables

**Authentication:**
| Variable | Provider |
|----------|----------|
| `OPENANALYST_AUTH_TOKEN` | OpenAnalyst (default) |
| `ANTHROPIC_API_KEY` | Anthropic / Claude |
| `OPENAI_API_KEY` | OpenAI / Codex |
| `GEMINI_API_KEY` | Google Gemini |
| `XAI_API_KEY` | xAI / Grok |
| `OPENROUTER_API_KEY` | OpenRouter |
| `BEDROCK_API_KEY` | Amazon Bedrock |

**Runtime:**
| Variable | Description |
|----------|-------------|
| `OPENANALYST_CONFIG_HOME` | Override config directory |
| `OPENANALYST_MODEL` | Override default model |
| `OPENANALYST_PERMISSION_MODE` | Set permission mode |

---

## Documentation

Full Mintlify-style documentation is included at [`docs/Documentation/index.html`](docs/Documentation/index.html) — 12 pages covering:

- Installation & Quick Start
- Authentication (OAuth + API keys)
- LLM Providers & model switching
- Terminal UI layout & keybindings
- All 51+ slash commands
- 19 built-in tools with permissions
- Multi-agent system (swarms, autonomous, MOE)
- Configuration & hooks
- MCP integration
- Architecture & building

---

## Legal & Credits

### OpenAnalyst CLI Is an Independent Product

OpenAnalyst CLI is **original software** developed by OpenAnalyst Inc. It is:

- **Not** a fork, copy, or derivative of any other product
- **Not** affiliated with, endorsed by, or maintained by Anthropic, OpenAI, Google, xAI, or any other provider
- Built using **industry-standard patterns** (agentic loops, tool calling, MCP, session management) that are common across the entire AI CLI ecosystem

The architectural patterns used — streaming tool execution, permission hierarchies, session persistence, system prompts with project context — are **well-established, open engineering practices** implemented independently by dozens of projects including Codex CLI, Gemini CLI, Aider, Goose, aichat, rust-code, kai, and many others. No single company owns these patterns.

### Provider Credits & Trademarks

OpenAnalyst CLI connects to third-party APIs. All credits belong to the respective providers:

- **Anthropic** — Claude, the Anthropic API, and all related trademarks are property of Anthropic, PBC
- **OpenAI** — GPT, DALL-E, Whisper, Codex, and all related trademarks are property of OpenAI, Inc
- **Google** — Gemini, Imagen, and all related trademarks are property of Google LLC
- **xAI** — Grok and all related trademarks are property of xAI Corp
- **Amazon** — Bedrock and all related trademarks are property of Amazon Web Services, Inc
- **OpenRouter** — OpenRouter is property of OpenRouter, Inc

Use of these providers' APIs through OpenAnalyst CLI is subject to each provider's respective Terms of Service. OpenAnalyst CLI is a client application that facilitates access to these APIs — it does not claim any ownership of or rights to the providers' services, models, or intellectual property.

### License

The Rust workspace is licensed under the **MIT License**. See `rust/LICENSE` for details.

### Disclaimer

THIS SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND. OpenAnalyst Inc makes no representations regarding the suitability of this software for any purpose. OpenAnalyst CLI is an independent product that connects to third-party APIs under their respective terms of service.

---

## Contact

- **Issues:** [github.com/OpenAnalystInc/openanalyst-cli/issues](https://github.com/OpenAnalystInc/openanalyst-cli/issues)
- **Email:** anit@openanalyst.com

---

<p align="center">
  <strong>OpenAnalyst CLI v1.0.91</strong> — Built by OpenAnalyst Inc<br>
  <em>An independent, open-source AI agent for the terminal.</em>
</p>
