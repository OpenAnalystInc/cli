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
| **Mid-conversation model switching** | Session persists across providers | Not supported |
| **TUI** | Full Ratatui-based terminal UI (default) | Most use basic REPL |
| **Multi-agent orchestrator** | Built-in parallel agent spawning | Limited or none |
| **Multimedia** | /image, /voice, /speak, /vision, /diagram | Rarely supported |
| **38 slash commands** | Git, AI planning, multimedia, web scraping | 5-15 typical |
| **Single binary** | Native Rust, no runtime dependencies | Often needs Node/Python |

---

## Quick Start

### 1. Install

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.sh | bash
```

**Windows PowerShell:**
```powershell
irm https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.ps1 | iex
```

**Build from source:**
```bash
cd rust && cargo build --release
```

### 2. Login

```bash
openanalyst login
```

Interactive provider picker — select provider with arrow keys, enter API key, connection tested automatically.

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

| Provider | Auth Variable | Models |
|----------|--------------|--------|
| **OpenAnalyst** (default) | `OPENANALYST_AUTH_TOKEN` | Fetched live from API |
| **Anthropic / Claude** | `ANTHROPIC_API_KEY` | opus, sonnet, haiku — fetched live |
| **OpenAI / GPT** | `OPENAI_API_KEY` | gpt-4o, o3, codex — fetched live |
| **Google Gemini** | `GEMINI_API_KEY` | gemini-2.5-pro, gemini-2.5-flash — fetched live |
| **xAI / Grok** | `XAI_API_KEY` | grok-3, grok-mini — fetched live |
| **OpenRouter** | `OPENROUTER_API_KEY` | 350+ models from any provider |
| **Amazon Bedrock** | `BEDROCK_API_KEY` | Fetched live from gateway |

**All credits and trademarks belong to their respective providers.** Claude is a trademark of Anthropic. GPT is a trademark of OpenAI. Gemini is a trademark of Google. Grok is a trademark of xAI. OpenAnalyst CLI is an independent tool that connects to these providers' public APIs.

---

## Features

### Full Terminal UI (Ratatui-based)
- Scrollable chat with inline tool call cards
- Startup banner with account info and OA block letters
- Status line with spinner, elapsed time, and token count
- Vim-mode input (via edtui)
- Permission dialogs as modal overlays
- Mouse scroll and keyboard navigation

### Multi-Agent Orchestrator
- Spawn sub-agents for parallel tasks
- Each agent has its own conversation runtime and tool permissions
- Agent lifecycle events displayed in real-time
- Channel-based async bridge (sync runtime ↔ async TUI)

### 38 Slash Commands

**Session:** /help, /status, /cost, /model, /clear, /compact, /session, /export, /resume, /version
**Code & Git:** /diff, /commit, /commit-push-pr, /pr, /issue, /branch, /worktree, /teleport, /diff-review
**Analysis:** /bughunter, /ultraplan, /debug-tool-call
**Multimedia:** /image, /voice, /speak, /vision, /diagram
**Web:** /scrape, /json
**AI:** /translate, /tokens
**Config:** /config, /memory, /init, /permissions, /plugins, /agents, /skills

### 19 Built-in Tools
Bash, ReadFile, WriteFile, EditFile, GlobSearch, GrepSearch, WebSearch, WebFetch, Agent, TodoWrite, NotebookEdit, Skill, ToolSearch, Sleep, SendUserMessage, Config, StructuredOutput, REPL, PowerShell

---

## Architecture

OpenAnalyst CLI is a **14-crate Rust workspace**:

```text
rust/crates/
├── api/                   # Multi-provider API client (7 providers)
├── commands/              # 38 slash commands
├── events/                # Shared TUI ↔ backend event types
├── orchestrator/          # Multi-agent lifecycle + channel bridge
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
| `.openanalyst/settings.json` | Project settings |
| `.openanalyst/settings.local.json` | Machine-local overrides (gitignored) |
| `~/.openanalyst/credentials.json` | Saved provider API keys |

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

- **Issues:** [github.com/AnitChaudhry/openanalyst-cli/issues](https://github.com/AnitChaudhry/openanalyst-cli/issues)
- **Email:** anit@openanalyst.com

---

<p align="center">
  <strong>OpenAnalyst CLI v1.0.1</strong> — Built by OpenAnalyst Inc<br>
  <em>An independent, open-source AI agent for the terminal.</em>
</p>
