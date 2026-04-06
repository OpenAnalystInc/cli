/**
 * Playwright MCP tool definitions.
 *
 * These describe the tools the @playwright/mcp server exposes via MCP protocol.
 * The TUI displays these in the sidebar's MCP section so the user knows what
 * browser automation capabilities are available.
 *
 * Note: The actual tool implementations live in the @playwright/mcp package.
 * These definitions are for display and documentation purposes only.
 */

// ---------------------------------------------------------------------------
// Tool definition type
// ---------------------------------------------------------------------------

export interface MCPToolDefinition {
  /** Tool name as registered in the MCP server. */
  name: string;
  /** Human-readable description. */
  description: string;
  /** Category for sidebar grouping. */
  category: 'navigation' | 'interaction' | 'extraction' | 'utility';
}

// ---------------------------------------------------------------------------
// Playwright MCP tool catalog
// ---------------------------------------------------------------------------

export const PLAYWRIGHT_TOOLS: MCPToolDefinition[] = [
  // Navigation
  {
    name: 'browser_navigate',
    description: 'Navigate to a URL in the browser',
    category: 'navigation',
  },
  {
    name: 'browser_go_back',
    description: 'Navigate back in browser history',
    category: 'navigation',
  },
  {
    name: 'browser_go_forward',
    description: 'Navigate forward in browser history',
    category: 'navigation',
  },
  {
    name: 'browser_wait_for_navigation',
    description: 'Wait for page navigation to complete',
    category: 'navigation',
  },

  // Interaction
  {
    name: 'browser_click',
    description: 'Click an element on the page',
    category: 'interaction',
  },
  {
    name: 'browser_fill',
    description: 'Fill a form input with text',
    category: 'interaction',
  },
  {
    name: 'browser_select_option',
    description: 'Select an option from a dropdown',
    category: 'interaction',
  },
  {
    name: 'browser_hover',
    description: 'Hover over an element',
    category: 'interaction',
  },
  {
    name: 'browser_press_key',
    description: 'Press a keyboard key',
    category: 'interaction',
  },
  {
    name: 'browser_drag',
    description: 'Drag an element to another location',
    category: 'interaction',
  },
  {
    name: 'browser_scroll',
    description: 'Scroll the page or an element',
    category: 'interaction',
  },

  // Extraction
  {
    name: 'browser_snapshot',
    description: 'Capture accessibility snapshot of the page',
    category: 'extraction',
  },
  {
    name: 'browser_screenshot',
    description: 'Take a screenshot of the current page',
    category: 'extraction',
  },
  {
    name: 'browser_get_text',
    description: 'Extract text content from an element',
    category: 'extraction',
  },
  {
    name: 'browser_evaluate',
    description: 'Execute JavaScript in the browser context',
    category: 'extraction',
  },
  {
    name: 'browser_pdf',
    description: 'Save the current page as PDF',
    category: 'extraction',
  },

  // Utility
  {
    name: 'browser_wait',
    description: 'Wait for an element to appear',
    category: 'utility',
  },
  {
    name: 'browser_resize',
    description: 'Resize the browser viewport',
    category: 'utility',
  },
  {
    name: 'browser_close',
    description: 'Close the current browser tab',
    category: 'utility',
  },
  {
    name: 'browser_new_tab',
    description: 'Open a new browser tab',
    category: 'utility',
  },
  {
    name: 'browser_switch_tab',
    description: 'Switch to a different browser tab',
    category: 'utility',
  },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Get tools by category for sidebar display. */
export function getToolsByCategory(category: MCPToolDefinition['category']): MCPToolDefinition[] {
  return PLAYWRIGHT_TOOLS.filter((t) => t.category === category);
}

/** Total number of Playwright MCP tools. */
export const PLAYWRIGHT_TOOL_COUNT = PLAYWRIGHT_TOOLS.length;
