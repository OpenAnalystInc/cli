/**
 * OpenAnalyst CLI — Theme barrel exports.
 *
 * Import from here instead of individual files:
 *   import { themeManager, useTheme, OADarkTheme } from '../themes/index.js';
 */
// Raw color palette (rarely needed directly — prefer semantic tokens)
export * from './colors.js';
// Theme class and built-in instances
export { OATheme, OADarkTheme, OALightTheme } from './theme.js';
// Singleton manager
export { themeManager } from './theme-manager.js';
// React context + hook (the primary API for components)
export { ThemeProvider, useTheme } from '../contexts/theme-context.js';
//# sourceMappingURL=index.js.map