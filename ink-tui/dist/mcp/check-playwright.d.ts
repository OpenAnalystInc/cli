/**
 * Check if Playwright browsers are installed.
 *
 * This is a quick check that can be called before starting the MCP server.
 * If browsers aren't installed, it returns a helpful message instead of
 * letting the server fail with a cryptic error.
 */
export interface PlaywrightCheckResult {
    /** Whether Playwright browsers are available. */
    available: boolean;
    /** Human-readable status message. */
    message: string;
    /** The path to the chromium executable, if found. */
    chromiumPath?: string;
}
/**
 * Check if Playwright chromium browser is installed.
 * This is a synchronous check for startup speed.
 */
export declare function checkPlaywrightInstalled(): PlaywrightCheckResult;
