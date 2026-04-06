/**
 * api-key — Detect OPENAI_API_KEY from environment and .env files.
 *
 * Priority chain:
 *   1. Project-local .env (process.cwd()/.env)
 *   2. Global fallback (~/.openanalyst/.env)
 *   3. Environment variable (process.env.OPENAI_API_KEY)
 *
 * Uses sync I/O — intended for a one-time check on startup or when
 * voice recording is first requested.
 */
export interface ApiKeyResult {
    /** The API key value, or null if not found. */
    key: string | null;
    /** Where the key was found: 'project' (.env in cwd), 'global' (~/.openanalyst/.env),
     *  'env' (process.env), or null if not found anywhere. */
    source: 'project' | 'global' | 'env' | null;
}
/**
 * Parse a .env file and return key-value pairs.
 *
 * Handles:
 *   - KEY=VALUE
 *   - KEY="VALUE" (double-quoted, strips quotes)
 *   - KEY='VALUE' (single-quoted, strips quotes)
 *   - Comments (lines starting with #)
 *   - Empty lines
 *   - Inline comments after unquoted values
 */
export declare function parseEnvFile(filePath: string): Record<string, string>;
/**
 * Find OPENAI_API_KEY with priority:
 *   1. Project folder .env (process.cwd()/.env)
 *   2. Global fallback (~/.openanalyst/.env)
 *   3. Environment variable (process.env.OPENAI_API_KEY)
 *
 * Returns the key and where it was found, or { key: null, source: null }.
 */
export declare function findOpenAIKey(): ApiKeyResult;
