# OpenAnalyst CLI

<p align="center">
  <strong>Experimental AI-Powered Code Analysis & Engineering Agent</strong><br>
  <em>Built by OpenAnalyst Inc for internal research and development</em>
</p>

---

> **Notice:** This project is an experimental, internally-focused CLI tool developed by **OpenAnalyst Inc** for research, testing, and evaluation of AI agent architectures. It is **not** a commercial product and is **not intended for redistribution**.

---

## Attribution & Credits

This project is built upon and inspired by the **open-source agent harness architecture** pioneered by [Anthropic](https://anthropic.com) through their [Claude Code](https://docs.anthropic.com/en/docs/claude-code) CLI tool.

**All architectural credit belongs to Anthropic and the Claude Code team.** The core patterns — including the agentic loop, tool execution framework, permission system, session management, MCP orchestration, and system prompt construction — originate from Anthropic's engineering work.

OpenAnalyst CLI exists as:
- An **experimental testbed** for evaluating multi-provider LLM routing within a unified CLI interface
- An **internal tool** for OpenAnalyst Inc's engineering workflows and research
- A **learning exercise** in systems-level Rust implementation of agent harness patterns

**This project does not claim originality over the underlying agent architecture.** It is a derivative work that adapts and extends the open-source patterns established by Anthropic, with modifications for multi-provider support and internal tooling needs.

### Specific acknowledgements:
- **Anthropic** — Original Claude Code architecture, API protocol design, and agent harness patterns
- **Claude Code** — The reference implementation that this project's structure is derived from
- The open-source Rust port community that provided the initial clean-room reimplementation foundation

---

## What This Project Is

OpenAnalyst CLI is a **unified AI agent CLI** that connects to multiple LLM providers through a single interface, using the OpenAnalyst API as its default endpoint. It is used internally by OpenAnalyst Inc for:

- Evaluating LLM provider performance across different coding tasks
- Testing the OpenAnalyst API's compatibility layer
- Internal engineering productivity workflows
- Research into agent harness patterns and tool orchestration

## What This Project Is NOT

- This is **not** an official Anthropic product
- This is **not** affiliated with, endorsed by, or maintained by Anthropic
- This is **not** a commercial offering or a replacement for Claude Code
- This does **not** contain any proprietary Anthropic source code

---

## Quick Start

### 1. Install

**macOS / Linux (curl):**
```bash
curl -fsSL https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.sh | bash
```

**npm (all platforms):**
```bash
npm install -g @openanalyst/cli
```

**Windows PowerShell:**
```powershell
irm https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.ps1 | iex
```

**Or build from source:**
```bash
cd rust && cargo build --release
```

### 2. Login

```bash
openanalyst login
```

Interactive provider picker — select provider with arrow keys, enter API key, connection tested automatically, credentials saved to `~/.openanalyst/credentials.json`. Run multiple times to add keys for different providers.

```
  Select your LLM provider:

  > OpenAnalyst          OpenAnalyst API (default)
    Anthropic / Claude
    OpenAI / GPT
    xAI / Grok
    OpenRouter
    Amazon Bedrock

  Enter your API key: sk-oa-...
  Testing connection... Connected
  Login complete
```

**Or set env vars directly:**

```bash
export OPENANALYST_AUTH_TOKEN="your-api-key-here"   # OpenAnalyst
export ANTHROPIC_API_KEY="sk-ant-..."               # Claude
export OPENAI_API_KEY="sk-..."                      # GPT
export OPENROUTER_API_KEY="sk-or-..."               # OpenRouter
export XAI_API_KEY="xai-..."                        # Grok

# Amazon Bedrock
export BEDROCK_API_KEY="..."
```

**Override the default model:**
```bash
export OPENANALYST_MODEL="openanalyst-beta"
# or
export ANTHROPIC_DEFAULT_SONNET_MODEL="openanalyst-beta"
```

### 3. Start Using OpenAnalyst CLI

```bash
$ cd your-project
$ openanalyst

# OpenAnalyst CLI is now connected
# Using model: openanalyst-beta
```

---

## Features

- **Cross-provider `/model` switching** — Switch from OpenAnalyst to GPT to Grok to Claude mid-conversation. Session ID, chat history, tool results, and context persist across provider boundaries. No other CLI does this.
- **Live model discovery** — `/model` fetches available models from each provider's API in real-time. No hardcoded model lists.
- **Interactive `openanalyst login`** — Arrow-key provider picker, API key input, connection test, credential persistence. Run multiple times to add keys for different providers. All keys loaded automatically on startup.
- **6 LLM providers** — OpenAnalyst (default), Anthropic, OpenAI, xAI, OpenRouter, Amazon Bedrock
- **Persistent sessions** — Save, resume, list, and export conversations. Sessions survive model and provider switches.
- **12 built-in tools** — Bash, ReadFile, WriteFile, EditFile, GlobSearch, GrepSearch, WebSearch, WebFetch, Agent, TodoWrite, NotebookEdit, Skill
- **30+ slash commands** — `/help`, `/status`, `/model`, `/cost`, `/commit`, `/pr`, `/bughunter`, `/ultraplan`, and more
- **OPENANALYST.md** — Project-specific AI instructions, auto-detected in directory tree
- **Permission system** — Read-only, workspace-write, or full access modes
- **Cross-platform** — Native binaries for Windows, macOS (Intel + Apple Silicon), Linux (x64 + ARM)

## Usage

```bash
# Interactive REPL (default provider)
openanalyst

# One-shot prompt
openanalyst "explain this codebase"

# Use a specific model (auto-detects provider)
openanalyst --model gpt-4o "summarize this repo"
openanalyst --model claude-sonnet-4-6 "fix the bug"
openanalyst --model grok "review this PR"
openanalyst --model openrouter/anthropic/claude-3.5-sonnet "explain"

# JSON output for scripting
openanalyst --output-format json prompt "list all functions"

# Resume a session
openanalyst --resume session.json /status

# Initialize project config
openanalyst init
```

## Supported Providers

| Provider | Auth Variable | Models |
|----------|--------------|--------|
| **OpenAnalyst** (default) | `OPENANALYST_AUTH_TOKEN` | Fetched live from API |
| **Anthropic / Claude** | `ANTHROPIC_API_KEY` | Fetched live from API |
| **OpenAI / GPT** | `OPENAI_API_KEY` | Fetched live from API |
| **xAI / Grok** | `XAI_API_KEY` | Fetched live from API |
| **OpenRouter** | `OPENROUTER_API_KEY` | 350+ models, fetched live |
| **Amazon Bedrock** | `BEDROCK_API_KEY` | Fetched live from gateway |

Run `/model` inside the CLI to see all available models from your configured providers.

## Configuration

| File | Purpose |
|------|---------|
| `.openanalyst.json` | Shared project defaults |
| `.openanalyst/settings.json` | Project settings |
| `.openanalyst/settings.local.json` | Machine-local overrides (gitignored) |
| `OPENANALYST.md` | Project-specific AI guidance |

## Repository Layout

```text
.
├── rust/                               # Rust implementation (primary)
│   ├── crates/api/                     # Multi-provider API client + streaming
│   ├── crates/runtime/                 # Session, tools, MCP, config
│   ├── crates/openanalyst-cli/         # Interactive CLI binary
│   ├── crates/plugins/                 # Plugin system
│   ├── crates/commands/                # Slash commands & skills
│   ├── crates/server/                  # HTTP/SSE server (axum)
│   ├── crates/lsp/                     # LSP client integration
│   └── crates/tools/                   # Built-in tool implementations
├── src/                                # Python reference workspace
├── install.sh                          # macOS / Linux installer
├── install.ps1                         # Windows installer
└── README.md
```

## Version

**OpenAnalyst CLI v1.0.1** — Experimental internal release

---

## Legal

### License

The Rust workspace is licensed under the **MIT License**. See `rust/LICENSE` for details.

### Disclaimer

THIS SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND. OpenAnalyst Inc makes no representations or warranties regarding the suitability of this software for any purpose.

This project is a derivative work based on publicly available open-source agent harness patterns. **All rights to the original Claude Code architecture, design patterns, API protocols, and associated intellectual property belong to Anthropic, PBC.** This project does not claim ownership of, nor rights to, any Anthropic intellectual property.

OpenAnalyst Inc acknowledges that:
1. The architectural patterns in this project originate from Anthropic's Claude Code
2. Anthropic retains all rights to their original work
3. This project exists solely for internal experimental and research purposes
4. This project is not a commercial product and is not offered as a service
5. Any use of Anthropic's API through this tool is subject to Anthropic's Terms of Service

If Anthropic or its representatives have concerns about this project, please contact: **anit@openanalyst.com**

### Third-Party Acknowledgements

- [Anthropic](https://anthropic.com) — Claude Code architecture and API
- [OpenAI](https://openai.com) — Chat completions API protocol
- [Rust](https://www.rust-lang.org) — Programming language
- Open-source dependencies listed in `rust/Cargo.lock`

---

<p align="center">
  <em>Built with respect for the open-source community and the engineers at Anthropic who pioneered this architecture.</em>
</p>
