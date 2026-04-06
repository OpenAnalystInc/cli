/**
 * Barrel exports for the MCP module.
 *
 * This module manages Model Context Protocol (MCP) servers that run alongside
 * the TUI, providing additional tool capabilities to the AI engine.
 *
 * Currently bundled:
 *   - Playwright MCP — browser automation (navigate, click, fill, screenshot, etc.)
 */
// Lifecycle manager
export { PlaywrightMCPLifecycle, } from './playwright-lifecycle.js';
// React context + hooks
export { PlaywrightMCPProvider, usePlaywrightMCP, usePlaywrightMCPReady, } from './playwright-context.js';
// Tool definitions (for sidebar display)
export { PLAYWRIGHT_TOOLS, PLAYWRIGHT_TOOL_COUNT, getToolsByCategory, } from './playwright-tools.js';
// Installation check
export { checkPlaywrightInstalled, } from './check-playwright.js';
//# sourceMappingURL=index.js.map