/**
 * OpenAnalyst CLI — Theme manager (singleton).
 *
 * Follows the same singleton pattern as Google Gemini CLI's ThemeManager
 * but tailored for the OA palette and two built-in themes.
 *
 * Usage:
 *   import { themeManager } from './theme-manager.js';
 *   const colors = themeManager.getSemanticColors();
 *   // colors.text.primary, colors.border.default, etc.
 */
import type { SemanticColors, ThemeType } from './semantic-tokens.js';
import { OATheme } from './theme.js';
declare class ThemeManager {
    private activeTheme;
    private cachedSemanticColors;
    private cachedGradientColors;
    private readonly themes;
    constructor();
    /** Returns the full SemanticColors for the active theme. */
    getSemanticColors(): SemanticColors;
    /** Returns the active theme instance. */
    getActiveTheme(): OATheme;
    /** Returns the active theme type. */
    getThemeType(): ThemeType;
    /**
     * Switches to a named theme.
     * @returns `true` if the theme was found and applied, `false` otherwise.
     */
    setTheme(name: string): boolean;
    /**
     * Auto-detects light/dark and switches to the appropriate built-in theme.
     * Call this on startup or when the terminal emits a background-change signal.
     */
    detectAndApply(): void;
    /** Returns all available theme names. */
    getAvailableThemes(): readonly string[];
    /**
     * Returns an array of N interpolated hex colors for smooth spinner animation.
     * The gradient is built from the active theme's spinner keyframes.
     *
     * @param steps Number of discrete color steps (default 64 for ~4s at 16fps).
     */
    getSpinnerGradient(steps?: number): readonly string[];
    /**
     * Interpolates between two colors by a factor (0..1).
     * Useful for fade effects, progress indicators, etc.
     */
    interpolateColor(from: string, to: string, factor: number): string;
    private clearCache;
}
export declare const themeManager: ThemeManager;
export {};
