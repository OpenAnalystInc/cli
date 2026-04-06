/**
 * TerminalProvider — tracks terminal dimensions and exposes responsive helpers.
 *
 * Listens to process.stdout resize events for live updates.
 * Components use `useTerminal()` to read the current terminal size
 * and derived responsive flags.
 */

import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useMemo,
  type ReactNode,
} from 'react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getTerminalWidth(): number {
  return process.stdout.columns ?? 80;
}

function getTerminalHeight(): number {
  return process.stdout.rows ?? 24;
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

const TerminalContext = createContext<TerminalContextValue | null>(null);

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

export interface TerminalProviderProps {
  children: ReactNode;
}

export function TerminalProvider({ children }: TerminalProviderProps): React.ReactElement {
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

  const value = useMemo<TerminalContextValue>(
    () => ({
      width,
      height,
      isNarrow: width < 80,
      canShowSidebar: width >= 60,
    }),
    [width, height],
  );

  return (
    <TerminalContext.Provider value={value}>
      {children}
    </TerminalContext.Provider>
  );
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

/**
 * Returns the current terminal dimensions and responsive flags.
 *
 * Must be called within a `<TerminalProvider>`.
 */
export function useTerminal(): TerminalContextValue {
  const ctx = useContext(TerminalContext);
  if (!ctx) {
    throw new Error('useTerminal() must be used within a <TerminalProvider>');
  }
  return ctx;
}
