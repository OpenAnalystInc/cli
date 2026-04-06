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
export interface ProviderCredential {
    provider: string;
    apiKey: string;
    envVarName: string;
    savedAt: number;
    source: 'login' | 'env' | 'manual';
}
export interface ProviderConfig {
    envVar: string;
    displayName: string;
    creditEndpoint: string | null;
    creditAuthHeader: 'bearer' | 'x-api-key';
    creditParser: (response: unknown) => string;
}
export interface ResolvedKey {
    key: string | null;
    source: 'project' | 'global' | 'sqlite' | 'env' | null;
}
export declare const PROVIDER_CONFIG: Record<string, ProviderConfig>;
declare class CredentialManager {
    private globalEnvPath;
    private credJsonPath;
    constructor();
    /**
     * Save an API key for a provider to ALL storage locations:
     *   - Global .env file
     *   - SQLite database (if available)
     *   - credentials.json
     */
    saveApiKey(provider: string, apiKey: string): Promise<void>;
    /**
     * Get the API key for a provider with priority chain:
     *   1. Project .env (process.cwd()/.env)
     *   2. Global .env (~/.openanalyst/.env)
     *   3. SQLite database
     *   4. Environment variable
     */
    getApiKey(provider: string): ResolvedKey;
    /**
     * Get the API key for a provider by env var name.
     * Useful when you know the env var but not the provider key.
     */
    getApiKeyByEnvVar(envVar: string): ResolvedKey;
    /**
     * Get all saved credentials from all storage locations.
     * Merges results with the priority chain (project > global > sqlite > env).
     */
    listCredentials(): ProviderCredential[];
    /**
     * Remove a provider's credentials from all locations.
     */
    removeCredential(provider: string): Promise<void>;
    /**
     * Remove ALL credentials except Gemini OAuth tokens.
     */
    removeAll(): Promise<void>;
    /**
     * Fetch credit balance for a specific provider.
     */
    fetchCredits(provider: string): Promise<string>;
    /**
     * Fetch credits for ALL configured providers.
     */
    fetchAllCredits(): Promise<Record<string, string>>;
    /**
     * Detect which provider a key belongs to by prefix.
     */
    detectProvider(apiKey: string): string | null;
    /**
     * Initialize the SQLite database asynchronously.
     * Call this once during app startup so that getApiKey() can use SQLite synchronously.
     */
    initialize(): Promise<void>;
    private upsertEnvKey;
    private removeEnvKey;
    private saveToCredJson;
    private removeFromCredJson;
    private validateKey;
}
export declare const credentialManager: CredentialManager;
export type { CredentialManager };
