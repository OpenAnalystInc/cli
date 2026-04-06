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
export interface ProviderPreferences {
    /** The user's chosen default provider (set during /login or /model). */
    defaultProvider: string | null;
    /** When the default was last set (epoch ms). */
    defaultSetAt: number | null;
    /** Per-action routing overrides (from sidebar). */
    routing: Record<string, RoutingChoice>;
}
export interface RoutingChoice {
    provider: string;
    model: string;
    tier: string;
}
/** A single model entry in the catalog. */
export interface ModelInfo {
    /** Canonical model ID sent to the engine, e.g. "claude-sonnet-4-6". */
    id: string;
    /** Human-readable name, e.g. "Sonnet 4". */
    name: string;
    /** Provider key matching PROVIDER_CONFIG, e.g. "anthropic". */
    provider: string;
    /** Short aliases the user can type, e.g. ["sonnet", "sonnet-4"]. */
    aliases: string[];
    /** Performance tier: "fast" | "balanced" | "capable". */
    tier: 'fast' | 'balanced' | 'capable';
    /** Context window size in tokens. */
    contextWindow: number;
    /** Whether the model supports image/vision input. */
    supportsVision: boolean;
    /** Whether the model supports tool/function calling. */
    supportsTools: boolean;
}
declare const MODEL_CATALOG: readonly ModelInfo[];
declare class ProviderPreferenceManager {
    private prefsPath;
    private _cache;
    constructor();
    /** Get the user's default provider key (e.g. "anthropic"). */
    getDefaultProvider(): string | null;
    /** Set the default provider (called during /login). */
    setDefaultProvider(provider: string): void;
    /** Get routing for a specific action category. */
    getRouting(action: string): RoutingChoice | null;
    /** Set routing for an action category (from sidebar). */
    setRouting(action: string, provider: string, model: string, tier: string): void;
    /** Get all models available based on configured API keys. */
    getAvailableModels(): ModelInfo[];
    /** Get models for a specific provider. */
    getModelsForProvider(provider: string): ModelInfo[];
    /** Get all providers that have valid API keys. */
    getConfiguredProviders(): string[];
    /** Get the full model catalog (regardless of API key availability). */
    getFullCatalog(): readonly ModelInfo[];
    /** Resolve a model alias to a ModelInfo, or null if not found. */
    resolveAlias(alias: string): ModelInfo | null;
    /** Get the default model for a provider (first "balanced" tier, or first model). */
    getDefaultModelForProvider(provider: string): ModelInfo | null;
    /** Format the /models output for display in the chat. */
    formatModelsOutput(currentModelId?: string): string;
    /** Load preferences from disk (cached). */
    private load;
    /** Save preferences to disk. */
    private save;
    /** Invalidate the in-memory cache (e.g. after /login changes keys). */
    invalidateCache(): void;
}
export declare const providerPreferences: ProviderPreferenceManager;
export { MODEL_CATALOG };
