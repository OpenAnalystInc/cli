/**
 * PlaywrightMCPProvider — React context that manages the Playwright MCP server lifecycle.
 *
 * Wraps PlaywrightMCPLifecycle in a React context so any component can:
 *   - Check if the Playwright MCP server is running
 *   - See the current state (stopped, starting, ready, error, unavailable)
 *   - Access the server process for engine bridge wiring
 *
 * Provider order in the tree:
 *   TerminalProvider > ThemeProvider > KeypressProvider > UIStateProvider
 *   > ChatProvider > PlaywrightMCPProvider > EngineProvider > layout
 *
 * The server starts automatically on mount and stops on unmount.
 */
import React, { type ReactNode } from 'react';
import { PlaywrightMCPLifecycle, type PlaywrightMCPConfig, type PlaywrightMCPState } from './playwright-lifecycle.js';
export interface PlaywrightMCPContextValue {
    /** Current server state. */
    state: PlaywrightMCPState;
    /** Whether the server is ready to accept MCP requests. */
    isReady: boolean;
    /** Human-readable status message (e.g., error details). */
    statusMessage: string | null;
    /** The underlying lifecycle manager (for engine bridge wiring). */
    lifecycle: PlaywrightMCPLifecycle;
    /** Manually restart the server. */
    restart: () => Promise<void>;
    /** Manually stop the server. */
    stop: () => Promise<void>;
}
export interface PlaywrightMCPProviderProps {
    /** Configuration for the Playwright MCP server. */
    config?: PlaywrightMCPConfig;
    /** Whether to auto-start the server on mount. Default: true. */
    autoStart?: boolean;
    children: ReactNode;
}
export declare function PlaywrightMCPProvider({ config, autoStart, children, }: PlaywrightMCPProviderProps): React.ReactElement;
/**
 * Access the Playwright MCP server state and lifecycle.
 * Must be used within a PlaywrightMCPProvider.
 */
export declare function usePlaywrightMCP(): PlaywrightMCPContextValue;
/**
 * Convenience hook: returns true if Playwright MCP is available and ready.
 * Safe to call outside of PlaywrightMCPProvider (returns false).
 */
export declare function usePlaywrightMCPReady(): boolean;
