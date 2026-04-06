#!/usr/bin/env node

/**
 * OpenAnalyst CLI — Branded Postinstall Script
 *
 * Features:
 *   - Orange OA ASCII art logo with ANSI color codes
 *   - Braille spinner animation during download
 *   - Progress reporting (downloading, extracting, setting permissions)
 *   - Box-drawing success UI with getting-started tips
 *   - Creates ~/.openanalyst/ directory and default .env template
 *   - Clear error messages on failure
 *
 * CommonJS only — runs during npm install before the project is built.
 */

const { execSync } = require("child_process");
const path = require("path");
const fs = require("fs");
const os = require("os");
const https = require("https");

// ---------------------------------------------------------------------------
// Version and platform
// ---------------------------------------------------------------------------

let PACKAGE_VERSION = "0.0.0";
try {
  PACKAGE_VERSION = require("../package.json").version;
} catch {
  // Fallback if package.json isn't accessible
}

const REPO = "OpenAnalystInc/cli";
const NATIVE_DIR = path.join(__dirname, "..", "native");
const CONFIG_DIR = path.join(os.homedir(), ".openanalyst");
const ENV_FILE = path.join(CONFIG_DIR, ".env");

const PLATFORM_MAP = {
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "win32-x64": "x86_64-pc-windows-msvc",
};

function getPlatformKey() {
  return `${os.platform()}-${os.arch()}`;
}

// ---------------------------------------------------------------------------
// ANSI color helpers (no dependencies — runs before node_modules may exist)
// ---------------------------------------------------------------------------

const ESC = "\x1b[";
const RESET = `${ESC}0m`;
const BOLD = `${ESC}1m`;
const DIM = `${ESC}2m`;

// Colors
const ORANGE = `${ESC}38;2;255;140;0m`;   // OA brand orange
const BLUE = `${ESC}38;2;100;149;237m`;    // OA brand blue (cornflower)
const GREEN = `${ESC}38;2;80;200;120m`;    // Success green
const RED = `${ESC}38;2;220;80;80m`;       // Error red
const CYAN = `${ESC}38;2;100;200;220m`;    // Accent cyan
const WHITE = `${ESC}97m`;
const GRAY = `${ESC}90m`;

// Box-drawing characters
const TL = "\u256D"; // rounded top-left
const TR = "\u256E"; // rounded top-right
const BL = "\u2570"; // rounded bottom-left
const BR = "\u256F"; // rounded bottom-right
const H  = "\u2500"; // horizontal
const V  = "\u2502"; // vertical

// ---------------------------------------------------------------------------
// Spinner (braille cycle — matches the TUI style)
// ---------------------------------------------------------------------------

const SPINNER_FRAMES = ["\u2801", "\u2809", "\u2819", "\u281B", "\u281E", "\u2816", "\u2826", "\u2834", "\u2838", "\u2830", "\u2820", "\u2800"];
let spinnerInterval = null;
let spinnerFrame = 0;

function startSpinner(message) {
  spinnerFrame = 0;
  process.stdout.write(`  ${CYAN}${SPINNER_FRAMES[0]}${RESET} ${GRAY}${message}${RESET}`);
  spinnerInterval = setInterval(() => {
    spinnerFrame = (spinnerFrame + 1) % SPINNER_FRAMES.length;
    // Move to start of line, clear, and rewrite
    process.stdout.write(`\r  ${CYAN}${SPINNER_FRAMES[spinnerFrame]}${RESET} ${GRAY}${message}${RESET}`);
  }, 80);
}

function stopSpinner(success, message) {
  if (spinnerInterval) {
    clearInterval(spinnerInterval);
    spinnerInterval = null;
  }
  const icon = success ? `${GREEN}\u2713${RESET}` : `${RED}\u2717${RESET}`;
  const color = success ? GRAY : RED;
  process.stdout.write(`\r  ${icon} ${color}${message}${RESET}\n`);
}

// ---------------------------------------------------------------------------
// ASCII art logo
// ---------------------------------------------------------------------------

function printLogo() {
  const lines = [
    `  ${ORANGE}${BOLD}\u2588\u2588\u2588\u2588\u2588\u2588\u2557  \u2588\u2588\u2588\u2588\u2557${RESET}         ${WHITE}${BOLD}OpenAnalyst CLI${RESET}`,
    `  ${ORANGE}\u2588\u2588\u2554\u2550\u2550\u2550\u2588\u2588\u2557\u2588\u2588\u2554\u2550\u2550\u2588\u2588\u2557${RESET}        ${GRAY}v${PACKAGE_VERSION}${RESET}`,
    `  ${ORANGE}\u2588\u2588\u2551   \u2588\u2588\u2551\u2588\u2588\u2588\u2588\u2588\u2588\u2588\u2551${RESET}`,
    `  ${ORANGE}\u2588\u2588\u2551   \u2588\u2588\u2551\u2588\u2588\u2554\u2550\u2550\u2588\u2588\u2551${RESET}        ${GRAY}Installing...${RESET}`,
    `  ${ORANGE}\u255A\u2588\u2588\u2588\u2588\u2588\u2588\u2554\u255D\u2588\u2588\u2551  \u2588\u2588\u2551${RESET}`,
    `  ${ORANGE} \u255A\u2550\u2550\u2550\u2550\u2550\u255D \u255A\u2550\u255D  \u255A\u2550\u255D${RESET}`,
  ];

  const boxW = 52;
  console.log("");
  console.log(`${BLUE}${TL}${H.repeat(boxW)}${TR}${RESET}`);
  console.log(`${BLUE}${V}${RESET}${" ".repeat(boxW)}${BLUE}${V}${RESET}`);
  for (const line of lines) {
    // Pad to boxW (accounting for ANSI codes — just pad generously)
    console.log(`${BLUE}${V}${RESET}${line}`);
  }
  console.log(`${BLUE}${V}${RESET}${" ".repeat(boxW)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${BL}${H.repeat(boxW)}${BR}${RESET}`);
  console.log("");
}

// ---------------------------------------------------------------------------
// Success box
// ---------------------------------------------------------------------------

function printSuccessBox(binaryPath) {
  const boxW = 52;
  const pad = (text, len) => {
    // Strip ANSI for length calculation
    const stripped = text.replace(/\x1b\[[0-9;]*m/g, "");
    const padding = Math.max(0, len - stripped.length);
    return text + " ".repeat(padding);
  };

  console.log("");
  console.log(`${BLUE}${TL}${H.repeat(boxW)}${TR}${RESET}`);
  console.log(`${BLUE}${V}${RESET}${" ".repeat(boxW)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}  ${GREEN}${BOLD}\u2713 Installation complete!${RESET}${" ".repeat(boxW - 27)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}${" ".repeat(boxW)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}  ${WHITE}${BOLD}Get started:${RESET}${" ".repeat(boxW - 14)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}    ${CYAN}$ openanalyst${RESET}${" ".repeat(boxW - 17)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}${" ".repeat(boxW)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}  ${WHITE}${BOLD}First time? Login with any AI provider:${RESET}${" ".repeat(boxW - 41)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}    ${CYAN}/login openai sk-abc123...${RESET}${" ".repeat(boxW - 30)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}    ${CYAN}/login anthropic sk-ant-abc...${RESET}${" ".repeat(boxW - 34)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}    ${CYAN}/login gemini AIza...${RESET}${" ".repeat(boxW - 25)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}${" ".repeat(boxW)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}  ${WHITE}${BOLD}Or set keys in:${RESET} ${GRAY}~/.openanalyst/.env${RESET}${" ".repeat(boxW - 37)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}${" ".repeat(boxW)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}  ${WHITE}${BOLD}Shortcuts:${RESET}${" ".repeat(boxW - 12)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}    ${GRAY}\u2022 ${CYAN}Ctrl+E${GRAY} toggle sidebar${RESET}${" ".repeat(boxW - 27)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}    ${GRAY}\u2022 ${CYAN}Ctrl+P${GRAY} permission mode${RESET}${" ".repeat(boxW - 28)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}    ${GRAY}\u2022 ${CYAN}/help${GRAY}  all commands${RESET}${" ".repeat(boxW - 25)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}${" ".repeat(boxW)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}  ${GRAY}Docs:    ${CYAN}https://openanalyst.com/docs${RESET}${" ".repeat(boxW - 39)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}  ${GRAY}Support: ${CYAN}support@openanalyst.com${RESET}${" ".repeat(boxW - 35)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${V}${RESET}${" ".repeat(boxW)}${BLUE}${V}${RESET}`);
  console.log(`${BLUE}${BL}${H.repeat(boxW)}${BR}${RESET}`);
  console.log("");
}

// ---------------------------------------------------------------------------
// Error box
// ---------------------------------------------------------------------------

function printErrorBox(errorMsg) {
  const boxW = 52;
  console.log("");
  console.log(`${RED}${TL}${H.repeat(boxW)}${TR}${RESET}`);
  console.log(`${RED}${V}${RESET}${" ".repeat(boxW)}${RED}${V}${RESET}`);
  console.log(`${RED}${V}${RESET}  ${RED}${BOLD}\u2717 Installation failed${RESET}${" ".repeat(boxW - 24)}${RED}${V}${RESET}`);
  console.log(`${RED}${V}${RESET}${" ".repeat(boxW)}${RED}${V}${RESET}`);

  // Word-wrap error message
  const maxLineW = boxW - 4;
  const words = errorMsg.split(" ");
  let currentLine = "";
  for (const word of words) {
    if (currentLine.length + word.length + 1 > maxLineW) {
      const padding = " ".repeat(Math.max(0, boxW - currentLine.length - 2));
      console.log(`${RED}${V}${RESET}  ${GRAY}${currentLine}${RESET}${padding}${RED}${V}${RESET}`);
      currentLine = word;
    } else {
      currentLine = currentLine ? currentLine + " " + word : word;
    }
  }
  if (currentLine) {
    const padding = " ".repeat(Math.max(0, boxW - currentLine.length - 2));
    console.log(`${RED}${V}${RESET}  ${GRAY}${currentLine}${RESET}${padding}${RED}${V}${RESET}`);
  }

  console.log(`${RED}${V}${RESET}${" ".repeat(boxW)}${RED}${V}${RESET}`);
  console.log(`${RED}${V}${RESET}  ${GRAY}Download manually:${RESET}${" ".repeat(boxW - 20)}${RED}${V}${RESET}`);
  console.log(`${RED}${V}${RESET}    ${CYAN}https://openanalyst.com${RESET}${" ".repeat(boxW - 27)}${RED}${V}${RESET}`);
  console.log(`${RED}${V}${RESET}  ${GRAY}Support: ${CYAN}support@openanalyst.com${RESET}${" ".repeat(boxW - 35)}${RED}${V}${RESET}`);
  console.log(`${RED}${V}${RESET}${" ".repeat(boxW)}${RED}${V}${RESET}`);
  console.log(`${RED}${BL}${H.repeat(boxW)}${BR}${RESET}`);
  console.log("");
}

// ---------------------------------------------------------------------------
// Binary download (same logic as before, with spinner)
// ---------------------------------------------------------------------------

function tryUrl(url, ext) {
  return new Promise((resolve) => {
    const request = https.get(url, { headers: { "User-Agent": "openanalyst-cli" } }, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        // Follow redirect (GitHub releases redirect to S3)
        https.get(res.headers.location, { headers: { "User-Agent": "openanalyst-cli" } }, (redirectRes) => {
          if (redirectRes.statusCode !== 200) {
            resolve(false);
            return;
          }
          downloadStream(redirectRes, ext, resolve);
        }).on("error", () => resolve(false));
        return;
      }
      if (res.statusCode !== 200) {
        resolve(false);
        return;
      }
      downloadStream(res, ext, resolve);
    });
    request.on("error", () => resolve(false));
    request.setTimeout(30000, () => { request.destroy(); resolve(false); });
  });
}

function downloadStream(stream, ext, resolve) {
  const chunks = [];
  let totalBytes = 0;
  stream.on("data", (chunk) => {
    chunks.push(chunk);
    totalBytes += chunk.length;
  });
  stream.on("end", () => {
    try {
      fs.mkdirSync(NATIVE_DIR, { recursive: true });
      const outPath = path.join(NATIVE_DIR, `openanalyst${ext}`);
      fs.writeFileSync(outPath, Buffer.concat(chunks));
      if (os.platform() !== "win32") {
        fs.chmodSync(outPath, 0o755);
      }
      resolve({ success: true, path: outPath, size: totalBytes });
    } catch (err) {
      resolve({ success: false, error: err.message });
    }
  });
  stream.on("error", () => resolve({ success: false, error: "Download stream error" }));
}

async function tryDownloadPrebuilt() {
  const platformKey = getPlatformKey();
  const target = PLATFORM_MAP[platformKey];

  if (!target) {
    return { success: false, error: `No prebuilt binary for ${platformKey}` };
  }

  const ext = os.platform() === "win32" ? ".exe" : "";
  const platformAsset = `openanalyst-${target}${ext}`;
  const genericAsset = `openanalyst${ext}`;
  const releaseUrl = `https://github.com/${REPO}/releases/download/v${PACKAGE_VERSION}/${platformAsset}`;
  const fallbackUrl = `https://github.com/${REPO}/releases/download/v${PACKAGE_VERSION}/${genericAsset}`;

  // Try platform-specific asset first
  let result = await tryUrl(releaseUrl, ext);
  if (result && result.success) return result;

  // Try generic fallback
  result = await tryUrl(fallbackUrl, ext);
  if (result && result.success) return result;

  return { success: false, error: "Binary not found in release assets" };
}

// ---------------------------------------------------------------------------
// Config directory setup
// ---------------------------------------------------------------------------

function setupConfigDir() {
  try {
    // Create main config directory
    fs.mkdirSync(CONFIG_DIR, { recursive: true });

    // Create subdirectories matching the Rust backend's expected structure.
    // Each directory is actively scanned by the corresponding Rust module:
    //   commands/      -> skills.rs:load_commands_from_dir()
    //   skills/        -> skills.rs:load_skills_from_dir()
    //   agents/        -> agents_config.rs:load_agents_from_dir()
    //   rules/         -> rules.rs:load_rules_from_dir()
    //   plugins/       -> plugins/lib.rs (PluginManager)
    //   output-styles/ -> output_styles.rs:load_output_styles()
    //   sessions/      -> session.rs (conversation persistence)
    //   hooks/         -> convention for plugin hook scripts (hooks themselves are configured in settings.json)
    const subdirs = [
      "sessions",        // Chat session persistence
      "credentials",     // Credential storage
      "commands",        // User custom slash commands (*.md files)
      "skills",          // Custom skills (SKILL.md directories)
      "plugins",         // Installed plugins
      "agents",          // Custom agent definitions
      "rules",           // Global rules (*.md with optional paths frontmatter)
      "hooks",           // Plugin hook scripts (hooks configured in settings.json)
      "output-styles",   // Custom output formatting styles
      "todos",           // Global task tracking (TodoWrite tool persists here)
    ];
    for (const sub of subdirs) {
      fs.mkdirSync(path.join(CONFIG_DIR, sub), { recursive: true });
    }

    let created = false;

    // Create OPENANALYST.md — global instructions file (mirrors ~/.claude/CLAUDE.md)
    const oaMdFile = path.join(CONFIG_DIR, "OPENANALYST.md");
    if (!fs.existsSync(oaMdFile)) {
      const oaMdTemplate = [
        "# Global Instructions for OpenAnalyst CLI",
        "",
        "This file provides instructions that apply to ALL projects when using",
        "OpenAnalyst CLI. It mirrors Claude Code's ~/.claude/CLAUDE.md file.",
        "",
        "## How it works",
        "",
        "- This file is loaded at the start of every OpenAnalyst session.",
        "- Project-level instructions (.openanalyst/OPENANALYST.md) take priority.",
        "- You can add coding conventions, preferred tools, or personal preferences.",
        "",
        "## Example instructions",
        "",
        "```",
        "- Always use TypeScript strict mode.",
        "- Prefer functional components with hooks over class components.",
        "- Use descriptive variable names; avoid abbreviations.",
        "- Run tests before committing.",
        "```",
        "",
        "## Tips",
        "",
        "- Keep this file concise — it is loaded into context every session.",
        "- Put project-specific rules in the project's OPENANALYST.md instead.",
        "- Custom slash commands go in ~/.openanalyst/commands/ as .md files.",
        "- Custom rules go in ~/.openanalyst/rules/ as .md files.",
        "",
      ].join("\n");
      fs.writeFileSync(oaMdFile, oaMdTemplate, "utf-8");
      created = true;
    }

    // Create settings.json — global settings (mirrors ~/.claude/settings.json)
    const settingsFile = path.join(CONFIG_DIR, "settings.json");
    if (!fs.existsSync(settingsFile)) {
      const settingsTemplate = {
        "$schema": "https://openanalyst.com/schemas/settings.json",
        "_comment": "Global settings for OpenAnalyst CLI. Project-level settings (.openanalyst/settings.json) override these.",
        "model": null,
        "effort": null,
        "theme": null,
        "defaultMode": null,
        "autoCompact": true,
        "includeCoAuthoredBy": true,
        "autoMemoryEnabled": true,
        "viModeEnabled": false,
        "fastModeEnabled": false,
        "permissionRules": {
          "allow": [],
          "ask": [],
          "deny": []
        },
        "mcpServers": {},
        "hooks": {},
        "env": {}
      };
      fs.writeFileSync(settingsFile, JSON.stringify(settingsTemplate, null, 2) + "\n", "utf-8");
      created = true;
    }

    // Create default .env template
    if (!fs.existsSync(ENV_FILE)) {
      const template = [
        "# ╔══════════════════════════════════════════════════════╗",
        "# ║          OpenAnalyst CLI Configuration               ║",
        "# ╚══════════════════════════════════════════════════════╝",
        "#",
        "# Add your API key for ANY ONE provider to get started.",
        "# Use /login inside the CLI for easier setup.",
        "",
        "# ── AI Providers (set at least one) ──",
        "# OPENAI_API_KEY=sk-your-key-here",
        "# ANTHROPIC_API_KEY=sk-ant-your-key-here",
        "# GEMINI_API_KEY=AIza-your-key-here",
        "# XAI_API_KEY=xai-your-key-here",
        "# OPENROUTER_API_KEY=sk-or-your-key-here",
        "",
        "# ── OpenAnalyst Platform (optional) ──",
        "# OPENANALYST_AUTH_TOKEN=sk-oa-your-key-here",
        "",
        "# ── Default Model (optional — auto-detected from provider) ──",
        "# OPENANALYST_MODEL=claude-sonnet-4-20250514",
        "",
        "# ── Sandbox Mode (recommended: true) ──",
        "OPENANALYST_SANDBOX=true",
        "",
        "# ── Voice Input (requires OpenAI key for Whisper) ──",
        "# OPENANALYST_VOICE_ENABLED=true",
        "",
      ].join("\n");
      fs.writeFileSync(ENV_FILE, template, "utf-8");
      created = true;
    }

    // Create credentials.json template
    const credFile = path.join(CONFIG_DIR, "credentials.json");
    if (!fs.existsSync(credFile)) {
      fs.writeFileSync(credFile, JSON.stringify({
        version: 1,
        providers: {},
        note: "Managed by OpenAnalyst CLI. Use /login to add credentials.",
      }, null, 2), "utf-8");
      created = true;
    }

    // Create preferences.json template
    const prefsFile = path.join(CONFIG_DIR, "preferences.json");
    if (!fs.existsSync(prefsFile)) {
      fs.writeFileSync(prefsFile, JSON.stringify({
        defaultProvider: null,
        defaultSetAt: null,
        routing: {},
        theme: "dark",
        vimMode: false,
        sidebarVisible: false,
      }, null, 2), "utf-8");
      created = true;
    }

    // Create trusted_folders.json (sandbox allowlist)
    // Must match the filename used by folder_trust.rs in the Rust backend.
    const trustFile = path.join(CONFIG_DIR, "trusted_folders.json");
    if (!fs.existsSync(trustFile)) {
      fs.writeFileSync(trustFile, JSON.stringify(
        [],
        null, 2
      ) + "\n", "utf-8");
      created = true;
    }

    return { created };
  } catch (err) {
    return { error: err.message };
  }
}

// ---------------------------------------------------------------------------
// Format bytes
// ---------------------------------------------------------------------------

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  // Print branded logo
  printLogo();

  // Step 1: Setup config directory + skeleton files
  startSpinner("Setting up config directory...");
  const configResult = setupConfigDir();
  if (configResult.error) {
    stopSpinner(false, `Config setup failed: ${configResult.error}`);
  } else if (configResult.created) {
    stopSpinner(true, `Created ${CYAN}~/.openanalyst/${RESET} ${GRAY}with OPENANALYST.md, settings.json, .env, credentials.json${RESET}`);
  } else {
    stopSpinner(true, `Config directory exists ${GRAY}~/.openanalyst/${RESET}`);
  }

  // Step 1.5: Install Playwright browser (for web tools)
  startSpinner("Setting up Playwright browser...");
  try {
    execSync("npx playwright install chromium --with-deps 2>&1", {
      timeout: 120000,
      stdio: "pipe",
    });
    stopSpinner(true, "Playwright Chromium installed (web browsing tools ready)");
  } catch {
    stopSpinner(false, "Playwright install skipped (optional — web tools won't be available)");
  }

  // Step 2: Download prebuilt binary
  const platformKey = getPlatformKey();
  const platformLabel = `${os.platform()}-${os.arch()}`;
  startSpinner(`Downloading prebuilt binary for ${platformLabel}...`);

  const downloadResult = await tryDownloadPrebuilt();

  if (downloadResult.success) {
    const sizeStr = downloadResult.size ? ` (${formatBytes(downloadResult.size)})` : "";
    stopSpinner(true, `Binary downloaded${sizeStr}`);

    // Show install path
    const installPath = downloadResult.path || path.join(NATIVE_DIR, "openanalyst" + (os.platform() === "win32" ? ".exe" : ""));
    console.log(`  ${GREEN}\u2713${RESET} ${GRAY}Installed to ${CYAN}${installPath}${RESET}`);

    // Step 3: Print success box
    printSuccessBox(installPath);
  } else {
    stopSpinner(false, `Could not download binary: ${downloadResult.error || "unknown error"}`);
    printErrorBox(downloadResult.error || "Could not download the prebuilt binary for your platform.");
  }
}

main().catch((err) => {
  // Don't fail the npm install — user can download manually
  console.error(`\n  ${RED}\u2717${RESET} ${GRAY}postinstall error: ${err.message}${RESET}\n`);
});
