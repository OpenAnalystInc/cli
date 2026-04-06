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
import React from 'react';
import type { SemanticColors, ThemeType } from '../themes/semantic-tokens.js';
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
interface ThemeProviderProps {
    children: React.ReactNode;
}
export declare function ThemeProvider({ children }: ThemeProviderProps): React.ReactElement;
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
export declare function useTheme(): ThemeContextValue;
export {};
