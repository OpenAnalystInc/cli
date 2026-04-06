# [OpenAnalyst CLI](https://openanalystinc.github.io/cli/)

**The Universal AI Agent for Your Terminal.**

Connect to any major LLM provider through a single, unified terminal interface. 8 providers, 65+ commands, 22 built-in tools, smart model routing, multi-agent orchestration, and a full-featured TUI.

## Install

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/OpenAnalystInc/openanalyst-cli/master/install.ps1 | iex
```

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/openanalyst-cli/master/install.sh | bash
```

**npm:**
```bash
npm install -g @openanalystinc/openanalyst-cli
```

## Quick Start

```bash
openanalyst           # launch the TUI
/login openai sk-...  # authenticate inside the TUI
```

Or set your API key before launching:
```bash
export OPENAI_API_KEY=sk-...
openanalyst
```

## Features

| Feature | OpenAnalyst CLI |
|---------|----------------|
| **Providers** | OpenAnalyst, Anthropic, OpenAI, Google Gemini, xAI, OpenRouter, Amazon Bedrock, Stability AI |
| **Commands** | 65+ slash commands — /commit, /pr, /model, /knowledge, /image, /bughunter, /swarm |
| **Tools** | 22 built-in — bash, PowerShell, file I/O, grep, glob, web search/fetch, REPL, agents, KB |
| **TUI** | Full terminal UI with branded banner, inline tool cards, diff view, streaming markdown |
| **Model Routing** | Smart per-task routing — fast models for explore, balanced for code, capable for planning |
| **Agents** | Autonomous agent, swarm execution, background tasks |
| **Knowledge Base** | Agentic RAG with vector + graph search |
| **Voice** | Microphone input with Whisper transcription |
| **Sessions** | Auto-save, resume, team sharing via git |
| **MCP** | Model Context Protocol with Playwright browser tools |
| **Permissions** | 4 modes — Default, Plan, Accept Edits, Danger |
| **Cost Tracking** | Per-model session costs with real-time display |
| **API Server** | HTTP API with SSE streaming for remote/cloud deployment |
| **Team Collaboration** | Sessions, todos, and plans shared via .openanalyst/ in git |

## Providers

| Provider | Auth | Models |
|----------|------|--------|
| **OpenAnalyst** | API key | openanalyst-beta |
| **Anthropic** | API key | Opus 4.6, Sonnet 4.6, Haiku 4.5 |
| **OpenAI** | API key | GPT-5.4, GPT-4.1, GPT-4o, o3, o4 Mini, Codex Mini |
| **Google Gemini** | OAuth or API key | Gemini 3.1 Pro, Gemini 3 Flash, Gemini 2.5 Pro/Flash |
| **xAI** | API key | Grok 4, Grok 4 Fast, Grok 3 |
| **OpenRouter** | API key | 350+ models (DeepSeek V4, Llama 4, Mistral, etc.) |
| **Amazon Bedrock** | API key | Bedrock Claude |
| **Stability AI** | API key | Stability models |

Switch models mid-conversation with `/model sonnet-4.6` — context persists across providers.

## Commands

```
/help            Show all commands and keybindings
/login           Authenticate with any AI provider
/logout          Remove credentials
/model           Switch AI model
/models          List all available models
/permissions     Show or set permission mode
/commit          Generate commit message and commit
/pr              Create a pull request
/knowledge       Search the knowledge base (Agentic RAG)
/image           Generate an image
/bughunter       Scan codebase for bugs
/diff-review     AI-powered code review
/openanalyst     Run autonomous agent loop
/swarm           Spawn parallel agents
/compact         Compact conversation context
/resume          Resume a previous session
/vim             Toggle vim mode
/sidebar         Toggle sidebar panel
/exit            Exit to terminal
```

65 commands total — type `/help` to see all with aliases.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| **Enter** | Submit prompt |
| **Shift+Enter** | New line |
| **Ctrl+P** | Cycle permission mode (Default → Plan → Accept Edits → Danger) |
| **Ctrl+E** | Toggle sidebar panel |
| **Ctrl+C** | Stop AI / quit (double-press to exit) |
| **Ctrl+B** | Run current prompt in background |
| **Ctrl+L** | Clear chat |
| **Esc** | Stop AI execution / enter scroll mode |
| **Up/Down** | Input history navigation |
| **j/k** | Navigate messages in scroll mode |
| **y** | Copy focused message to clipboard |

## Smart Model Routing

OpenAnalyst intelligently selects the right model for each task:

| Task Type | Model Tier | Example (Anthropic) | Example (OpenAI) |
|-----------|-----------|---------------------|-------------------|
| **Explore** (file search, scanning) | Fast | Haiku 4.5 | GPT-5.4 Nano |
| **Research** (reading, understanding) | Balanced | Sonnet 4.6 | GPT-5.4 Mini |
| **Code** (writing, editing) | Balanced | Sonnet 4.6 | GPT-5.4 Mini |
| **Write** (planning, complex reasoning) | Capable | Opus 4.6 | GPT-5.4 |

Change routing in the sidebar (Ctrl+E → navigate to Routing → Enter to cycle tier).

## Team Collaboration

When your project has `.openanalyst/` checked into git, team members get:

- **Shared conversations** — resume any team member's AI session
- **Shared tasks** — see pending todos and plans
- **Shared instructions** — project-specific OPENANALYST.md
- **Shared settings** — consistent model routing and permissions

Each developer uses their own API keys. Credentials are never shared.

## API Server

Deploy OpenAnalyst on a remote server for API access:

```bash
openanalyst serve --port 8080
```

```bash
# Simple one-shot query
curl -X POST http://localhost:8080/v1/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "explain this codebase"}'

# Session-based with SSE streaming
curl -X POST http://localhost:8080/sessions
curl http://localhost:8080/sessions/session-1/events
```

## Configuration

Global config: `~/.openanalyst/`
Project config: `.openanalyst/` (checked into git)

```
~/.openanalyst/
├── OPENANALYST.md       # Global AI instructions
├── settings.json        # Global settings
├── .env                 # API keys
├── credentials.json     # Credential storage
├── commands/            # Custom slash commands
├── rules/               # Global rules
├── agents/              # Custom agent definitions
└── sessions/            # Chat history
```

## Support

- **Email**: support@openanalyst.com
- **Website**: [openanalyst.com](https://openanalyst.com)
- **GitHub**: [OpenAnalystInc/openanalyst-cli](https://github.com/OpenAnalystInc/openanalyst-cli)

---

Developed by **OpenAnalyst Inc.** All rights reserved.
