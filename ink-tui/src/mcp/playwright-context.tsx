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

import React, {
  createContext,
  useContext,
  useEffect,
  useRef,
  useMemo,
  useState,
  type ReactNode,
} from 'react';

import {
  PlaywrightMCPLifecycle,
  type PlaywrightMCPConfig,
  type PlaywrightMCPState,
} from './playwright-lifecycle.js';

// ---------------------------------------------------------------------------
// Context value
// ---------------------------------------------------------------------------

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

const PlaywrightMCPContext = createContext<PlaywrightMCPContextValue | null>(null);

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

export interface PlaywrightMCPProviderProps {
  /** Configuration for the Playwright MCP server. */
  config?: PlaywrightMCPConfig;
  /** Whether to auto-start the server on mount. Default: true. */
  autoStart?: boolean;
  children: ReactNode;
}

export function PlaywrightMCPProvider({
  config,
  autoStart = true,
  children,
}: PlaywrightMCPProviderProps): React.ReactElement {
  const [state, setState] = useState<PlaywrightMCPState>('stopped');
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  // Create lifecycle once
  const lifecycleRef = useRef<PlaywrightMCPLifecycle | null>(null);
  if (lifecycleRef.current === null) {
    lifecycleRef.current = new PlaywrightMCPLifecycle(config);
  }
  const lifecycle = lifecycleRef.current;

  // Wire state changes
  useEffect(() => {
    const handleStateChange = (newState: PlaywrightMCPState, message?: string) => {
      setState(newState);
      setStatusMessage(message ?? null);
    };

    lifecycle.on('state_change', handleStateChange);

    // Auto-start if configured
    if (autoStart) {
      lifecycle.start().catch(() => {
        // Error is emitted via state_change, no need to handle here
      });
    }

    return () => {
      lifecycle.removeListener('state_change', handleStateChange);
      lifecycle.dispose().catch(() => {
        // Best-effort cleanup
      });
    };
  }, [lifecycle, autoStart]);

  // Build stable context value
  const value = useMemo<PlaywrightMCPContextValue>(() => ({
    state,
    isReady: state === 'ready',
    statusMessage,
    lifecycle,

    async restart() {
      await lifecycle.stop();
      await lifecycle.start();
    },

    async stop() {
      await lifecycle.stop();
    },
  }), [state, statusMessage, lifecycle]);

  return (
    <PlaywrightMCPContext.Provider value={value}>
      {children}
    </PlaywrightMCPContext.Provider>
  );
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

/**
 * Access the Playwright MCP server state and lifecycle.
 * Must be used within a PlaywrightMCPProvider.
 */
export function usePlaywrightMCP(): PlaywrightMCPContextValue {
  const ctx = useContext(PlaywrightMCPContext);
  if (!ctx) {
    throw new Error('usePlaywrightMCP() must be used within a <PlaywrightMCPProvider>');
  }
  return ctx;
}

/**
 * Convenience hook: returns true if Playwright MCP is available and ready.
 * Safe to call outside of PlaywrightMCPProvider (returns false).
 */
export function usePlaywrightMCPReady(): boolean {
  const ctx = useContext(PlaywrightMCPContext);
  return ctx?.isReady ?? false;
}
