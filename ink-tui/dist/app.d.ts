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
import { type BridgeConfig } from './engine/index.js';
export interface AppProps {
    /** Engine configuration. Defaults to mock mode if OA_MOCK=1 or --mock flag. */
    engineConfig?: BridgeConfig;
}
export declare function App({ engineConfig }: AppProps): React.ReactElement;
