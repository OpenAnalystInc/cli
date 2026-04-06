/**
 * Provider preference manager for OpenAnalyst CLI.
 *
 * Manages the user's AI provider preferences:
 *   - Persists the user's default provider across sessions
 *   - Tracks which providers have valid API keys
 *   - Filters available models based on configured providers
 *   - Stores preferences in ~/.openanalyst/preferences.json
 *
 * The model catalog mirrors the Rust engine's MODEL_REGISTRY in
 * `rust/crates/api/src/providers/mod.rs` so the TUI can show
 * model availability without querying the engine.
 */
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { credentialManager, PROVIDER_CONFIG } from './credential-manager.js';
// ---------------------------------------------------------------------------
// Model catalog (mirrors Rust MODEL_REGISTRY)
// ---------------------------------------------------------------------------
const MODEL_CATALOG = [
    // ── OpenAI (April 2026) ──
    { id: 'gpt-5.4', name: 'GPT-5.4', provider: 'openai', aliases: ['5.4', 'gpt5'], tier: 'capable', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'gpt-5.4-mini', name: 'GPT-5.4 Mini', provider: 'openai', aliases: ['5.4-mini'], tier: 'balanced', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'gpt-5.4-nano', name: 'GPT-5.4 Nano', provider: 'openai', aliases: ['5.4-nano'], tier: 'fast', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'gpt-4o', name: 'GPT-4o', provider: 'openai', aliases: ['4o'], tier: 'capable', contextWindow: 128_000, supportsVision: true, supportsTools: true },
    { id: 'gpt-4o-mini', name: 'GPT-4o Mini', provider: 'openai', aliases: ['4o-mini', 'mini'], tier: 'fast', contextWindow: 128_000, supportsVision: true, supportsTools: true },
    { id: 'gpt-4.1', name: 'GPT-4.1', provider: 'openai', aliases: ['4.1'], tier: 'capable', contextWindow: 1_047_576, supportsVision: true, supportsTools: true },
    { id: 'gpt-4.1-mini', name: 'GPT-4.1 Mini', provider: 'openai', aliases: ['4.1-mini'], tier: 'balanced', contextWindow: 1_047_576, supportsVision: true, supportsTools: true },
    { id: 'gpt-4.1-nano', name: 'GPT-4.1 Nano', provider: 'openai', aliases: ['4.1-nano', 'nano'], tier: 'fast', contextWindow: 1_047_576, supportsVision: true, supportsTools: true },
    { id: 'o3', name: 'o3', provider: 'openai', aliases: [], tier: 'capable', contextWindow: 200_000, supportsVision: false, supportsTools: true },
    { id: 'o3-pro', name: 'o3 Pro', provider: 'openai', aliases: [], tier: 'capable', contextWindow: 200_000, supportsVision: false, supportsTools: true },
    { id: 'o3-mini', name: 'o3 Mini', provider: 'openai', aliases: [], tier: 'balanced', contextWindow: 200_000, supportsVision: false, supportsTools: true },
    { id: 'o4-mini', name: 'o4 Mini', provider: 'openai', aliases: [], tier: 'balanced', contextWindow: 200_000, supportsVision: true, supportsTools: true },
    { id: 'codex-mini', name: 'Codex Mini', provider: 'openai', aliases: ['codex'], tier: 'fast', contextWindow: 200_000, supportsVision: false, supportsTools: true },
    // ── Anthropic (Claude — April 2026) ──
    { id: 'claude-opus-4-6', name: 'Opus 4.6', provider: 'anthropic', aliases: ['opus', 'opus-4.6'], tier: 'capable', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'claude-sonnet-4-6', name: 'Sonnet 4.6', provider: 'anthropic', aliases: ['sonnet', 'sonnet-4.6'], tier: 'balanced', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'claude-haiku-4-5-20251213', name: 'Haiku 4.5', provider: 'anthropic', aliases: ['haiku'], tier: 'fast', contextWindow: 200_000, supportsVision: true, supportsTools: true },
    // ── Google Gemini (April 2026) ──
    { id: 'gemini-3.1-pro', name: 'Gemini 3.1 Pro', provider: 'gemini', aliases: ['gemini-pro', 'gemini'], tier: 'capable', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'gemini-3-flash', name: 'Gemini 3 Flash', provider: 'gemini', aliases: ['gemini-3-flash'], tier: 'balanced', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'gemini-3.1-flash-lite', name: 'Gemini 3.1 Flash-Lite', provider: 'gemini', aliases: ['gemini-lite'], tier: 'fast', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'gemini-2.5-pro', name: 'Gemini 2.5 Pro', provider: 'gemini', aliases: [], tier: 'capable', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'gemini-2.5-flash', name: 'Gemini 2.5 Flash', provider: 'gemini', aliases: ['gemini-flash', 'flash'], tier: 'fast', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    // ── xAI Grok (April 2026) ──
    { id: 'grok-4', name: 'Grok 4', provider: 'xai', aliases: ['grok'], tier: 'capable', contextWindow: 2_000_000, supportsVision: true, supportsTools: true },
    { id: 'grok-4-fast', name: 'Grok 4 Fast', provider: 'xai', aliases: ['grok-fast'], tier: 'fast', contextWindow: 2_000_000, supportsVision: true, supportsTools: true },
    { id: 'grok-3', name: 'Grok 3', provider: 'xai', aliases: [], tier: 'balanced', contextWindow: 131_072, supportsVision: false, supportsTools: true },
    // ── DeepSeek (March 2026) ──
    { id: 'deepseek-v4', name: 'DeepSeek V4', provider: 'openrouter', aliases: ['deepseek', 'ds-v4'], tier: 'capable', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    { id: 'deepseek-r2', name: 'DeepSeek R2', provider: 'openrouter', aliases: ['ds-r2'], tier: 'balanced', contextWindow: 1_000_000, supportsVision: false, supportsTools: true },
    // ── Meta Llama 4 (via OpenRouter) ──
    { id: 'llama-4-maverick', name: 'Llama 4 Maverick', provider: 'openrouter', aliases: ['llama', 'maverick'], tier: 'balanced', contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
    // ── OpenRouter (meta-provider) ──
    { id: 'openrouter/auto', name: 'Auto (best available)', provider: 'openrouter', aliases: ['auto'], tier: 'balanced', contextWindow: 200_000, supportsVision: true, supportsTools: true },
    // ── Amazon Bedrock ──
    { id: 'bedrock/claude', name: 'Bedrock Claude', provider: 'bedrock', aliases: ['bedrock'], tier: 'capable', contextWindow: 200_000, supportsVision: true, supportsTools: true },
    // ── OpenAnalyst ──
    { id: 'openanalyst-beta', name: 'OpenAnalyst Beta', provider: 'openanalyst', aliases: ['oa-beta', 'default'], tier: 'balanced', contextWindow: 200_000, supportsVision: true, supportsTools: true },
];
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function getPrefsPath() {
    const configHome = process.env['OPENANALYST_CONFIG_HOME']
        ?? path.join(os.homedir(), '.openanalyst');
    return path.join(configHome, 'preferences.json');
}
function createDefaultPrefs() {
    return {
        defaultProvider: null,
        defaultSetAt: null,
        routing: {},
    };
}
function formatContextWindow(tokens) {
    if (tokens >= 1_000_000)
        return `${(tokens / 1_000_000).toFixed(0)}M context`;
    return `${Math.round(tokens / 1_000)}K context`;
}
// ---------------------------------------------------------------------------
// ProviderPreferenceManager
// ---------------------------------------------------------------------------
class ProviderPreferenceManager {
    prefsPath;
    _cache = null;
    constructor() {
        this.prefsPath = getPrefsPath();
    }
    // -- Default provider ---------------------------------------------------
    /** Get the user's default provider key (e.g. "anthropic"). */
    getDefaultProvider() {
        return this.load().defaultProvider;
    }
    /** Set the default provider (called during /login). */
    setDefaultProvider(provider) {
        const prefs = this.load();
        prefs.defaultProvider = provider;
        prefs.defaultSetAt = Date.now();
        this.save(prefs);
    }
    // -- Routing ------------------------------------------------------------
    /** Get routing for a specific action category. */
    getRouting(action) {
        const prefs = this.load();
        return prefs.routing[action] ?? null;
    }
    /** Set routing for an action category (from sidebar). */
    setRouting(action, provider, model, tier) {
        const prefs = this.load();
        prefs.routing[action] = { provider, model, tier };
        this.save(prefs);
    }
    // -- Model queries ------------------------------------------------------
    /** Get all models available based on configured API keys. */
    getAvailableModels() {
        const configured = new Set(this.getConfiguredProviders());
        return MODEL_CATALOG.filter((m) => configured.has(m.provider));
    }
    /** Get models for a specific provider. */
    getModelsForProvider(provider) {
        return MODEL_CATALOG.filter((m) => m.provider === provider);
    }
    /** Get all providers that have valid API keys. */
    getConfiguredProviders() {
        const providers = [];
        for (const providerKey of Object.keys(PROVIDER_CONFIG)) {
            const resolved = credentialManager.getApiKey(providerKey);
            if (resolved.key) {
                providers.push(providerKey);
            }
        }
        return providers;
    }
    /** Get the full model catalog (regardless of API key availability). */
    getFullCatalog() {
        return MODEL_CATALOG;
    }
    /** Resolve a model alias to a ModelInfo, or null if not found. */
    resolveAlias(alias) {
        const lower = alias.toLowerCase();
        return MODEL_CATALOG.find((m) => m.id.toLowerCase() === lower ||
            m.aliases.some((a) => a.toLowerCase() === lower)) ?? null;
    }
    /** Get the default model for a provider (first "balanced" tier, or first model). */
    getDefaultModelForProvider(provider) {
        const models = this.getModelsForProvider(provider);
        return models.find((m) => m.tier === 'balanced') ?? models[0] ?? null;
    }
    // -- Formatted output ---------------------------------------------------
    /** Format the /models output for display in the chat. */
    formatModelsOutput(currentModelId) {
        const configured = new Set(this.getConfiguredProviders());
        const defaultProvider = this.getDefaultProvider();
        const lines = [];
        lines.push('Available models (based on your API keys):');
        lines.push('');
        // Show default provider
        if (defaultProvider) {
            const config = PROVIDER_CONFIG[defaultProvider];
            const name = config?.displayName ?? defaultProvider;
            lines.push(`  \u2605 Default provider: ${name} (set during /login)`);
            lines.push('');
        }
        // Group models by provider -- configured first
        const allProviders = Object.keys(PROVIDER_CONFIG);
        const configuredProviders = allProviders.filter((p) => configured.has(p));
        const unconfiguredProviders = allProviders.filter((p) => !configured.has(p));
        for (const providerKey of configuredProviders) {
            const config = PROVIDER_CONFIG[providerKey];
            const models = this.getModelsForProvider(providerKey);
            if (models.length === 0)
                continue;
            lines.push(`  ${config.displayName} (${config.envVar} \u2713)`);
            for (const model of models) {
                const primaryAlias = model.aliases[0] ?? model.id;
                const aliasCol = primaryAlias.padEnd(16);
                const nameCol = model.name.padEnd(24);
                const tierCol = model.tier.padEnd(10);
                const ctxCol = formatContextWindow(model.contextWindow);
                const isCurrent = currentModelId && (model.id === currentModelId ||
                    model.aliases.some((a) => a === currentModelId));
                const marker = isCurrent ? '  \u2190 current' : '';
                lines.push(`    ${aliasCol}${nameCol}${tierCol}${ctxCol}${marker}`);
            }
            lines.push('');
        }
        // Show unconfigured providers
        if (unconfiguredProviders.length > 0) {
            lines.push('  \u2500\u2500\u2500 Not configured \u2500\u2500\u2500');
            for (const providerKey of unconfiguredProviders) {
                const config = PROVIDER_CONFIG[providerKey];
                const models = this.getModelsForProvider(providerKey);
                if (models.length === 0)
                    continue;
                lines.push(`  ${config.displayName} (${config.envVar} \u2717) \u2014 /login ${providerKey} <key> to add`);
            }
            lines.push('');
        }
        lines.push('Usage: /model <name> to switch (e.g., /model opus)');
        return lines.join('\n');
    }
    // -- Persistence --------------------------------------------------------
    /** Load preferences from disk (cached). */
    load() {
        if (this._cache)
            return this._cache;
        try {
            if (fs.existsSync(this.prefsPath)) {
                const raw = fs.readFileSync(this.prefsPath, 'utf-8');
                const parsed = JSON.parse(raw);
                this._cache = {
                    defaultProvider: parsed.defaultProvider ?? null,
                    defaultSetAt: parsed.defaultSetAt ?? null,
                    routing: parsed.routing ?? {},
                };
                return this._cache;
            }
        }
        catch {
            // Corrupt or unreadable -- start fresh
        }
        this._cache = createDefaultPrefs();
        return this._cache;
    }
    /** Save preferences to disk. */
    save(prefs) {
        this._cache = prefs;
        try {
            const dir = path.dirname(this.prefsPath);
            fs.mkdirSync(dir, { recursive: true });
            fs.writeFileSync(this.prefsPath, JSON.stringify(prefs, null, 2) + '\n');
        }
        catch {
            // Ignore write errors -- preference loss is not critical
        }
    }
    /** Invalidate the in-memory cache (e.g. after /login changes keys). */
    invalidateCache() {
        this._cache = null;
    }
}
// ---------------------------------------------------------------------------
// Singleton export
// ---------------------------------------------------------------------------
export const providerPreferences = new ProviderPreferenceManager();
export { MODEL_CATALOG };
//# sourceMappingURL=provider-preferences.js.map