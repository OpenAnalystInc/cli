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
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
// ---------------------------------------------------------------------------
// .env parser
// ---------------------------------------------------------------------------
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
export function parseEnvFile(filePath) {
    const result = {};
    let content;
    try {
        content = fs.readFileSync(filePath, 'utf-8');
    }
    catch {
        // File doesn't exist or is unreadable
        return result;
    }
    for (const rawLine of content.split('\n')) {
        const line = rawLine.trim();
        // Skip empty lines and comments
        if (!line || line.startsWith('#'))
            continue;
        // Find the first '=' separator
        const eqIndex = line.indexOf('=');
        if (eqIndex === -1)
            continue;
        const key = line.slice(0, eqIndex).trim();
        let value = line.slice(eqIndex + 1).trim();
        // Strip surrounding quotes
        if ((value.startsWith('"') && value.endsWith('"')) ||
            (value.startsWith("'") && value.endsWith("'"))) {
            value = value.slice(1, -1);
        }
        else {
            // Remove inline comments for unquoted values
            const hashIndex = value.indexOf(' #');
            if (hashIndex !== -1) {
                value = value.slice(0, hashIndex).trim();
            }
        }
        if (key) {
            result[key] = value;
        }
    }
    return result;
}
// ---------------------------------------------------------------------------
// Key finder
// ---------------------------------------------------------------------------
const ENV_KEY_NAME = 'OPENAI_API_KEY';
/**
 * Find OPENAI_API_KEY with priority:
 *   1. Project folder .env (process.cwd()/.env)
 *   2. Global fallback (~/.openanalyst/.env)
 *   3. Environment variable (process.env.OPENAI_API_KEY)
 *
 * Returns the key and where it was found, or { key: null, source: null }.
 */
export function findOpenAIKey() {
    // 1. Project-local .env
    const projectEnvPath = path.join(process.cwd(), '.env');
    const projectEnv = parseEnvFile(projectEnvPath);
    if (projectEnv[ENV_KEY_NAME]) {
        return { key: projectEnv[ENV_KEY_NAME], source: 'project' };
    }
    // 2. Global ~/.openanalyst/.env
    const globalEnvPath = path.join(os.homedir(), '.openanalyst', '.env');
    const globalEnv = parseEnvFile(globalEnvPath);
    if (globalEnv[ENV_KEY_NAME]) {
        return { key: globalEnv[ENV_KEY_NAME], source: 'global' };
    }
    // 3. Environment variable
    const envVar = process.env[ENV_KEY_NAME];
    if (envVar) {
        return { key: envVar, source: 'env' };
    }
    return { key: null, source: null };
}
//# sourceMappingURL=api-key.js.map