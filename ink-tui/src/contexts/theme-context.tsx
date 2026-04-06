/**
 * OpenAnalyst CLI — React context for theme colors.
 *
 * Wraps the singleton themeManager in a React context so components
 * can access semantic colors via `useTheme()` and re-render on
 * theme changes.
 *
 * Usage:
 *   // In root app:
 *   <ThemeProvider>
 *     <App />
 *   </ThemeProvider>
 *
 *   // In any component:
 *   const theme = useTheme();
 *   <Text color={theme.text.primary}>Hello</Text>
 */

import React, { createContext, useContext, useState, useCallback, useMemo } from 'react';
import type { SemanticColors, ThemeType } from '../themes/semantic-tokens.js';
import { themeManager } from '../themes/theme-manager.js';

// ---------------------------------------------------------------------------
// Context value
// ---------------------------------------------------------------------------

interface ThemeContextValue {
  /** The resolved semantic color tokens for the active theme. */
  colors: SemanticColors;
  /** Current theme type: 'dark' or 'light'. */
  type: ThemeType;
  /** Switch to a named theme. Returns false if the name is unknown. */
  setTheme: (name: string) => boolean;
  /** Re-detect terminal background and switch automatically. */
  detectAndApply: () => void;
  /** Get interpolated spinner gradient (N steps). */
  getSpinnerGradient: (steps?: number) => readonly string[];
  /** Interpolate between two colors (factor 0..1). */
  interpolateColor: (from: string, to: string, factor: number) => string;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

interface ThemeProviderProps {
  children: React.ReactNode;
}

export function ThemeProvider({ children }: ThemeProviderProps): React.ReactElement {
  // Revision counter forces re-render when the theme changes.
  const [, setRevision] = useState(0);

  const setTheme = useCallback((name: string): boolean => {
    const ok = themeManager.setTheme(name);
    if (ok) setRevision((r) => r + 1);
    return ok;
  }, []);

  const detectAndApply = useCallback((): void => {
    themeManager.detectAndApply();
    setRevision((r) => r + 1);
  }, []);

  const getSpinnerGradient = useCallback((steps?: number): readonly string[] => {
    return themeManager.getSpinnerGradient(steps);
  }, []);

  const interpolateColor = useCallback((from: string, to: string, factor: number): string => {
    return themeManager.interpolateColor(from, to, factor);
  }, []);

  const value = useMemo<ThemeContextValue>(() => ({
    colors: themeManager.getSemanticColors(),
    type: themeManager.getThemeType(),
    setTheme,
    detectAndApply,
    getSpinnerGradient,
    interpolateColor,
  }), [setTheme, detectAndApply, getSpinnerGradient, interpolateColor]);

  // Re-read colors on every render so a revision bump picks up the new theme.
  // The useMemo deps include the stable callbacks, but the actual colors
  // object is always fresh from the manager.
  const currentValue = useMemo<ThemeContextValue>(() => ({
    ...value,
    colors: themeManager.getSemanticColors(),
    type: themeManager.getThemeType(),
  }), [value]);

  return (
    <ThemeContext.Provider value={currentValue}>
      {children}
    </ThemeContext.Provider>
  );
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

/**
 * Returns the current theme's semantic colors and utilities.
 *
 * Must be called within a `<ThemeProvider>`.
 *
 * @example
 * ```tsx
 * const { colors } = useTheme();
 * <Text color={colors.text.heading}>Title</Text>
 * <Box borderColor={colors.border.default}>...</Box>
 * ```
 */
export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error('useTheme() must be used within a <ThemeProvider>');
  }
  return ctx;
}
