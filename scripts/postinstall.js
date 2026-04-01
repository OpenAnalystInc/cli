#!/usr/bin/env node

const { execSync } = require("child_process");
const path = require("path");
const fs = require("fs");
const os = require("os");
const https = require("https");

const PACKAGE_VERSION = require("../package.json").version;
const REPO = "AnitChaudhry/openanalyst-cli";
const NATIVE_DIR = path.join(__dirname, "..", "native");

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

function log(msg) {
  console.log(`  [openanalyst] ${msg}`);
}

// Try to download a prebuilt binary from GitHub releases
async function tryDownloadPrebuilt() {
  const platformKey = getPlatformKey();
  const target = PLATFORM_MAP[platformKey];

  if (!target) {
    log(`No prebuilt binary for ${platformKey}, will build from source`);
    return false;
  }

  const ext = os.platform() === "win32" ? ".exe" : "";
  const assetName = `openanalyst-${target}${ext}`;
  const releaseUrl = `https://github.com/${REPO}/releases/download/v${PACKAGE_VERSION}/${assetName}`;

  log(`Checking for prebuilt binary: ${assetName}`);

  return new Promise((resolve) => {
    const request = https.get(releaseUrl, { headers: { "User-Agent": "openanalyst-cli" } }, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        // Follow redirect (GitHub releases redirect to S3)
        https.get(res.headers.location, { headers: { "User-Agent": "openanalyst-cli" } }, (redirectRes) => {
          if (redirectRes.statusCode !== 200) {
            resolve(false);
            return;
          }
          downloadStream(redirectRes, assetName, ext, resolve);
        }).on("error", () => resolve(false));
        return;
      }
      if (res.statusCode !== 200) {
        resolve(false);
        return;
      }
      downloadStream(res, assetName, ext, resolve);
    });
    request.on("error", () => resolve(false));
    request.setTimeout(15000, () => { request.destroy(); resolve(false); });
  });
}

function downloadStream(stream, assetName, ext, resolve) {
  const chunks = [];
  stream.on("data", (chunk) => chunks.push(chunk));
  stream.on("end", () => {
    try {
      fs.mkdirSync(NATIVE_DIR, { recursive: true });
      const outPath = path.join(NATIVE_DIR, `openanalyst${ext}`);
      fs.writeFileSync(outPath, Buffer.concat(chunks));
      if (os.platform() !== "win32") {
        fs.chmodSync(outPath, 0o755);
      }
      log(`Downloaded prebuilt binary to ${outPath}`);
      resolve(true);
    } catch (err) {
      log(`Failed to write binary: ${err.message}`);
      resolve(false);
    }
  });
  stream.on("error", () => resolve(false));
}

// Build from source using cargo
function buildFromSource() {
  const rustDir = path.join(__dirname, "..", "rust");
  const cargoToml = path.join(rustDir, "Cargo.toml");

  if (!fs.existsSync(cargoToml)) {
    log("Rust source not found — skipping build");
    log("Download a prebuilt release from GitHub or install Rust to build");
    return false;
  }

  // Check cargo
  try {
    execSync("cargo --version", { stdio: "pipe" });
  } catch {
    log("Rust/Cargo not installed — cannot build from source");
    log("Install Rust: https://rustup.rs");
    return false;
  }

  log("Building from source (this may take a few minutes)...");
  try {
    execSync("cargo build --release -p openanalyst-cli", {
      cwd: rustDir,
      stdio: "inherit",
      timeout: 600000, // 10 minute timeout
    });

    // Copy binary to native/
    const ext = os.platform() === "win32" ? ".exe" : "";
    const srcBin = path.join(rustDir, "target", "release", `openanalyst${ext}`);
    if (fs.existsSync(srcBin)) {
      fs.mkdirSync(NATIVE_DIR, { recursive: true });
      fs.copyFileSync(srcBin, path.join(NATIVE_DIR, `openanalyst${ext}`));
      if (os.platform() !== "win32") {
        fs.chmodSync(path.join(NATIVE_DIR, `openanalyst${ext}`), 0o755);
      }
      log("Build complete");
      return true;
    }
  } catch (err) {
    log(`Build failed: ${err.message}`);
  }
  return false;
}

async function main() {
  console.log("");
  log("Installing OpenAnalyst CLI v" + PACKAGE_VERSION);
  console.log("");

  // Strategy 1: Try prebuilt download
  const downloaded = await tryDownloadPrebuilt();
  if (downloaded) {
    log("Installation complete (prebuilt binary)");
    console.log("");
    return;
  }

  // Strategy 2: Build from source
  log("No prebuilt binary available, building from source...");
  const built = buildFromSource();
  if (built) {
    log("Installation complete (built from source)");
    console.log("");
    return;
  }

  // Neither worked
  console.log("");
  log("Could not install the OpenAnalyst CLI binary.");
  log("Options:");
  log("  1. Install Rust (https://rustup.rs) and run: npm run build");
  log("  2. Download a release binary from GitHub");
  console.log("");
}

main().catch((err) => {
  console.error(`  [openanalyst] postinstall error: ${err.message}`);
  // Don't fail the npm install — user can build manually
});
