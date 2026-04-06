/**
 * Root App component — wires all context providers around the main layout.
 *
 * Provider order (outermost to innermost):
 *   1. TerminalProvider  — terminal size tracking (no deps)
 *   2. ThemeProvider     — color tokens (no deps)
 *   3. KeypressProvider  — priority key dispatch (no deps)
 *   4. UIStateProvider   — central UI state (reads terminal, dispatches on keys)
 *   5. ChatProvider      — chat message store
 *   6. PlaywrightMCPProvider — auto-starts Playwright MCP server for browser tools
 *   7. EngineProvider    — connects to Rust engine, dispatches to chat + UI
 *   8. SessionProvider   — auto-saves chat sessions, enables /resume
 *   9. DefaultLayout     — renders the panel arrangement
 */

import React from 'react';
import { TerminalProvider } from './contexts/terminal-context.js';
import { ThemeProvider } from './contexts/theme-context.js';
import { KeypressProvider } from './contexts/keypress-context.js';
import { UIStateProvider } from './contexts/ui-state-context.js';
import { ChatProvider } from './contexts/chat-context.js';
import { PlaywrightMCPProvider } from './mcp/index.js';
import { EngineProvider, type BridgeConfig } from './engine/index.js';
import { SessionProvider } from './contexts/session-context.js';
import { DefaultLayout } from './layouts/default-layout.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface AppProps {
  /** Engine configuration. Defaults to spawning 'openanalyst' binary. */
  engineConfig?: BridgeConfig;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function App({ engineConfig }: AppProps): React.ReactElement {
  const config: BridgeConfig = engineConfig ?? {
    binaryPath: process.env['OA_ENGINE_PATH'] || 'openanalyst',
    autoRestart: true,
    maxRestartAttempts: 3,
  };

  return (
    <TerminalProvider>
      <ThemeProvider>
        <KeypressProvider>
          <UIStateProvider>
            <ChatProvider>
              <PlaywrightMCPProvider autoStart={true}>
                <EngineProvider config={config}>
                  <SessionProvider>
                    <DefaultLayout />
                  </SessionProvider>
                </EngineProvider>
              </PlaywrightMCPProvider>
            </ChatProvider>
          </UIStateProvider>
        </KeypressProvider>
      </ThemeProvider>
    </TerminalProvider>
  );
}
