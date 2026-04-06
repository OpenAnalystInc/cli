import { jsx as _jsx } from "react/jsx-runtime";
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
import { createContext, useContext, useEffect, useRef, useMemo, useState, } from 'react';
import { PlaywrightMCPLifecycle, } from './playwright-lifecycle.js';
const PlaywrightMCPContext = createContext(null);
export function PlaywrightMCPProvider({ config, autoStart = true, children, }) {
    const [state, setState] = useState('stopped');
    const [statusMessage, setStatusMessage] = useState(null);
    // Create lifecycle once
    const lifecycleRef = useRef(null);
    if (lifecycleRef.current === null) {
        lifecycleRef.current = new PlaywrightMCPLifecycle(config);
    }
    const lifecycle = lifecycleRef.current;
    // Wire state changes
    useEffect(() => {
        const handleStateChange = (newState, message) => {
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
    const value = useMemo(() => ({
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
    return (_jsx(PlaywrightMCPContext.Provider, { value: value, children: children }));
}
// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------
/**
 * Access the Playwright MCP server state and lifecycle.
 * Must be used within a PlaywrightMCPProvider.
 */
export function usePlaywrightMCP() {
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
export function usePlaywrightMCPReady() {
    const ctx = useContext(PlaywrightMCPContext);
    return ctx?.isReady ?? false;
}
//# sourceMappingURL=playwright-context.js.map