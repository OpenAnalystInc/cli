#!/usr/bin/env node

const { execSync } = require("child_process");
const path = require("path");
const fs = require("fs");
const os = require("os");

const rustDir = path.join(__dirname, "..", "rust");
const nativeDir = path.join(__dirname, "..", "native");

console.log("");
console.log("  Building OpenAnalyst CLI from source...");
console.log("");

try {
  execSync("cargo --version", { stdio: "pipe" });
} catch {
  console.error("  Error: Rust/Cargo not installed.");
  console.error("  Install from: https://rustup.rs");
  process.exit(1);
}

try {
  execSync("cargo build --release -p openanalyst-cli", {
    cwd: rustDir,
    stdio: "inherit",
  });

  const ext = os.platform() === "win32" ? ".exe" : "";
  const srcBin = path.join(rustDir, "target", "release", `openanalyst${ext}`);
  fs.mkdirSync(nativeDir, { recursive: true });
  fs.copyFileSync(srcBin, path.join(nativeDir, `openanalyst${ext}`));
  if (os.platform() !== "win32") {
    fs.chmodSync(path.join(nativeDir, `openanalyst${ext}`), 0o755);
  }

  console.log("");
  console.log("  Build complete: native/openanalyst" + ext);
  console.log("  Run with: npx openanalyst");
  console.log("");
} catch (err) {
  console.error(`  Build failed: ${err.message}`);
  process.exit(1);
}
