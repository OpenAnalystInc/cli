import { jsx as _jsx } from "react/jsx-runtime";
/**
 * TerminalProvider — tracks terminal dimensions and exposes responsive helpers.
 *
 * Listens to process.stdout resize events for live updates.
 * Components use `useTerminal()` to read the current terminal size
 * and derived responsive flags.
 */
import { createContext, useContext, useState, useEffect, useMemo, } from 'react';
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function getTerminalWidth() {
    return process.stdout.columns ?? 80;
}
function getTerminalHeight() {
    return process.stdout.rows ?? 24;
}
// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------
const TerminalContext = createContext(null);
export function TerminalProvider({ children }) {
    const [width, setWidth] = useState(getTerminalWidth);
    const [height, setHeight] = useState(getTerminalHeight);
    useEffect(() => {
        const onResize = () => {
            setWidth(getTerminalWidth());
            setHeight(getTerminalHeight());
        };
        process.stdout.on('resize', onResize);
        return () => {
            process.stdout.off('resize', onResize);
        };
    }, []);
    const value = useMemo(() => ({
        width,
        height,
        isNarrow: width < 80,
        canShowSidebar: width >= 60,
    }), [width, height]);
    return (_jsx(TerminalContext.Provider, { value: value, children: children }));
}
// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------
/**
 * Returns the current terminal dimensions and responsive flags.
 *
 * Must be called within a `<TerminalProvider>`.
 */
export function useTerminal() {
    const ctx = useContext(TerminalContext);
    if (!ctx) {
        throw new Error('useTerminal() must be used within a <TerminalProvider>');
    }
    return ctx;
}
//# sourceMappingURL=terminal-context.js.map