#!/bin/sh
# OpenAnalyst CLI Installer - macOS / Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.sh | sh

set -e

PACKAGE_NAME="@openanalystinc/openanalyst-cli"
LEGACY_PACKAGE_NAME="@openanalyst/cli"

repair_openanalyst_global_bin() {
    prefix=$(npm prefix -g 2>/dev/null || true)
    [ -n "$prefix" ] || return 0

    shim="$prefix/bin/openanalyst"
    [ -e "$shim" ] || [ -L "$shim" ] || return 0

    target=""
    if [ -L "$shim" ]; then
        target=$(readlink "$shim" 2>/dev/null || true)
    fi

    pattern='@openanalyst(/|\\)cli|@openanalystinc(/|\\)openanalyst-cli|node_modules(/|\\)@openanalyst'
    if { [ -n "$target" ] && printf '%s' "$target" | grep -Eiq "$pattern"; } \
        || grep -Eiq "$pattern" "$shim" 2>/dev/null; then
        rm -f "$shim" 2>/dev/null || true
    fi
}

echo ""
echo "   OpenAnalyst CLI"
echo "   The Universal AI Agent for Your Terminal"
echo ""

# Check for Node.js
if ! command -v node >/dev/null 2>&1; then
    echo "   Node.js is required but not found."
    echo "   Install from: https://nodejs.org/"
    echo ""
    exit 1
fi

NODE_VERSION=$(node --version 2>/dev/null)
echo "   Node.js $NODE_VERSION detected"

# Repair legacy global installs before npm links the new binary.
repair_openanalyst_global_bin
npm uninstall -g "$LEGACY_PACKAGE_NAME" "$PACKAGE_NAME" >/dev/null 2>&1 || true
repair_openanalyst_global_bin

# Install via npm
echo ""
printf "   Installing..."

if npm install -g "$PACKAGE_NAME@latest" >/dev/null 2>&1; then
    echo " done"
else
    echo " failed"
    echo "   Try:"
    echo "     npm uninstall -g $LEGACY_PACKAGE_NAME $PACKAGE_NAME"
    echo "     npm install -g $PACKAGE_NAME@latest"
    exit 1
fi

# Verify
echo ""
VERSION=$(openanalyst --version 2>/dev/null || echo "Installed")
echo "   $VERSION"
echo ""
echo "   To get started:"
echo ""
echo "     openanalyst"
echo ""
