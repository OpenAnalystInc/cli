#!/usr/bin/env node

const { execFileSync, spawn } = require("child_process");
const path = require("path");
const fs = require("fs");
const os = require("os");

// Resolve the native binary path
function getBinaryPath() {
  const platform = os.platform();
  const ext = platform === "win32" ? ".exe" : "";
  const binaryName = `openanalyst${ext}`;

  // Check multiple locations in priority order
  const candidates = [
    // Installed via postinstall (prebuilt binary)
    path.join(__dirname, "..", "native", binaryName),
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
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

const binary = getBinaryPath();

if (!binary) {
  console.error(`
  OpenAnalyst CLI binary not found.

  Try reinstalling:
    npm install -g @openanalyst/openanalyst-cli

  Or download from: https://openanalyst.com
  Support: support@openanalyst.com
  `);
  process.exit(1);
}

// Pass through all arguments to the native binary
const args = process.argv.slice(2);
const child = spawn(binary, args, {
  stdio: "inherit",
  env: process.env,
});

child.on("error", (err) => {
  console.error(`Failed to start OpenAnalyst CLI: ${err.message}`);
  process.exit(1);
});

child.on("exit", (code) => {
  process.exit(code ?? 0);
});
