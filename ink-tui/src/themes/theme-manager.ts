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

import tinygradient from 'tinygradient';
import type { SemanticColors, ThemeType } from './semantic-tokens.js';
import { OATheme, OADarkTheme, OALightTheme } from './theme.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Attempts to determine whether the terminal has a light or dark background.
 *
 * Heuristics (in priority order):
 *   1. COLORFGBG environment variable (set by some terminals: "fg;bg")
 *   2. Defaults to 'dark' — most developer terminals are dark.
 */
function detectTerminalBackground(): ThemeType {
  const colorfgbg = process.env['COLORFGBG'];
  if (colorfgbg) {
    const parts = colorfgbg.split(';');
    const bg = parseInt(parts[parts.length - 1] ?? '', 10);
    // ANSI colors 0-6 and 8 are typically dark, 7 and 9-15 are light.
    if (!isNaN(bg) && (bg === 7 || (bg >= 9 && bg <= 15))) {
      return 'light';
    }
  }
  return 'dark';
}

// ---------------------------------------------------------------------------
// ThemeManager
// ---------------------------------------------------------------------------

class ThemeManager {
  private activeTheme: OATheme;
  private cachedSemanticColors: SemanticColors | undefined;
  private cachedGradientColors: string[] | undefined;
  private readonly themes: ReadonlyMap<string, OATheme>;

  constructor() {
    const themeMap = new Map<string, OATheme>();
    themeMap.set(OADarkTheme.name, OADarkTheme);
    themeMap.set(OALightTheme.name, OALightTheme);
    this.themes = themeMap;

    // Auto-select based on terminal background
    const detected = detectTerminalBackground();
    this.activeTheme = detected === 'light' ? OALightTheme : OADarkTheme;
  }

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /** Returns the full SemanticColors for the active theme. */
  getSemanticColors(): SemanticColors {
    if (this.cachedSemanticColors) {
      return this.cachedSemanticColors;
    }
    this.cachedSemanticColors = this.activeTheme.colors;
    return this.cachedSemanticColors;
  }

  /** Returns the active theme instance. */
  getActiveTheme(): OATheme {
    return this.activeTheme;
  }

  /** Returns the active theme type. */
  getThemeType(): ThemeType {
    return this.activeTheme.type;
  }

  /**
   * Switches to a named theme.
   * @returns `true` if the theme was found and applied, `false` otherwise.
   */
  setTheme(name: string): boolean {
    const theme = this.themes.get(name);
    if (!theme) return false;
    if (this.activeTheme === theme) return true;

    this.activeTheme = theme;
    this.clearCache();
    return true;
  }

  /**
   * Auto-detects light/dark and switches to the appropriate built-in theme.
   * Call this on startup or when the terminal emits a background-change signal.
   */
  detectAndApply(): void {
    const type = detectTerminalBackground();
    this.setTheme(type === 'light' ? OALightTheme.name : OADarkTheme.name);
  }

  /** Returns all available theme names. */
  getAvailableThemes(): readonly string[] {
    return [...this.themes.keys()];
  }

  // -------------------------------------------------------------------------
  // Gradient helpers
  // -------------------------------------------------------------------------

  /**
   * Returns an array of N interpolated hex colors for smooth spinner animation.
   * The gradient is built from the active theme's spinner keyframes.
   *
   * @param steps Number of discrete color steps (default 64 for ~4s at 16fps).
   */
  getSpinnerGradient(steps = 64): readonly string[] {
    if (this.cachedGradientColors && this.cachedGradientColors.length === steps) {
      return this.cachedGradientColors;
    }

    const keyframes = this.activeTheme.colors.spinner.gradient;
    try {
      const gradient = tinygradient(keyframes as string[]);
      this.cachedGradientColors = gradient.rgb(steps).map((c) => c.toHexString());
    } catch {
      // Fallback: just repeat the keyframes
      this.cachedGradientColors = [...keyframes] as string[];
    }

    return this.cachedGradientColors;
  }

  /**
   * Interpolates between two colors by a factor (0..1).
   * Useful for fade effects, progress indicators, etc.
   */
  interpolateColor(from: string, to: string, factor: number): string {
    if (factor <= 0) return from;
    if (factor >= 1) return to;
    if (!from || !to) return from || to || '';

    try {
      const gradient = tinygradient(from, to);
      return gradient.rgbAt(factor).toHexString();
    } catch {
      return from;
    }
  }

  // -------------------------------------------------------------------------
  // Private
  // -------------------------------------------------------------------------

  private clearCache(): void {
    this.cachedSemanticColors = undefined;
    this.cachedGradientColors = undefined;
  }
}

// ---------------------------------------------------------------------------
// Singleton export
// ---------------------------------------------------------------------------

export const themeManager = new ThemeManager();
