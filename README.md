# OpenAnalyst CLI

Public distribution repository for the latest OpenAnalyst CLI binaries, install scripts, and documentation.

## Install

```bash
npm install -g @openanalystinc/openanalyst-cli
```

macOS / Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.ps1 | iex
```

## Usage

```bash
openanalyst
openanalyst --notui
openanalyst --serve 8080
```

- `openanalyst` launches the default interactive TUI.
- `openanalyst --notui` runs the backend CLI without the TUI.
- `openanalyst --serve 8080` exposes the hosted session API:
  - `POST /sessions`
  - `POST /sessions/{id}/message`
  - `GET /sessions/{id}/events`

## Providers

OpenAnalyst CLI supports:

- OpenAnalyst
- Anthropic / Claude
- OpenAI / GPT / Codex
- Google Gemini
- xAI / Grok
- OpenRouter
- Amazon Bedrock
- Stability AI

## Docs

Public docs are published from the `docs/` folder in this repository and mirrored at:

- [openanalystinc.github.io/cli](https://openanalystinc.github.io/cli/)

## Release Contents

This repository is intentionally limited to public release assets:

- install scripts
- documentation HTML
- packaged release binaries
- npm package metadata

Source development for the private release pipeline happens separately.
