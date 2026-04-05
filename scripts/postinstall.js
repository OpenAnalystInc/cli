#!/usr/bin/env node

const { execSync } = require("child_process");
const path = require("path");
const fs = require("fs");
const os = require("os");
const https = require("https");

const PACKAGE_VERSION = require("../package.json").version;
const REPO = "OpenAnalystInc/cli";
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
  // Try platform-specific name first, then generic name
  const platformAsset = `openanalyst-${target}${ext}`;
  const genericAsset = `openanalyst${ext}`;
  const assetName = platformAsset;
  const releaseUrl = `https://github.com/${REPO}/releases/download/v${PACKAGE_VERSION}/${assetName}`;
  const fallbackUrl = `https://github.com/${REPO}/releases/download/v${PACKAGE_VERSION}/${genericAsset}`;

  log(`Checking for prebuilt binary: ${assetName}`);

  // Try platform-specific asset first, then generic fallback
  const downloaded = await tryUrl(releaseUrl, ext);
  if (downloaded) return true;

  log(`Platform-specific binary not found, trying generic: ${genericAsset}`);
  return tryUrl(fallbackUrl, ext);
}

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
          downloadStream(redirectRes, path.basename(url), ext, resolve);
        }).on("error", () => resolve(false));
        return;
      }
      if (res.statusCode !== 200) {
        resolve(false);
        return;
      }
      downloadStream(res, path.basename(url), ext, resolve);
    });
    request.on("error", () => resolve(false));
    request.setTimeout(30000, () => { request.destroy(); resolve(false); });
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

  // No prebuilt available
  console.log("");
  log("Could not install the OpenAnalyst CLI binary.");
  log("Download from: https://openanalyst.com");
  log("Support: support@openanalyst.com");
  console.log("");
}

main().catch((err) => {
  console.error(`  [openanalyst] postinstall error: ${err.message}`);
  // Don't fail the npm install — user can build manually
});
