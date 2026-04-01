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
    // Installed via postinstall (prebuilt or cargo-built)
    path.join(__dirname, "..", "native", binaryName),
    // Local development build
    path.join(__dirname, "..", "rust", "target", "release", binaryName),
    path.join(__dirname, "..", "rust", "target", "debug", binaryName),
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
    npm install -g @openanalyst/cli

  Or build from source:
    cd rust && cargo build --release

  Requires Rust toolchain: https://rustup.rs
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
