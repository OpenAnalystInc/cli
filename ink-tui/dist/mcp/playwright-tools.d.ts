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
export interface MCPToolDefinition {
    /** Tool name as registered in the MCP server. */
    name: string;
    /** Human-readable description. */
    description: string;
    /** Category for sidebar grouping. */
    category: 'navigation' | 'interaction' | 'extraction' | 'utility';
}
export declare const PLAYWRIGHT_TOOLS: MCPToolDefinition[];
/** Get tools by category for sidebar display. */
export declare function getToolsByCategory(category: MCPToolDefinition['category']): MCPToolDefinition[];
/** Total number of Playwright MCP tools. */
export declare const PLAYWRIGHT_TOOL_COUNT: number;
