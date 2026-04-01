#!/usr/bin/env bash
set -e

# OpenAnalyst CLI Installer — macOS / Linux
# Usage: curl -fsSL https://openanalyst.com/install.sh | bash

BINARY_NAME="openanalyst"
INSTALL_DIR="${HOME}/.local/bin"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ""
echo "  ######   #####"
echo " ##    ## ##   ##"
echo " ##    ## #######"
echo " ##    ## ##   ##"
echo "  ######  ##   ##"
echo ""
echo " OpenAnalyst CLI Installer"
echo " ─────────────────────────"
echo ""

# Check if Rust is installed (needed for building from source)
if ! command -v cargo &> /dev/null; then
    echo " [!] Rust/Cargo not found. Install from https://rustup.rs"
    echo "     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build release binary
echo " [1/3] Building release binary..."
cd "${REPO_DIR}/rust"
cargo build --release --quiet 2>&1

BINARY_PATH="${REPO_DIR}/rust/target/release/${BINARY_NAME}"
if [ ! -f "$BINARY_PATH" ]; then
    echo " [!] Build failed — binary not found at ${BINARY_PATH}"
    exit 1
fi

# Create install directory
echo " [2/3] Installing to ${INSTALL_DIR}..."
mkdir -p "$INSTALL_DIR"
cp "$BINARY_PATH" "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

# Add to PATH if not already there
echo " [3/3] Configuring PATH..."
SHELL_RC=""
if [ -f "${HOME}/.zshrc" ]; then
    SHELL_RC="${HOME}/.zshrc"
elif [ -f "${HOME}/.bashrc" ]; then
    SHELL_RC="${HOME}/.bashrc"
elif [ -f "${HOME}/.bash_profile" ]; then
    SHELL_RC="${HOME}/.bash_profile"
fi

PATH_LINE="export PATH=\"${INSTALL_DIR}:\$PATH\""
if [ -n "$SHELL_RC" ]; then
    if ! grep -q "${INSTALL_DIR}" "$SHELL_RC" 2>/dev/null; then
        echo "" >> "$SHELL_RC"
        echo "# OpenAnalyst CLI" >> "$SHELL_RC"
        echo "$PATH_LINE" >> "$SHELL_RC"
        echo " Added ${INSTALL_DIR} to PATH in ${SHELL_RC}"
    else
        echo " PATH already configured in ${SHELL_RC}"
    fi
else
    echo " [!] Could not detect shell config. Add this manually:"
    echo "     ${PATH_LINE}"
fi

echo ""
echo " ── Installation complete ──"
echo ""
echo " Version:  $(${INSTALL_DIR}/${BINARY_NAME} --version 2>&1 | head -2 | tail -1 | xargs)"
echo " Binary:   ${INSTALL_DIR}/${BINARY_NAME}"
echo ""
echo " Configure your API credentials:"
echo ""
echo "   # OpenAnalyst API"
echo "   export OPENANALYST_AUTH_TOKEN=\"your-api-key-here\""
echo ""
echo "   # Or use Anthropic / OpenAI / OpenRouter / Bedrock"
echo "   export ANTHROPIC_API_KEY=\"sk-ant-...\""
echo "   export OPENAI_API_KEY=\"sk-...\""
echo "   export OPENROUTER_API_KEY=\"sk-or-...\""
echo ""
echo " Start using:"
echo ""
echo "   $ openanalyst"
echo ""
echo " Reload your shell or run: source ${SHELL_RC}"
echo ""
