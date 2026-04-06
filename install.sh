#!/usr/bin/env bash
set -e

# ─────────────────────────────────────────────────
# OpenAnalyst CLI Installer — macOS / Linux
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.sh | bash
#
# Or with a specific version:
#   curl -fsSL ... | bash -s -- --version 1.0.89
# ─────────────────────────────────────────────────

REPO="OpenAnalystInc/cli"
BINARY_NAME="openanalyst"
INSTALL_DIR="${OPENANALYST_INSTALL_DIR:-$HOME/.local/bin}"
CONFIG_DIR="$HOME/.openanalyst"
VERSION="${OPENANALYST_VERSION:-latest}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --dir) INSTALL_DIR="$2"; shift 2 ;;
    *) shift ;;
  esac
done

# ── Colors & Symbols ──
C1='\033[38;5;39m'   # Blue
C2='\033[38;5;45m'   # Cyan
C3='\033[38;5;81m'   # Light blue
C4='\033[38;5;46m'   # Green
C5='\033[38;5;208m'  # Orange
DIM='\033[2m'
BOLD='\033[1m'
R='\033[0m'
CHECK="${C4}✓${R}"
ARROW="${C2}›${R}"
DOT="${DIM}·${R}"

clear 2>/dev/null || true
echo ""
echo ""
echo -e "   ${C1}████████  ${C2}  ████   ${R}"
echo -e "   ${C1}██    ██  ${C2} ██  ██  ${R}"
echo -e "   ${C1}██    ██  ${C2}██    ██ ${R}"
echo -e "   ${C1}██    ██  ${C2}████████ ${R}"
echo -e "   ${C1}██    ██  ${C2}██    ██ ${R}"
echo -e "   ${C1}████████  ${C2}██    ██ ${R}"
echo ""
echo -e "   ${BOLD}OpenAnalyst CLI${R}  ${DIM}v${VERSION}${R}"
echo -e "   ${DIM}The Universal AI Agent for Your Terminal${R}"
echo ""
echo -e "   ${DIM}────────────────────────────────────────────${R}"
echo ""

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)   PLATFORM="unknown-linux-gnu"; OS_LABEL="Linux" ;;
  Darwin)  PLATFORM="apple-darwin"; OS_LABEL="macOS" ;;
  *)       echo -e "   ${C5}✗ Unsupported OS: $OS${R}"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  TARGET="x86_64-${PLATFORM}"; ARCH_LABEL="x86_64" ;;
  aarch64|arm64)  TARGET="aarch64-${PLATFORM}"; ARCH_LABEL="ARM64" ;;
  *)              echo -e "   ${C5}✗ Unsupported architecture: $ARCH${R}"; exit 1 ;;
esac

echo -e "   ${DIM}┌──────────────────────────────────────────┐${R}"
echo -e "   ${DIM}│${R}  ${BOLD}System${R}       ${OS_LABEL} ${ARCH_LABEL}                  ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${BOLD}Target${R}       ${TARGET}  ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${BOLD}Install to${R}   ${INSTALL_DIR}               ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${BOLD}Config at${R}    ${CONFIG_DIR}               ${DIM}│${R}"
echo -e "   ${DIM}└──────────────────────────────────────────┘${R}"
echo ""

# ═══════════════════════════════════════════════════
#  Step 1 — Resolve version
# ═══════════════════════════════════════════════════
if [ "$VERSION" = "latest" ]; then
  echo -ne "   ${ARROW} ${DIM}Fetching latest release...${R}"
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
    | grep '"tag_name"' | head -1 | sed 's/.*"v\(.*\)".*/\1/' || echo "")
  if [ -z "$VERSION" ]; then
    echo -e " ${C5}not found${R}"
    echo -e "   ${DIM}Will build from source.${R}"
  else
    echo -e " ${C4}v${VERSION}${R}"
  fi
fi

# ═══════════════════════════════════════════════════
#  Step 2 — Download or build
# ═══════════════════════════════════════════════════
DOWNLOADED=false

if [ -n "$VERSION" ]; then
  DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/openanalyst-${TARGET}"
  echo -ne "   ${ARROW} ${DIM}Downloading binary...${R}"

  mkdir -p "$INSTALL_DIR"
  if curl -fsSL "$DOWNLOAD_URL" -o "${INSTALL_DIR}/${BINARY_NAME}" 2>/dev/null; then
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    DOWNLOADED=true
    echo -e " ${CHECK} ${DIM}done${R}"
  else
    echo -e " ${C5}unavailable${R}"
  fi
fi

if [ "$DOWNLOADED" = false ]; then
  if ! command -v cargo &>/dev/null; then
    echo ""
    echo -e "   ${C5}Rust is required to build from source.${R}"
    echo ""
    echo -e "   ${DIM}Install Rust:${R}"
    echo -e "   ${C2}curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${R}"
    echo ""
    exit 1
  fi

  echo -ne "   ${ARROW} ${DIM}Building from source (this may take a few minutes)...${R}"
  TMPDIR=$(mktemp -d)
  git clone --depth 1 "https://github.com/${REPO}.git" "$TMPDIR/openanalyst-cli" 2>/dev/null
  cd "$TMPDIR/openanalyst-cli/rust"
  cargo build --release -p openanalyst-cli --quiet 2>&1
  mkdir -p "$INSTALL_DIR"
  cp "target/release/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
  rm -rf "$TMPDIR"
  echo -e " ${CHECK}"
fi

# ═══════════════════════════════════════════════════
#  Step 3 — PATH
# ═══════════════════════════════════════════════════
echo -ne "   ${ARROW} ${DIM}Configuring PATH...${R}"
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
    echo -e " ${CHECK} ${DIM}added to ${SHELL_RC}${R}"
  else
    echo -e " ${CHECK} ${DIM}already configured${R}"
  fi
else
  echo -e " ${C5}no shell rc found — add ${INSTALL_DIR} to PATH manually${R}"
fi
export PATH="${INSTALL_DIR}:$PATH"

# ═══════════════════════════════════════════════════
#  Step 4 — Config directory + .env
# ═══════════════════════════════════════════════════
echo -ne "   ${ARROW} ${DIM}Creating config...${R}"
mkdir -p "$CONFIG_DIR"

if [ ! -f "$CONFIG_DIR/.env" ]; then
  cat > "$CONFIG_DIR/.env" << 'ENVEOF'
# ═══════════════════════════════════════════════════════════════════
#  OpenAnalyst CLI — Environment Configuration
# ═══════════════════════════════════════════════════════════════════
#
#  Add your API keys below. The CLI loads this file on every startup.
#  Uncomment and fill in the providers you want to use.
#  Or run `openanalyst login` for interactive browser-based setup.
#
#  Docs: https://github.com/OpenAnalystInc/cli
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

# ── Model Override ───────────────────────────────────────────────

# OPENANALYST_MODEL=claude-sonnet-4-6
ENVEOF
  echo -e " ${CHECK} ${DIM}~/.openanalyst/.env${R}"
else
  echo -e " ${CHECK} ${DIM}already exists${R}"
fi

# ═══════════════════════════════════════════════════
#  Summary
# ═══════════════════════════════════════════════════
CLI_VERSION=$("${INSTALL_DIR}/${BINARY_NAME}" --version 2>&1 | head -2 | tail -1 | xargs 2>/dev/null || echo "openanalyst")

echo ""
echo ""
echo -e "   ${DIM}────────────────────────────────────────────${R}"
echo ""
echo -e "   ${C4}${BOLD}✓ Installation complete${R}"
echo ""
echo -e "   ${DIM}┌──────────────────────────────────────────┐${R}"
echo -e "   ${DIM}│${R}                                          ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${BOLD}Binary${R}     ${INSTALL_DIR}/${BINARY_NAME}     ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${BOLD}Config${R}     ~/.openanalyst/.env           ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${BOLD}Version${R}    ${CLI_VERSION}              ${DIM}│${R}"
echo -e "   ${DIM}│${R}                                          ${DIM}│${R}"
echo -e "   ${DIM}└──────────────────────────────────────────┘${R}"
echo ""
echo -e "   ${BOLD}Next steps${R}"
echo ""
echo -e "   ${C2}1.${R} ${BOLD}Login to your LLM provider${R}"
echo ""
echo -e "      ${C2}\$ openanalyst login${R}"
echo ""
echo -e "      ${DIM}Select a provider, authenticate via browser${R}"
echo -e "      ${DIM}or paste your API key. Credentials are saved${R}"
echo -e "      ${DIM}and remembered across sessions.${R}"
echo ""
echo -e "   ${C2}2.${R} ${BOLD}Start coding${R}"
echo ""
echo -e "      ${C2}\$ openanalyst${R}"
echo ""
echo ""
echo -e "   ${DIM}┌──────────────────────────────────────────┐${R}"
echo -e "   ${DIM}│${R}                                          ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${BOLD}7 LLM Providers. One Interface.${R}         ${DIM}│${R}"
echo -e "   ${DIM}│${R}                                          ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${C2}■${R} OpenAnalyst ${DIM}(default)${R}                 ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${C2}■${R} Anthropic / Claude  ${DOT} direct API       ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${C2}■${R} OpenAI / Codex     ${DOT} direct API        ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${C2}■${R} Google Gemini      ${DOT} direct API        ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${C2}■${R} xAI / Grok                              ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${C2}■${R} OpenRouter         ${DOT} 350+ models       ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${C2}■${R} Amazon Bedrock                           ${DIM}│${R}"
echo -e "   ${DIM}│${R}                                          ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${DIM}Switch models mid-conversation with${R}      ${DIM}│${R}"
echo -e "   ${DIM}│${R}  ${C2}/model gpt-4o${R}  ${DIM}or${R}  ${C2}/model gemini-2.5-pro${R} ${DIM}│${R}"
echo -e "   ${DIM}│${R}                                          ${DIM}│${R}"
echo -e "   ${DIM}└──────────────────────────────────────────┘${R}"
echo ""
if [ -n "$SHELL_RC" ]; then
  echo -e "   ${DIM}Reload your shell:  source ${SHELL_RC}${R}"
fi
echo -e "   ${DIM}Documentation:      github.com/OpenAnalystInc/cli${R}"
echo -e "   ${DIM}Support:            anit@openanalyst.com${R}"
echo ""
echo ""
