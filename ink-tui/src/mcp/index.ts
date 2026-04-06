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
export {
  PlaywrightMCPLifecycle,
  type PlaywrightMCPConfig,
  type PlaywrightMCPState,
  type PlaywrightMCPEvents,
} from './playwright-lifecycle.js';

// React context + hooks
export {
  PlaywrightMCPProvider,
  usePlaywrightMCP,
  usePlaywrightMCPReady,
  type PlaywrightMCPContextValue,
  type PlaywrightMCPProviderProps,
} from './playwright-context.js';

// Tool definitions (for sidebar display)
export {
  PLAYWRIGHT_TOOLS,
  PLAYWRIGHT_TOOL_COUNT,
  getToolsByCategory,
  type MCPToolDefinition,
} from './playwright-tools.js';

// Installation check
export {
  checkPlaywrightInstalled,
  type PlaywrightCheckResult,
} from './check-playwright.js';
