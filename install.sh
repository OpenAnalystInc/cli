#!/bin/sh
# OpenAnalyst CLI Installer - macOS / Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.sh | sh

set -e

REPO="OpenAnalystInc/cli"
BASE_URL="https://github.com/${REPO}/releases/latest/download"

echo ""
echo "   OpenAnalyst CLI"
echo "   The Universal AI Agent for Your Terminal"
echo ""

if ! command -v curl >/dev/null 2>&1; then
    echo "   curl is required but not found."
    echo "   Install curl and rerun this command."
    echo ""
    exit 1
fi

OS="$(uname -s 2>/dev/null || echo unknown)"
ARCH="$(uname -m 2>/dev/null || echo unknown)"

case "$OS" in
    Linux) PLATFORM="unknown-linux-gnu" ;;
    Darwin) PLATFORM="apple-darwin" ;;
    *)
        echo "   Unsupported OS: $OS"
        echo "   Download a release manually from https://github.com/${REPO}/releases/latest"
        echo ""
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64) TARGET_ARCH="x86_64" ;;
    arm64|aarch64) TARGET_ARCH="aarch64" ;;
    *)
        echo "   Unsupported architecture: $ARCH"
        echo "   Download a release manually from https://github.com/${REPO}/releases/latest"
        echo ""
        exit 1
        ;;
esac

ASSET="openanalyst-${TARGET_ARCH}-${PLATFORM}"
DOWNLOAD_URL="${BASE_URL}/${ASSET}"
TMP_FILE="$(mktemp "${TMPDIR:-/tmp}/openanalyst.XXXXXX")"

INSTALL_DIR="/usr/local/bin"
if [ ! -w "$INSTALL_DIR" ]; then
    INSTALL_DIR="${HOME}/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

echo "   Download target: $ASSET"
echo ""
printf "   Downloading..."
curl -fsSL "$DOWNLOAD_URL" -o "$TMP_FILE"
chmod +x "$TMP_FILE"
echo " done"

echo ""
printf "   Installing..."
mv "$TMP_FILE" "${INSTALL_DIR}/openanalyst"
echo " done"

echo ""
VERSION="$("${INSTALL_DIR}/openanalyst" --version 2>/dev/null || echo "Installed")"
echo "   OK $VERSION"
echo ""
echo "   Installed to: ${INSTALL_DIR}/openanalyst"
case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        echo "   Add this to your PATH if needed:"
        echo "     export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac
echo ""
echo "   To get started:"
echo ""
echo "     openanalyst"
echo ""
