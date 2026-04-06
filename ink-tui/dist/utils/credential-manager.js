/**
 * Unified credential manager for OpenAnalyst CLI.
 *
 * Stores and retrieves API keys from multiple locations with a priority chain:
 *   1. Project .env (process.cwd()/.env) -- highest priority
 *   2. Global .env (~/.openanalyst/.env) -- fallback
 *   3. SQLite database (~/.openanalyst/credentials.db) -- persistent store
 *   4. Environment variables -- lowest priority
 *
 * When user does /login:
 *   - API key is saved to ALL locations (global .env + SQLite + credentials.json)
 *   - This ensures the key persists across sessions and is accessible everywhere
 *
 * SQLite uses sql.js (pure JS, no native compilation) for maximum portability.
 * If SQLite is unavailable, gracefully falls back to .env + credentials.json only.
 */
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import https from 'node:https';
import { parseEnvFile } from './api-key.js';
// ---------------------------------------------------------------------------
// Provider configuration
// ---------------------------------------------------------------------------
export const PROVIDER_CONFIG = {
    openai: {
        envVar: 'OPENAI_API_KEY',
        displayName: 'OpenAI',
        creditEndpoint: 'https://api.openai.com/dashboard/billing/credit_grants',
        creditAuthHeader: 'bearer',
        creditParser: (r) => {
            const data = r;
            return typeof data.total_available === 'number'
                ? `$${data.total_available.toFixed(2)}`
                : 'Connected';
        },
    },
    anthropic: {
        envVar: 'ANTHROPIC_API_KEY',
        displayName: 'Anthropic',
        creditEndpoint: null,
        creditAuthHeader: 'x-api-key',
        creditParser: () => 'Connected',
    },
    gemini: {
        envVar: 'GEMINI_API_KEY',
        displayName: 'Google Gemini',
        creditEndpoint: null,
        creditAuthHeader: 'bearer',
        creditParser: () => 'Connected',
    },
    xai: {
        envVar: 'XAI_API_KEY',
        displayName: 'xAI',
        creditEndpoint: null,
        creditAuthHeader: 'bearer',
        creditParser: () => 'Connected',
    },
    openrouter: {
        envVar: 'OPENROUTER_API_KEY',
        displayName: 'OpenRouter',
        creditEndpoint: 'https://openrouter.ai/api/v1/auth/key',
        creditAuthHeader: 'bearer',
        creditParser: (r) => {
            const data = r;
            return typeof data.data?.limit_remaining === 'number'
                ? `$${data.data.limit_remaining.toFixed(2)}`
                : 'Connected';
        },
    },
    bedrock: {
        envVar: 'BEDROCK_API_KEY',
        displayName: 'Amazon Bedrock',
        creditEndpoint: null,
        creditAuthHeader: 'bearer',
        creditParser: () => 'Connected',
    },
    stability: {
        envVar: 'STABILITY_API_KEY',
        displayName: 'Stability AI',
        creditEndpoint: 'https://api.stability.ai/v1/user/balance',
        creditAuthHeader: 'bearer',
        creditParser: (r) => {
            const data = r;
            return typeof data.credits === 'number'
                ? `${data.credits.toFixed(1)} credits`
                : 'Connected';
        },
    },
    openanalyst: {
        envVar: 'OPENANALYST_AUTH_TOKEN',
        displayName: 'OpenAnalyst',
        creditEndpoint: null,
        creditAuthHeader: 'bearer',
        creditParser: () => 'Connected',
    },
};
let _sqlDb = null;
let _sqlInitAttempted = false;
function getDbPath() {
    const configHome = process.env['OPENANALYST_CONFIG_HOME']
        ?? path.join(os.homedir(), '.openanalyst');
    return path.join(configHome, 'credentials.db');
}
/**
 * Attempt to open or create the SQLite database.
 * Returns null if sql.js is not available or the database cannot be opened.
 * This is a best-effort operation -- the credential manager works without SQLite.
 */
async function getSqlDb() {
    if (_sqlDb)
        return _sqlDb;
    if (_sqlInitAttempted)
        return null;
    _sqlInitAttempted = true;
    try {
        // Dynamic import so it fails gracefully if sql.js isn't installed
        const initSqlJs = (await import('sql.js')).default;
        const SQL = await initSqlJs();
        const dbPath = getDbPath();
        const dbDir = path.dirname(dbPath);
        // Ensure directory exists
        try {
            fs.mkdirSync(dbDir, { recursive: true });
        }
        catch { /* ignore */ }
        // Load existing database or create new
        let db;
        if (fs.existsSync(dbPath)) {
            const buffer = fs.readFileSync(dbPath);
            db = new SQL.Database(buffer);
        }
        else {
            db = new SQL.Database();
        }
        // Create table if it doesn't exist
        db.run(`
      CREATE TABLE IF NOT EXISTS credentials (
        provider TEXT PRIMARY KEY,
        api_key TEXT NOT NULL,
        env_var_name TEXT NOT NULL,
        saved_at INTEGER NOT NULL,
        source TEXT NOT NULL DEFAULT 'login'
      )
    `);
        // Persist the initial creation
        const data = db.export();
        fs.writeFileSync(dbPath, Buffer.from(data));
        _sqlDb = db;
        return db;
    }
    catch {
        // sql.js not available or database error -- fall back silently
        return null;
    }
}
function persistSqlDb() {
    if (!_sqlDb)
        return;
    try {
        const data = _sqlDb.export();
        fs.writeFileSync(getDbPath(), Buffer.from(data));
    }
    catch {
        // Ignore write errors -- .env and credentials.json are the primary stores
    }
}
// ---------------------------------------------------------------------------
// HTTPS helper
// ---------------------------------------------------------------------------
function httpsRequest(url, headers) {
    return new Promise((resolve, reject) => {
        const parsedUrl = new URL(url);
        const request = https.get({
            hostname: parsedUrl.hostname,
            path: parsedUrl.pathname + parsedUrl.search,
            headers: { 'User-Agent': 'openanalyst-cli', ...headers },
        }, (res) => {
            const chunks = [];
            res.on('data', (chunk) => chunks.push(chunk));
            res.on('end', () => {
                resolve({
                    status: res.statusCode ?? 0,
                    body: Buffer.concat(chunks).toString('utf-8'),
                });
            });
            res.on('error', reject);
        });
        request.on('error', reject);
        request.setTimeout(10_000, () => {
            request.destroy();
            reject(new Error('Request timed out'));
        });
    });
}
// ---------------------------------------------------------------------------
// CredentialManager
// ---------------------------------------------------------------------------
class CredentialManager {
    globalEnvPath;
    credJsonPath;
    constructor() {
        const configHome = process.env['OPENANALYST_CONFIG_HOME']
            ?? path.join(os.homedir(), '.openanalyst');
        this.globalEnvPath = path.join(configHome, '.env');
        this.credJsonPath = path.join(configHome, 'credentials.json');
    }
    // ── Save ────────────────────────────────────────────────────────────────
    /**
     * Save an API key for a provider to ALL storage locations:
     *   - Global .env file
     *   - SQLite database (if available)
     *   - credentials.json
     */
    async saveApiKey(provider, apiKey) {
        const config = PROVIDER_CONFIG[provider];
        if (!config) {
            throw new Error(`Unknown provider: ${provider}`);
        }
        const timestamp = Date.now();
        // 1. Save to global .env
        this.upsertEnvKey(this.globalEnvPath, config.envVar, apiKey);
        // 2. Save to credentials.json
        this.saveToCredJson(provider, config, apiKey);
        // 3. Save to SQLite (best-effort)
        try {
            const db = await getSqlDb();
            if (db) {
                db.run(`INSERT OR REPLACE INTO credentials (provider, api_key, env_var_name, saved_at, source)
           VALUES (?, ?, ?, ?, ?)`, [provider, apiKey, config.envVar, timestamp, 'login']);
                persistSqlDb();
            }
        }
        catch {
            // SQLite unavailable -- .env and credentials.json are sufficient
        }
        // 4. Set in current process environment
        process.env[config.envVar] = apiKey;
    }
    // ── Read ────────────────────────────────────────────────────────────────
    /**
     * Get the API key for a provider with priority chain:
     *   1. Project .env (process.cwd()/.env)
     *   2. Global .env (~/.openanalyst/.env)
     *   3. SQLite database
     *   4. Environment variable
     */
    getApiKey(provider) {
        const config = PROVIDER_CONFIG[provider];
        if (!config)
            return { key: null, source: null };
        // 1. Project .env
        const projectEnvPath = path.join(process.cwd(), '.env');
        const projectEnv = parseEnvFile(projectEnvPath);
        if (projectEnv[config.envVar]) {
            return { key: projectEnv[config.envVar], source: 'project' };
        }
        // 2. Global .env
        const globalEnv = parseEnvFile(this.globalEnvPath);
        if (globalEnv[config.envVar]) {
            return { key: globalEnv[config.envVar], source: 'global' };
        }
        // 3. SQLite (sync check -- database was loaded asynchronously)
        if (_sqlDb) {
            try {
                const rows = _sqlDb.exec('SELECT api_key FROM credentials WHERE provider = ?', [provider]);
                if (rows.length > 0 && rows[0].values.length > 0) {
                    const key = rows[0].values[0][0];
                    if (key)
                        return { key, source: 'sqlite' };
                }
            }
            catch {
                // Ignore SQLite errors
            }
        }
        // 4. Environment variable
        const envVal = process.env[config.envVar];
        if (envVal) {
            return { key: envVal, source: 'env' };
        }
        return { key: null, source: null };
    }
    /**
     * Get the API key for a provider by env var name.
     * Useful when you know the env var but not the provider key.
     */
    getApiKeyByEnvVar(envVar) {
        const provider = Object.entries(PROVIDER_CONFIG).find(([, cfg]) => cfg.envVar === envVar);
        if (!provider)
            return { key: null, source: null };
        return this.getApiKey(provider[0]);
    }
    // ── List ────────────────────────────────────────────────────────────────
    /**
     * Get all saved credentials from all storage locations.
     * Merges results with the priority chain (project > global > sqlite > env).
     */
    listCredentials() {
        const results = [];
        for (const [providerKey, config] of Object.entries(PROVIDER_CONFIG)) {
            const resolved = this.getApiKey(providerKey);
            if (resolved.key) {
                results.push({
                    provider: providerKey,
                    apiKey: resolved.key,
                    envVarName: config.envVar,
                    savedAt: Date.now(),
                    source: resolved.source === 'env' ? 'env' : 'login',
                });
            }
        }
        return results;
    }
    // ── Remove ──────────────────────────────────────────────────────────────
    /**
     * Remove a provider's credentials from all locations.
     */
    async removeCredential(provider) {
        const config = PROVIDER_CONFIG[provider];
        if (!config)
            return;
        // Remove from global .env
        this.removeEnvKey(this.globalEnvPath, config.envVar);
        // Remove from credentials.json
        this.removeFromCredJson(provider);
        // Remove from SQLite
        try {
            const db = await getSqlDb();
            if (db) {
                db.run('DELETE FROM credentials WHERE provider = ?', [provider]);
                persistSqlDb();
            }
        }
        catch {
            // Ignore
        }
        // Remove from process env
        delete process.env[config.envVar];
    }
    /**
     * Remove ALL credentials except Gemini OAuth tokens.
     */
    async removeAll() {
        for (const providerKey of Object.keys(PROVIDER_CONFIG)) {
            if (providerKey === 'gemini')
                continue; // Keep Gemini OAuth
            await this.removeCredential(providerKey);
        }
    }
    // ── Credits ─────────────────────────────────────────────────────────────
    /**
     * Fetch credit balance for a specific provider.
     */
    async fetchCredits(provider) {
        const config = PROVIDER_CONFIG[provider];
        if (!config)
            return 'Unknown provider';
        const resolved = this.getApiKey(provider);
        if (!resolved.key)
            return 'No API key';
        if (!config.creditEndpoint) {
            // No billing API -- validate key instead
            return this.validateKey(provider, resolved.key);
        }
        try {
            const headers = config.creditAuthHeader === 'x-api-key'
                ? { 'x-api-key': resolved.key, 'anthropic-version': '2023-06-01' }
                : { Authorization: `Bearer ${resolved.key}` };
            const res = await httpsRequest(config.creditEndpoint, headers);
            if (res.status === 200) {
                const data = JSON.parse(res.body);
                return config.creditParser(data);
            }
            if (res.status === 401 || res.status === 403) {
                return 'Invalid key';
            }
            // Endpoint available but billing info not returned -- key is valid
            return `${config.displayName} connected`;
        }
        catch {
            // Network error -- key might still be valid
            return `${config.displayName} API key`;
        }
    }
    /**
     * Fetch credits for ALL configured providers.
     */
    async fetchAllCredits() {
        const results = {};
        const providers = this.listCredentials();
        const promises = providers.map(async (cred) => {
            try {
                results[cred.provider] = await this.fetchCredits(cred.provider);
            }
            catch {
                results[cred.provider] = 'Error';
            }
        });
        await Promise.all(promises);
        return results;
    }
    // ── Detection ───────────────────────────────────────────────────────────
    /**
     * Detect which provider a key belongs to by prefix.
     */
    detectProvider(apiKey) {
        if (apiKey.startsWith('sk-ant-'))
            return 'anthropic';
        if (apiKey.startsWith('sk-or-'))
            return 'openrouter';
        if (apiKey.startsWith('sk-oa-'))
            return 'openanalyst';
        if (apiKey.startsWith('sk-'))
            return 'openai';
        if (apiKey.startsWith('AIza'))
            return 'gemini';
        if (apiKey.startsWith('xai-'))
            return 'xai';
        return null;
    }
    // ── Initialize ──────────────────────────────────────────────────────────
    /**
     * Initialize the SQLite database asynchronously.
     * Call this once during app startup so that getApiKey() can use SQLite synchronously.
     */
    async initialize() {
        await getSqlDb();
    }
    // ── Private helpers ─────────────────────────────────────────────────────
    upsertEnvKey(envPath, key, value) {
        try {
            const dir = path.dirname(envPath);
            fs.mkdirSync(dir, { recursive: true });
            const content = fs.existsSync(envPath)
                ? fs.readFileSync(envPath, 'utf-8')
                : '';
            const newLine = `${key}=${value}`;
            let found = false;
            const lines = content.split('\n').map((line) => {
                const trimmed = line.trim();
                const bare = trimmed.replace(/^#+\s*/, '');
                if (bare.startsWith(`${key}=`)) {
                    found = true;
                    return newLine;
                }
                return line;
            });
            if (!found) {
                if (lines.length > 0 && lines[lines.length - 1].trim() !== '') {
                    lines.push('');
                }
                lines.push(newLine);
            }
            fs.writeFileSync(envPath, lines.join('\n'));
        }
        catch {
            // Ignore write errors
        }
    }
    removeEnvKey(envPath, key) {
        try {
            if (!fs.existsSync(envPath))
                return;
            const content = fs.readFileSync(envPath, 'utf-8');
            const lines = content.split('\n').filter((line) => {
                const bare = line.trim().replace(/^#+\s*/, '');
                return !bare.startsWith(`${key}=`);
            });
            fs.writeFileSync(envPath, lines.join('\n'));
        }
        catch {
            // Ignore
        }
    }
    saveToCredJson(provider, config, apiKey) {
        try {
            const dir = path.dirname(this.credJsonPath);
            fs.mkdirSync(dir, { recursive: true });
            let creds = {};
            if (fs.existsSync(this.credJsonPath)) {
                try {
                    creds = JSON.parse(fs.readFileSync(this.credJsonPath, 'utf-8'));
                }
                catch {
                    creds = {};
                }
            }
            creds['active_provider'] = config.displayName;
            if (!creds['providers'] || typeof creds['providers'] !== 'object') {
                creds['providers'] = {};
            }
            creds['providers'][config.displayName] = {
                env_var: config.envVar,
                api_key: apiKey,
            };
            fs.writeFileSync(this.credJsonPath, JSON.stringify(creds, null, 2) + '\n');
        }
        catch {
            // Ignore write errors
        }
    }
    removeFromCredJson(provider) {
        try {
            if (!fs.existsSync(this.credJsonPath))
                return;
            const creds = JSON.parse(fs.readFileSync(this.credJsonPath, 'utf-8'));
            const config = PROVIDER_CONFIG[provider];
            if (!config)
                return;
            const providers = creds['providers'];
            if (providers) {
                delete providers[config.displayName];
            }
            if (creds['active_provider'] === config.displayName) {
                delete creds['active_provider'];
            }
            fs.writeFileSync(this.credJsonPath, JSON.stringify(creds, null, 2) + '\n');
        }
        catch {
            // Ignore
        }
    }
    async validateKey(provider, apiKey) {
        const config = PROVIDER_CONFIG[provider];
        if (!config)
            return 'Unknown';
        // Use a lightweight endpoint to validate the key
        const validationUrls = {
            openai: 'https://api.openai.com/v1/models',
            anthropic: 'https://api.anthropic.com/v1/messages?limit=0',
            gemini: 'https://generativelanguage.googleapis.com/v1beta/openai/models',
            xai: 'https://api.x.ai/v1/models',
            openrouter: 'https://openrouter.ai/api/v1/models',
        };
        const url = validationUrls[provider];
        if (!url)
            return `${config.displayName} configured`;
        try {
            const headers = config.creditAuthHeader === 'x-api-key'
                ? {
                    'x-api-key': apiKey,
                    'anthropic-version': '2023-06-01',
                }
                : { Authorization: `Bearer ${apiKey}` };
            const res = await httpsRequest(url, headers);
            if (res.status === 401 || res.status === 403) {
                return 'Invalid key';
            }
            return `${config.displayName} connected`;
        }
        catch {
            return `${config.displayName} API key`;
        }
    }
}
// ---------------------------------------------------------------------------
// Singleton export
// ---------------------------------------------------------------------------
export const credentialManager = new CredentialManager();
//# sourceMappingURL=credential-manager.js.map