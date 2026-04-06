import { jsx as _jsx } from "react/jsx-runtime";
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
import { createContext, useContext, useState, useCallback, useMemo } from 'react';
import { themeManager } from '../themes/theme-manager.js';
const ThemeContext = createContext(null);
export function ThemeProvider({ children }) {
    // Revision counter forces re-render when the theme changes.
    const [, setRevision] = useState(0);
    const setTheme = useCallback((name) => {
        const ok = themeManager.setTheme(name);
        if (ok)
            setRevision((r) => r + 1);
        return ok;
    }, []);
    const detectAndApply = useCallback(() => {
        themeManager.detectAndApply();
        setRevision((r) => r + 1);
    }, []);
    const getSpinnerGradient = useCallback((steps) => {
        return themeManager.getSpinnerGradient(steps);
    }, []);
    const interpolateColor = useCallback((from, to, factor) => {
        return themeManager.interpolateColor(from, to, factor);
    }, []);
    const value = useMemo(() => ({
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
    const currentValue = useMemo(() => ({
        ...value,
        colors: themeManager.getSemanticColors(),
        type: themeManager.getThemeType(),
    }), [value]);
    return (_jsx(ThemeContext.Provider, { value: currentValue, children: children }));
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
export function useTheme() {
    const ctx = useContext(ThemeContext);
    if (!ctx) {
        throw new Error('useTheme() must be used within a <ThemeProvider>');
    }
    return ctx;
}
//# sourceMappingURL=theme-context.js.map