# OpenAnalyst CLI

**The Universal AI Agent for Your Terminal.**

Connect to any major LLM provider through a single, unified terminal interface. 7 providers, 65+ commands, 24 built-in tools, smart model routing, multi-agent orchestration, and a full-featured TUI.

## Install

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.ps1 | iex
```

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.sh | bash
```

**npm:**
```bash
npm install -g @openanalystinc/openanalyst-cli
```

## Quick Start

```bash
openanalyst login     # authenticate with your LLM provider
openanalyst           # launch the TUI
```

## Features

| Feature | OpenAnalyst CLI |
|---------|----------------|
| **Providers** | OpenAnalyst, Anthropic Claude, OpenAI GPT, Google Gemini, xAI Grok, OpenRouter, Amazon Bedrock |
| **Commands** | 65+ slash commands — /commit, /pr, /undo, /model, /knowledge, /image, /bughunter |
| **Tools** | 24 built-in — bash, file I/O, grep, glob, web search, knowledge base |
| **TUI** | Full terminal UI with branded banner, inline tool cards, status bar |
| **Model Routing** | Smart per-task routing — explore, research, code, write |
| **Agents** | Autonomous agent, swarm execution, background tasks |
| **Knowledge Base** | Agentic RAG with vector + graph search |
| **Voice** | Microphone input with transcription |
| **Sessions** | Auto-save, resume, export |
| **MCP** | Model Context Protocol server support |
| **Permissions** | 4 modes — Default, Plan, Accept Edits, Danger |
| **Single Binary** | Native binary, no runtime dependencies (18 MB) |

## Providers

| Provider | Auth | Models |
|----------|------|--------|
| **OpenAnalyst** | Free model or API key | gpt-oss-120b, openanalyst-beta |
| **Anthropic / Claude** | API key | claude-opus-4-6, claude-sonnet-4-6, claude-haiku-4-5 |
| **OpenAI** | API key | gpt-4o, gpt-4.1, gpt-4o-mini |
| **Google Gemini** | OAuth or API key | gemini-2.5-pro, gemini-2.5-flash |
| **xAI / Grok** | API key | grok-3, grok-3-mini |
| **OpenRouter** | API key | 350+ models |
| **Amazon Bedrock** | API key | Bedrock models |

Switch models mid-conversation with `/model gpt-4o` — session persists across providers.

## Commands

```
/help          Show all commands
/model         Switch model or view available providers
/commit        Generate commit message and commit
/pr            Create a pull request
/undo          Revert all uncommitted file changes
/knowledge     Search the knowledge base (Agentic RAG)
/image         Generate an image
/bughunter     Scan codebase for bugs
/diff-review   AI-powered code review
/openanalyst   Run autonomous agent
/swarm         Spawn parallel agents
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| **Enter** | Submit prompt |
| **Shift+Enter** | New line |
| **Ctrl+P** | Cycle permission mode |
| **Ctrl+C** | Cancel / quit |
| **Ctrl+V** | Paste from clipboard |
| **Ctrl+Z** | Undo |
| **Esc** | Enter scroll mode |
| **Up/Down** | Input history |
| **Tab** | Toggle sidebar |

## Support

- **Email**: support@openanalyst.com
- **Website**: [openanalyst.com](https://openanalyst.com)

---

Developed by **OpenAnalyst Inc.** All rights reserved.
