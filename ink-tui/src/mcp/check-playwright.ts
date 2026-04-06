/**
 * Check if Playwright browsers are installed.
 *
 * This is a quick check that can be called before starting the MCP server.
 * If browsers aren't installed, it returns a helpful message instead of
 * letting the server fail with a cryptic error.
 */

import { execSync } from 'node:child_process';
import { createRequire } from 'node:module';

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
export function checkPlaywrightInstalled(): PlaywrightCheckResult {
  try {
    // Use createRequire for ESM compatibility — require.resolve is not
    // available natively in ESM modules (package.json "type": "module").
    const require = createRequire(import.meta.url);
    require.resolve('@playwright/mcp');
  } catch {
    return {
      available: false,
      message: '@playwright/mcp package not found. Run: npm install @playwright/mcp',
    };
  }

  try {
    // Check if chromium browser is installed by running playwright's browser check
    const output = execSync('npx playwright install --dry-run chromium 2>&1', {
      encoding: 'utf-8',
      timeout: 10_000,
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    // If dry-run shows nothing to install, browsers are already there
    if (output.includes('already installed') || output.trim() === '') {
      return {
        available: true,
        message: 'Playwright chromium browser is installed and ready.',
      };
    }

    return {
      available: false,
      message: 'Playwright chromium browser not installed. Run: npx playwright install chromium',
    };
  } catch {
    // If the check itself fails, assume browsers might be installed
    // and let the MCP server attempt to launch (it will report the real error)
    return {
      available: true,
      message: 'Playwright installation status unknown. MCP server will attempt to start.',
    };
  }
}
