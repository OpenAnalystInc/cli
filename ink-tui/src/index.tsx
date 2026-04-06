/**
 * Entry point — renders the Ink TUI app.
 *
 * Handles:
 *   - CLI argument parsing
 *   - Graceful shutdown on Ctrl+C / SIGTERM
 *   - Uncaught error handling
 */

import React from 'react';
import { render } from 'ink';
import { App } from './app.js';
import type { BridgeConfig } from './engine/index.js';

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

const enginePath = process.env['OA_ENGINE_PATH'] || 'openanalyst';

const engineConfig: BridgeConfig = {
  binaryPath: enginePath,
  autoRestart: true,
  maxRestartAttempts: 3,
};

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

const { unmount, waitUntilExit } = render(
  <App engineConfig={engineConfig} />,
);

// ---------------------------------------------------------------------------
// Graceful shutdown
// ---------------------------------------------------------------------------

function cleanup(): void {
  unmount();
}

process.on('SIGTERM', cleanup);
process.on('SIGINT', cleanup);

// Handle uncaught errors gracefully
process.on('uncaughtException', (err) => {
  process.stderr.write(`\nFatal error: ${err.message}\n`);
  if (err.stack) {
    process.stderr.write(`${err.stack}\n`);
  }
  cleanup();
  process.exit(1);
});

process.on('unhandledRejection', (reason) => {
  process.stderr.write(`\nUnhandled rejection: ${String(reason)}\n`);
});

// Wait for the app to exit
waitUntilExit().then(() => {
  process.exit(0);
}).catch(() => {
  process.exit(1);
});
