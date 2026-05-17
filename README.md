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

## License

Copyright (c) 2026 OpenAnalyst Inc.

OpenAnalyst CLI is released under the
[Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
The full text is in the [LICENSE](LICENSE) file at the project root.

Apache 2.0 grants you the right to use, modify, and redistribute
the software (including for commercial purposes), provided that
you include the license, state any modifications, and preserve
copyright, patent, trademark, and attribution notices. A
royalty-free patent grant is included. The software is provided
"AS IS" with no warranty.

The Apache 2.0 license does NOT grant rights to the
**OpenAnalyst** name or marks beyond fair attribution. For
questions: support@openanalyst.com.
