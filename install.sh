#!/usr/bin/env bash
set -e

# ─────────────────────────────────────────────────
# OpenAnalyst CLI Installer — macOS / Linux
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.sh | bash
#
# Or with a specific version:
#   curl -fsSL ... | bash -s -- --version 1.0.1
# ─────────────────────────────────────────────────

REPO="AnitChaudhry/openanalyst-cli"
BINARY_NAME="openanalyst"
INSTALL_DIR="${OPENANALYST_INSTALL_DIR:-$HOME/.local/bin}"
CONFIG_DIR="$HOME/.openanalyst"
VERSION="${OPENANALYST_VERSION:-latest}"

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --dir) INSTALL_DIR="$2"; shift 2 ;;
    *) shift ;;
  esac
done

# Colors
CYAN='\033[38;5;45m'
GREEN='\033[38;5;46m'
BLUE='\033[38;5;39m'
DIM='\033[2m'
BOLD='\033[1m'
RESET='\033[0m'

echo ""
echo -e "${BLUE}  ######   #####${RESET}"
echo -e "${BLUE} ##    ## ${CYAN}##   ##${RESET}"
echo -e "${BLUE} ##    ## ${CYAN}#######${RESET}"
echo -e "${BLUE} ##    ## ${CYAN}##   ##${RESET}"
echo -e "${BLUE}  ######  ${CYAN}##   ##${RESET}"
echo ""
echo -e " ${BOLD}OpenAnalyst CLI Installer${RESET}"
echo -e " ${DIM}─────────────────────────${RESET}"
echo ""

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)   PLATFORM="unknown-linux-gnu" ;;
  Darwin)  PLATFORM="apple-darwin" ;;
  *)       echo " Error: Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  TARGET="x86_64-${PLATFORM}" ;;
  aarch64|arm64)  TARGET="aarch64-${PLATFORM}" ;;
  *)              echo " Error: Unsupported arch: $ARCH"; exit 1 ;;
esac

echo -e " ${DIM}Platform:${RESET}  ${OS} ${ARCH}"
echo -e " ${DIM}Target:${RESET}    ${TARGET}"
echo -e " ${DIM}Install:${RESET}   ${INSTALL_DIR}"
echo ""

# ── Step 1: Download or build ──

# Resolve version
if [ "$VERSION" = "latest" ]; then
  echo -e " ${DIM}[1/4] Fetching latest version...${RESET}"
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
    | grep '"tag_name"' | head -1 | sed 's/.*"v\(.*\)".*/\1/' || echo "")
  if [ -z "$VERSION" ]; then
    echo " Could not fetch latest release. Falling back to build from source."
    VERSION=""
  fi
fi

DOWNLOADED=false

# Try to download prebuilt binary
if [ -n "$VERSION" ]; then
  DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/openanalyst-${TARGET}"
  echo -e " ${DIM}[2/4] Downloading v${VERSION} for ${TARGET}...${RESET}"

  mkdir -p "$INSTALL_DIR"
  if curl -fsSL "$DOWNLOAD_URL" -o "${INSTALL_DIR}/${BINARY_NAME}" 2>/dev/null; then
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    DOWNLOADED=true
    echo -e " ${GREEN}✓ Downloaded prebuilt binary${RESET}"
  else
    echo -e " ${DIM}No prebuilt binary available, building from source...${RESET}"
  fi
fi

# Fall back to building from source
if [ "$DOWNLOADED" = false ]; then
  if ! command -v cargo &>/dev/null; then
    echo ""
    echo " Rust is required to build from source."
    echo " Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
  fi

  echo -e " ${DIM}[2/4] Building from source (this takes a few minutes)...${RESET}"
  TMPDIR=$(mktemp -d)
  git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR/openanalyst-cli" 2>/dev/null
  cd "$TMPDIR/openanalyst-cli/rust"
  cargo build --release -p openanalyst-cli --quiet 2>&1
  mkdir -p "$INSTALL_DIR"
  cp "target/release/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
  rm -rf "$TMPDIR"
  echo -e " ${GREEN}✓ Built from source${RESET}"
fi

# ── Step 2: Add to PATH ──
echo -e " ${DIM}[3/4] Configuring PATH...${RESET}"
SHELL_RC=""
if [ -f "$HOME/.zshrc" ]; then SHELL_RC="$HOME/.zshrc"
elif [ -f "$HOME/.bashrc" ]; then SHELL_RC="$HOME/.bashrc"
elif [ -f "$HOME/.bash_profile" ]; then SHELL_RC="$HOME/.bash_profile"
fi

if [ -n "$SHELL_RC" ]; then
  if ! grep -q "$INSTALL_DIR" "$SHELL_RC" 2>/dev/null; then
    echo "" >> "$SHELL_RC"
    echo "# OpenAnalyst CLI" >> "$SHELL_RC"
    echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "$SHELL_RC"
  fi
fi

export PATH="${INSTALL_DIR}:$PATH"

# ── Step 3: Create config directory and .env template ──
echo -e " ${DIM}[4/4] Setting up config...${RESET}"
mkdir -p "$CONFIG_DIR"

if [ ! -f "$CONFIG_DIR/.env" ]; then
  cat > "$CONFIG_DIR/.env" << 'ENVEOF'
# ═══════════════════════════════════════════════════════════════════
#  OpenAnalyst CLI — Environment Configuration
# ═══════════════════════════════════════════════════════════════════
#
#  Add your API keys here. The CLI loads this file on startup.
#  Only uncomment and fill in the providers you want to use.
#  You can also use `openanalyst login` for interactive setup.
#
#  Docs: https://github.com/AnitChaudhry/openanalyst-cli
# ═══════════════════════════════════════════════════════════════════

# ── Provider API Keys ─────────────────────────────────────────────

# OpenAnalyst (default provider)
# OPENANALYST_API_KEY=
# OPENANALYST_AUTH_TOKEN=

# Anthropic / Claude (opus, sonnet, haiku)
# ANTHROPIC_API_KEY=sk-ant-...

# OpenAI / Codex (gpt-4o, o3, codex-mini)
# OPENAI_API_KEY=sk-...

# Google Gemini (gemini-2.5-pro, flash)
# GEMINI_API_KEY=AIza...

# xAI / Grok (grok-3, grok-mini)
# XAI_API_KEY=xai-...

# OpenRouter (350+ models via one key)
# OPENROUTER_API_KEY=sk-or-...

# Amazon Bedrock
# BEDROCK_API_KEY=

# Stability AI (image generation via /image)
# STABILITY_API_KEY=sk-...

# ── Base URL Overrides (optional) ─────────────────────────────────

# OPENANALYST_BASE_URL=https://api.openanalyst.com/api
# ANTHROPIC_BASE_URL=https://api.anthropic.com
# OPENAI_BASE_URL=https://api.openai.com/v1
# GEMINI_BASE_URL=https://generativelanguage.googleapis.com/v1beta/openai
# XAI_BASE_URL=https://api.x.ai/v1
# OPENROUTER_BASE_URL=https://openrouter.ai/api/v1
# BEDROCK_BASE_URL=https://bedrock-runtime.us-east-1.amazonaws.com/v1

# ── Model Configuration ──────────────────────────────────────────

# OPENANALYST_MODEL=claude-sonnet-4-6
ENVEOF
  echo -e " ${GREEN}✓ Created ${CONFIG_DIR}/.env${RESET}"
else
  echo -e " ${DIM}Config already exists${RESET}"
fi

# ── Done ──
echo ""
echo -e " ${GREEN}${BOLD}✓ Installation complete${RESET}"
echo ""
echo -e " ${DIM}Version:${RESET}   $("${INSTALL_DIR}/${BINARY_NAME}" --version 2>&1 | head -2 | tail -1 | xargs)"
echo -e " ${DIM}Binary:${RESET}    ${INSTALL_DIR}/${BINARY_NAME}"
echo -e " ${DIM}Config:${RESET}    ${CONFIG_DIR}/.env"
echo ""
echo -e " ${BOLD}Get started:${RESET}"
echo ""
echo -e "   ${CYAN}openanalyst login${RESET}              ${DIM}# Interactive login (browser or API key)${RESET}"
echo -e "   ${CYAN}openanalyst${RESET}                    ${DIM}# Start a new session${RESET}"
echo ""
echo -e " ${DIM}Or edit ~/.openanalyst/.env to add your API keys directly.${RESET}"
echo ""
echo -e " ${BOLD}Available providers:${RESET}"
echo -e "   OpenAnalyst ${DIM}(default)${RESET}  •  Anthropic/Claude  •  OpenAI/Codex"
echo -e "   Google Gemini  •  xAI/Grok  •  OpenRouter  •  Amazon Bedrock"
echo ""
if [ -n "$SHELL_RC" ]; then
  echo -e " ${DIM}Reload shell: source ${SHELL_RC}${RESET}"
fi
echo -e " ${DIM}Questions? anit@openanalyst.com${RESET}"
echo ""
