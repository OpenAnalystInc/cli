#!/usr/bin/env node

/**
 * OpenAnalyst CLI — Entry Point
 *
 * Architecture:
 *   bin/cli.js  -->  Ink TUI (Node.js React)  -->  Rust engine (--headless mode)
 *
 * This launcher:
 *   1. Finds the native Rust engine binary (native/ dir or system PATH)
 *   2. Launches the Ink TUI with OA_ENGINE_PATH env var
 *   3. Falls back to --mock mode if no binary found (TUI still works)
 *   4. Supports --headless flag to bypass TUI and run Rust engine directly
 */

const { spawn, execFileSync } = require("child_process");
const path = require("path");
const fs = require("fs");
const os = require("os");

// ---------------------------------------------------------------------------
// Find the native Rust binary
// ---------------------------------------------------------------------------

function getBinaryPath() {
  const platform = os.platform();
  const ext = platform === "win32" ? ".exe" : "";
  const binaryName = `openanalyst${ext}`;

  const candidates = [
    // Installed via postinstall (prebuilt binary)
    path.join(__dirname, "..", "native", binaryName),
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate;
  }

  // Try system PATH
  try {
    const which = platform === "win32" ? "where" : "which";
    const result = execFileSync(which, ["openanalyst"], {
      encoding: "utf8",
      stdio: ["pipe", "pipe", "pipe"],
    }).trim();
    if (result) return result.split("\n")[0].trim();
  } catch {
    // not in PATH
  }

  return null;
}

// ---------------------------------------------------------------------------
// Find the Ink TUI entry point
// ---------------------------------------------------------------------------

function getTuiEntryPoint() {
  const candidates = [
    path.join(__dirname, "..", "ink-tui", "dist", "index.js"),
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const binaryPath = getBinaryPath();
const tuiEntry = getTuiEntryPoint();
const args = process.argv.slice(2);

// --headless: bypass TUI, pass directly to Rust engine
if (args.includes("--headless")) {
  if (!binaryPath) {
    console.error(
      "OpenAnalyst engine binary not found.\n" +
      "Install with: npm install -g @openanalystinc/openanalyst-cli"
    );
    process.exit(1);
  }
  const child = spawn(binaryPath, args, { stdio: "inherit", env: process.env });
  child.on("exit", (code) => process.exit(code ?? 0));
  child.on("error", (err) => {
    console.error(`Engine error: ${err.message}`);
    process.exit(1);
  });
  return;
}

// --version / -v: print version and exit
if (args.includes("--version") || args.includes("-v")) {
  const pkg = require("../package.json");
  console.log(`OpenAnalyst CLI v${pkg.version}`);
  process.exit(0);
}

// --help / -h: show basic usage
if (args.includes("--help") || args.includes("-h")) {
  const pkg = require("../package.json");
  console.log(`
OpenAnalyst CLI v${pkg.version}
The Universal AI Agent for Your Terminal

Usage:
  openanalyst              Launch the interactive TUI
  openanalyst --headless   Run the engine in headless mode (no TUI)
  openanalyst --version    Show version
  openanalyst --help       Show this help

Docs:    https://openanalyst.com/docs
Support: support@openanalyst.com
`);
  process.exit(0);
}

// Launch the Ink TUI
if (!tuiEntry) {
  console.error(
    "OpenAnalyst TUI not found.\n" +
    "Reinstall with: npm install -g @openanalystinc/openanalyst-cli"
  );
  process.exit(1);
}

const env = {
  ...process.env,
  OA_ENGINE_PATH: binaryPath || "",
  OA_MOCK: binaryPath ? "0" : "1",
};

const child = spawn(process.execPath, [tuiEntry, ...args], {
  stdio: "inherit",
  env,
  cwd: process.cwd(),
});

child.on("error", (err) => {
  console.error(`Failed to start OpenAnalyst TUI: ${err.message}`);
  process.exit(1);
});

child.on("exit", (code) => {
  process.exit(code ?? 0);
});
