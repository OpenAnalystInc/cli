# OpenAnalyst CLI

**The Universal AI Agent for Your Terminal — and now your Browser.**

OpenAnalyst is a single, focused AI development tool you can use three ways with the same install: as a rich terminal UI (TUI), as a browser-based web console pointed at your local machine, or as a self-hosted service on your own cloud infrastructure. Workspace-jailed by default, BYO-key for every major provider, zero lock-in.

## Install

The fastest path on any OS:

```bash
npm install -g @openanalystinc/openanalyst-cli
```

That gives you the `openanalyst` (and short `oa`) binary on your `$PATH`. If you prefer not to use npm:

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.sh | bash
```

**Windows PowerShell:**

```powershell
irm https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.ps1 | iex
```

Verify it installed:

```bash
openanalyst --version
```

## Sign in

You have three ways to give the CLI credentials. We recommend **signing in with an OpenAnalyst account** — one credit balance across every frontier model, the [10x.in](https://10x.in) skills & plugins catalog included, and no per-provider key management.

### 1. Sign in with your OpenAnalyst account (recommended)

**Step 1 — create a free account at [10x.in](https://10x.in)** (30 seconds, just an email). New accounts get free credits to try out the CLI immediately.

**Step 2 — sign in from your terminal:**

```bash
openanalyst account login
```

The CLI prompts for your email, sends a 6-digit code, you paste it back, and you're in. No browser, no long keys.

```
  OpenAnalyst CLI — Account sign in

  Email: you@example.com
  Sent a verification code to y***@example.com.
  Code: 123456

  ✓ Signed in as you@example.com

  You can now run:
    openanalyst                 start the TUI
    openanalyst "<prompt>"      one-shot prompt
    openanalyst account status  show plan + credits
```

**Step 3 — use it.** Just start the TUI and pick a model from the picker, or one-shot a prompt:

```bash
openanalyst                              # interactive TUI
openanalyst "explain this codebase"      # one-shot
openanalyst -m opus "review my diff"     # one-shot with explicit model
openanalyst account status               # plan + credits left, anytime
openanalyst account logout               # sign out
```

**What the account unlocks** (everything below, on one credit balance):

- the **OpenAnalyst-routed model catalog** — see [Models](#models) below.
- the **[10x.in/skills](https://10x.in/skills) marketplace** — `/code-review`, `/release-notes`, `/security-audit`, `/standup`, `/onboarding` and many more, installable as first-class slash commands.
- the **[10x.in/plugins](https://10x.in/plugins) catalog** — CRM connectors, internal-doc readers, deploy helpers, custom MCP tool servers.
- **usage analytics, team seats, and SSO** at [10x.in/team](https://10x.in/team).

> **Token lifetime:** the CLI-issued access token expires after ~1 hour. The web console (`openanalyst --serve` → Settings → Account) refreshes it automatically. For the pure-CLI path, just re-run `openanalyst account login` if you see a 401 — it's a 10-second roundtrip.

### 2. Paste an OpenAnalyst API key

Prefer a long-lived key instead of the OTP flow? Go to [10x.in](https://10x.in) → **Dashboard → API Keys**, mint an `sk-oa-v1-…` key, and set it either as an env var:

```bash
export OPENANALYST_API_KEY=sk-oa-v1-...
openanalyst
```

…or once in `~/.openanalyst/.env` (`OPENANALYST_API_KEY=sk-oa-v1-...`) so every shell picks it up. Same model catalog, same credits — just authenticated with a static key instead of a session.

### 3. Bring your own provider keys (BYOP)

Already pay for Anthropic / OpenAI / Gemini / xAI / OpenRouter / Bedrock directly and want to use those keys? Fully supported.

```bash
# Either as env vars …
export ANTHROPIC_API_KEY=sk-ant-...
export OPENAI_API_KEY=sk-...
export GEMINI_API_KEY=...
openanalyst

# … or persisted in the dotenv file once:
echo "ANTHROPIC_API_KEY=sk-ant-..." >> ~/.openanalyst/.env
```

The CLI auto-discovers what's available and surfaces only the models you can actually use. You can **mix BYOP with an OpenAnalyst login** — both work side-by-side; pick per-session which one inference uses (CLI flag `--model`, web console Settings → Account → "Use account login / Use API key"). BYOP calls bill against your direct provider account; OpenAnalyst-routed calls bill against your OpenAnalyst credits.

## Models

There are two distinct ways to address a model — pick whichever matches how you're authenticated:

### OpenAnalyst-routed models (when signed in / using an `sk-oa-v1-*` key)

These route through `api.openanalyst.com` and bill against your OpenAnalyst credit balance. **One credential, every frontier model.** The exact catalog updates server-side as new models launch — run `openanalyst account status` or open Settings → Account in the web console to see the current list for your plan. As of 2.0.37:

| Model name (CLI) | What it is | Best for |
|---|---|---|
| `openanalyst-beta` | OpenAnalyst's smart-routed default — picks the right frontier model for the task automatically | Daily use; let the gateway decide what to call |
| `openanalyst-max` | Routed to the strongest available reasoning model | Hard problems, long contexts, complex coding |
| `opus`, `sonnet`, `haiku` | Claude Opus / Sonnet / Haiku via the OpenAnalyst gateway | When you specifically want Anthropic models |
| `gpt-5`, `gpt-4.1`, `o3`, `o4-mini` | GPT family via the OpenAnalyst gateway | When you specifically want OpenAI models |
| `gemini-3-pro`, `gemini-3-flash` | Gemini via the OpenAnalyst gateway | Large contexts, multimodal, lower cost |
| `grok-4`, `grok-4-fast` | xAI Grok via the OpenAnalyst gateway | When you specifically want xAI models |

All of the above just work after `openanalyst account login` — no per-provider keys needed. Switch between them anytime with `-m <model>` or the in-TUI model picker (`/model <name>`).

### Direct-provider models (BYOP — your own keys)

When you set a provider's own env var (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, …), the CLI talks to that provider's API directly, billed against your account there. Same model names as the provider's docs — `claude-opus-4-6`, `gpt-4.1`, `gemini-3-pro`, `grok-4`, etc.

**OpenAnalyst-routed vs direct — what's the difference?**

|  | OpenAnalyst-routed | Direct (BYOP) |
|---|---|---|
| Credential | OAuth (`account login`) or one `sk-oa-v1-*` key | One key per provider |
| Billing | OpenAnalyst credits, single invoice | Each provider bills separately |
| Model name | Curated aliases (`openanalyst-beta`, `opus`, `gpt-5`) | Provider's exact name (`claude-opus-4-6`, `gpt-4.1`) |
| Setup | 30 seconds — one login | Sign up for each provider, mint keys, manage rotation |
| Catalog | Grows automatically as 10x.in adds new models | What you've subscribed to with each provider |
| Account features | Credits, usage analytics, team seats, skills, plugins | None — bare API access |
| Failover | Automatic across providers in the route | Manual |

Both paths work, neither is locked in, and you can mix them session-by-session.

## Three ways to use it

**1. TUI in your terminal.** Just type `openanalyst` in any folder. The whole project becomes your workspace; the model can read and edit files only inside that folder.

**2. Web console in your browser.** Run `openanalyst --serve` from your project folder. Open the OpenAnalyst web console (hosted at `openanalystinc.github.io/cli/console` or self-built from the `web-ui/` source) and point it at `http://localhost:3080`. The Settings → **Account** tab gives you the same email/OTP sign-in flow as the CLI; the **Providers** tab lets you paste any other API keys you already have. The file viewer on the right side shows everything the model creates in your workspace, with one-click download.

**3. Self-host for your team.** Deploy `openanalyst --serve` on your AWS / Fly / Render / VPS. Build the `web-ui/` SPA pointing at your backend URL. Drop the static files on S3 + CloudFront. One client deployment per workspace, locked to one folder, fully your infrastructure.

## Usage

```bash
openanalyst                              # launch the default interactive TUI
openanalyst "what does this repo do?"    # one-shot prompt, prints to stdout
openanalyst account login                # sign in with email + OTP (no browser)
openanalyst account status               # show your plan + credits left
openanalyst account logout               # sign out
openanalyst --resume                     # restore the most recent saved session on launch
openanalyst --notui                      # backend CLI without the TUI
openanalyst --serve 8080                 # hosted HTTP session API
openanalyst --acp                        # Agent Client Protocol — JSON-RPC 2.0 over stdio
openanalyst --acp --session-id ID        # ACP session pinned to a persistent id
openanalyst --acp --session-id ID --conversation-id CONV    # plus a stable conversation handle
openanalyst --control-socket             # local-only IPC for external apps to spawn / list / load sessions
```

- `openanalyst` launches the default interactive TUI.
- `openanalyst --resume` rehydrates the most recently saved session on startup, equivalent to running `/resume` after launch.
- `openanalyst --notui` runs the backend CLI without the TUI.
- `openanalyst --serve 8080` exposes the hosted session API:
  - `POST /sessions`
  - `POST /sessions/{id}/message`
  - `GET /sessions/{id}/events`
- `openanalyst --acp` speaks Agent Client Protocol (JSON-RPC 2.0 over stdio) — used by IDE integrations and the embedding-app pattern below.
- `openanalyst --control-socket` runs a local-only Unix domain socket (Linux/macOS) or named pipe (Windows) at `~/.openanalyst/control.sock` for orchestrators and external apps. Token-gated via `~/.openanalyst/control.token` (mode 0600 on Unix). Exposes `authenticate`, `list`, `loadSession`, `spawn`, `shutdown`.

## Persistent multi-session

Pass `--session-id` and `--conversation-id` to `--acp` to make the CLI durable across restarts. Sessions are persisted to `~/.openanalyst/sessions.db` (single SQLite file, WAL mode, cross-process safe). The same `session_id` re-attached later picks up exactly where the previous process left off. External apps remember the stable `conversation_id` — `session_id` is the runtime handle and may rotate; `conversation_id` is the long-lived thread.

The Agent tool accepts an `isolation` parameter — set to `"process"` to spawn the sub-agent as a child `openanalyst-cli --acp` subprocess for crash isolation, true parallel CPU work, and restart durability. Default is `"thread"` for cheap, short-lived workers.

## Workspace jail

Every file operation is jailed to the folder the CLI was launched from. `Read`, `Write`, `Edit`, `Glob`, `Grep`, and `Bash` all canonicalize the requested path and refuse anything outside the workspace root. Path traversal (`../../etc/passwd`) is blocked. Symlinks are resolved before the check.

This is admin-controlled. The end user using the web console cannot disable the jail. Only the person who starts the server can — by setting `OPENANALYST_DISABLE_WORKSPACE_JAIL=1` before launching. Trusted external paths (e.g., shared library folders) can be allowlisted in `~/.openanalyst/settings.json` under `workspace_allow_paths`.

## Providers

The full list of authentication slots the CLI recognizes. Set as env vars, drop in `~/.openanalyst/.env`, or paste in the web console's Settings → Providers tab — keys are encrypted on disk.

**Chat / language models:**

| Provider | Credential | Env var | How to authenticate |
|---|---|---|---|
| **OpenAnalyst** (recommended) | account login OR `sk-oa-v1-*` key | `OPENANALYST_API_KEY` / `OPENANALYST_AUTH_TOKEN` | `openanalyst account login` or [10x.in](https://10x.in) → Dashboard → API Keys |
| Anthropic / Claude | API key | `ANTHROPIC_API_KEY` | [console.anthropic.com](https://console.anthropic.com) |
| OpenAI / GPT / Codex | API key | `OPENAI_API_KEY` | [platform.openai.com](https://platform.openai.com) |
| Google Gemini | API key | `GEMINI_API_KEY` | [aistudio.google.com](https://aistudio.google.com) |
| xAI / Grok | API key | `XAI_API_KEY` | [console.x.ai](https://console.x.ai) |
| OpenRouter | API key (one key, 350+ models) | `OPENROUTER_API_KEY` | [openrouter.ai](https://openrouter.ai) |
| Amazon Bedrock | API key | `BEDROCK_API_KEY` | AWS Bedrock console |

**Image generation** (used by the `/image` slash command):

| Provider | Env var |
|---|---|
| Stability AI (SDXL, SD3) | `STABILITY_API_KEY` |
| Google Imagen (via Gemini) | `GEMINI_IMAGE_API_KEY` (can reuse `GEMINI_API_KEY`) |
| OpenAI Images (DALL·E / gpt-image) | `OPENAI_IMAGE_API_KEY` (can reuse `OPENAI_API_KEY`) |

**Web search** (used by `/scrape` and the `WebSearch` tool — the CLI picks the first one available):

| Provider | Env var |
|---|---|
| Brave Search (independent index) | `BRAVE_SEARCH_API_KEY` |
| Tavily (RAG-tuned) | `TAVILY_API_KEY` |
| Serper (Google SERP) | `SERPER_API_KEY` |
| Exa (semantic web) | `EXA_API_KEY` |

## Skills & plugins (10x.in)

The CLI extends beyond raw model calls with **skills** (typed slash commands like `/code-review`) and **plugins** (custom tools like a CRM connector). Both are pulled from the [10x.in](https://10x.in) catalog once you're signed in.

- **[10x.in/skills](https://10x.in/skills)** — browse and install curated skills. Examples: `/code-review`, `/release-notes`, `/security-audit`, `/standup`, `/onboarding`. Install with `openanalyst skills install <name>` — they appear in the slash-command palette instantly.
- **[10x.in/plugins](https://10x.in/plugins)** — first-party + community plugins that add tools to the agent: CRM connectors, internal-doc readers, deploy helpers, MCP servers. `openanalyst extension install <plugin>` to add.
- **Author your own** — write a skill at [10x.in/skills/new](https://10x.in/skills/new), publish it to your team (private) or the world (public). Local skills under `.openanalyst/skills/` work too.

You can also create custom **agents** (named personas with their own system prompts, tools, and default models) — define them in `~/.openanalyst/agents/` and invoke with `/agent <name>` or list them with `openanalyst agents`.

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
