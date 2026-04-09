#!/bin/sh
# OpenAnalyst CLI Installer — macOS / Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.sh | sh

set -e

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

# Install via npm
echo ""
printf "   Installing..."

if npm install -g @openanalystinc/openanalyst-cli@latest >/dev/null 2>&1; then
    echo " done"
else
    echo " failed"
    echo "   Try: sudo npm install -g @openanalystinc/openanalyst-cli"
    exit 1
fi

# Verify
echo ""
VERSION=$(openanalyst --version 2>/dev/null || echo "Installed")
echo "   ✓ $VERSION"
echo ""
echo "   To get started:"
echo ""
echo "     openanalyst"
echo ""
