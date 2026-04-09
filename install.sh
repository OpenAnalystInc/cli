#!/bin/sh
# OpenAnalyst CLI Installer - macOS / Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.sh | sh

set -eu

REPO="OpenAnalystInc/cli"
BASE_URL="https://github.com/$REPO/releases/latest/download"

echo ""
echo "   OpenAnalyst CLI"
echo "   The Universal AI Agent for Your Terminal"
echo ""

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64|aarch64) ASSET="openanalyst-aarch64-apple-darwin" ;;
      x86_64|amd64) ASSET="openanalyst-x86_64-apple-darwin" ;;
      *)
        echo "   Unsupported macOS architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64|amd64) ASSET="openanalyst-x86_64-unknown-linux-gnu" ;;
      arm64|aarch64) ASSET="openanalyst-aarch64-unknown-linux-gnu" ;;
      *)
        echo "   Unsupported Linux architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  *)
    echo "   Unsupported operating system: $OS"
    exit 1
    ;;
esac

TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t openanalyst)"
TMP_BIN="$TMP_DIR/openanalyst"
URL="$BASE_URL/$ASSET"

cleanup() {
  rm -rf "$TMP_DIR"
}

trap cleanup EXIT INT TERM

echo "   Downloading $ASSET"

if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "$TMP_BIN"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$TMP_BIN" "$URL"
else
  echo "   curl or wget is required to download the binary."
  exit 1
fi

chmod +x "$TMP_BIN"

TARGET_DIR="${OPENANALYST_INSTALL_DIR:-}"
USE_SUDO=0

if [ -z "$TARGET_DIR" ]; then
  if [ -w /usr/local/bin ]; then
    TARGET_DIR="/usr/local/bin"
  elif command -v sudo >/dev/null 2>&1; then
    TARGET_DIR="/usr/local/bin"
    USE_SUDO=1
  else
    TARGET_DIR="${HOME}/.local/bin"
  fi
fi

mkdir -p "$TARGET_DIR"

if [ "$USE_SUDO" -eq 1 ]; then
  sudo install "$TMP_BIN" "$TARGET_DIR/openanalyst"
else
  install "$TMP_BIN" "$TARGET_DIR/openanalyst"
fi

echo ""
echo "   Installed to $TARGET_DIR/openanalyst"

case ":$PATH:" in
  *:"$TARGET_DIR":*)
    ;;
  *)
    if [ "$TARGET_DIR" = "${HOME}/.local/bin" ]; then
      echo "   Add $TARGET_DIR to your PATH if it is not already present."
    fi
    ;;
esac

echo ""
if "$TARGET_DIR/openanalyst" --version >/dev/null 2>&1; then
  VERSION="$("$TARGET_DIR/openanalyst" --version)"
  echo "   OK  $VERSION"
else
  echo "   Installed, but version check did not complete."
fi

echo ""
echo "   To get started:"
echo ""
echo "     openanalyst"
echo "     openanalyst --notui"
echo "     openanalyst --serve 8080"
echo ""
