/**
 * TerminalProvider — tracks terminal dimensions and exposes responsive helpers.
 *
 * Listens to process.stdout resize events for live updates.
 * Components use `useTerminal()` to read the current terminal size
 * and derived responsive flags.
 */
import React, { type ReactNode } from 'react';
export interface TerminalContextValue {
    /** Current terminal width in columns. */
    width: number;
    /** Current terminal height in rows. */
    height: number;
    /** True when width < 80 — triggers compact layouts. */
    isNarrow: boolean;
    /** True when width >= 60 — sidebar can be shown. */
    canShowSidebar: boolean;
}
export interface TerminalProviderProps {
    children: ReactNode;
}
export declare function TerminalProvider({ children }: TerminalProviderProps): React.ReactElement;
/**
 * Returns the current terminal dimensions and responsive flags.
 *
 * Must be called within a `<TerminalProvider>`.
 */
export declare function useTerminal(): TerminalContextValue;
