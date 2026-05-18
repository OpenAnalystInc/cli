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

You have three ways to give the CLI credentials. **We recommend signing in with an OpenAnalyst account** — you get one place to pay, one credit balance across every frontier model, plus the [10x.in](https://10x.in) skills & plugins catalog that turns the CLI into your team's dedicated workflow runner.

**1. Sign in with your OpenAnalyst account (recommended).** Free to start. One command, no API key juggling:

```bash
openanalyst account login
```

Don't have an account yet? Create one in 30 seconds at **[10x.in](https://10x.in)** — that's where you sign up, get free credits, manage plans, and grab an OpenAnalyst API key if you'd rather paste one in. The same login gives you:

- the full OpenAnalyst-routed model catalog — **Beta, Max, GPT-5.4, Claude Opus 4.7, Gemini 3 Pro** — through one balance, no per-provider keys.
- the **skills marketplace** at [10x.in/skills](https://10x.in/skills) — install pre-built `/skills` like `/code-review`, `/release-notes`, `/security-audit` that run as first-class slash commands inside the CLI.
- the **plugin catalog** at [10x.in/plugins](https://10x.in/plugins) — extend the CLI with custom tools (CRM connectors, internal-doc readers, deploy helpers) maintained by your team or the community.
- usage analytics, team seats, and SSO at [10x.in/team](https://10x.in/team).

The CLI will prompt for your email, send a 6-digit code, you paste it back, and you're signed in. No browser, no copy-paste of long keys.

```bash
openanalyst account status   # see plan + credits left
openanalyst account logout   # sign out
```

**2. Paste an OpenAnalyst API key.** Already have an `sk-oa-v1-*` key from [10x.in](https://10x.in) → Dashboard → API Keys?

```bash
export OPENANALYST_API_KEY=sk-oa-v1-...
openanalyst
```

…or set it once in `~/.openanalyst/.env` and forget it.

**3. Bring your own provider keys (BYOP).** Prefer to talk to Anthropic, OpenAI, Gemini, xAI, OpenRouter, or Bedrock directly with your own keys? Totally supported — drop them in `~/.openanalyst/.env` or paste them into the web console's Settings → Providers tab. The CLI auto-discovers what's available and shows only the models you can actually use. You can mix BYOP with an OpenAnalyst login; both work side-by-side and you pick which one inference uses per-session.

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

OpenAnalyst CLI routes to any of these — pick whichever you want, mix and match:

- **OpenAnalyst (recommended)** — sign in with `openanalyst account login` or paste an `sk-oa-v1-*` key from [10x.in](https://10x.in). One credit balance, every frontier model: Beta, Max, GPT-5.4, Claude Opus 4.7, Gemini 3 Pro. Free credits on sign-up.
- **Anthropic / Claude** — `ANTHROPIC_API_KEY` (Claude Opus, Sonnet, Haiku).
- **OpenAI / GPT / Codex** — `OPENAI_API_KEY`.
- **Google Gemini** — `GEMINI_API_KEY`.
- **xAI / Grok** — `XAI_API_KEY`.
- **OpenRouter** — `OPENROUTER_API_KEY` (350+ models behind one key).
- **Amazon Bedrock** — `BEDROCK_API_KEY` (Bedrock-hosted Claude).
- **Stability AI / Gemini Imagen / OpenAI Images** — image generation via `/image`.
- **Brave Search / Tavily / Serper / Exa** — web search for `/scrape` and the WebSearch tool.

Set any of these as environment variables, drop them in `~/.openanalyst/.env`, or paste them in the web console's Settings → Providers tab. They're stored encrypted on disk.

## Skills & plugins (10x.in)

Beyond the model providers, the CLI installs **skills** (typed slash commands) and **plugins** (custom tools) from the [10x.in](https://10x.in) catalog. Signed-in users get access to:

- **[10x.in/skills](https://10x.in/skills)** — curated skills like `/code-review`, `/release-notes`, `/security-audit`, `/standup`, `/onboarding`. Install with `openanalyst skills install <name>` and they show up in the slash-command palette instantly.
- **[10x.in/plugins](https://10x.in/plugins)** — first-party + community plugins: CRM connectors, internal-doc readers, deploy helpers, custom tool servers. `openanalyst extension install <plugin>` to add one.
- **Skill authoring** — write your own skill at [10x.in/skills/new](https://10x.in/skills/new), share it with your team (private) or the world (public). The `/skills` directory inside the CLI honors both local and remote sources.

The skills & plugins index is fetched at startup using your account credentials, so they "just work" once you've run `openanalyst account login`.

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
