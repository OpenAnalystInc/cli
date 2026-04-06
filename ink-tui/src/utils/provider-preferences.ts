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
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Model catalog (mirrors Rust MODEL_REGISTRY)
// ---------------------------------------------------------------------------

const MODEL_CATALOG: readonly ModelInfo[] = [
  // -- OpenAI --
  { id: 'gpt-4o',       name: 'GPT-4o',        provider: 'openai', aliases: ['4o'],                tier: 'capable',  contextWindow: 128_000, supportsVision: true,  supportsTools: true },
  { id: 'gpt-4o-mini',  name: 'GPT-4o Mini',   provider: 'openai', aliases: ['4o-mini', 'mini'],   tier: 'fast',     contextWindow: 128_000, supportsVision: true,  supportsTools: true },
  { id: 'gpt-4.1',      name: 'GPT-4.1',       provider: 'openai', aliases: ['4.1'],               tier: 'capable',  contextWindow: 1_047_576, supportsVision: true,  supportsTools: true },
  { id: 'gpt-4.1-mini', name: 'GPT-4.1 Mini',  provider: 'openai', aliases: ['4.1-mini'],          tier: 'balanced', contextWindow: 1_047_576, supportsVision: true,  supportsTools: true },
  { id: 'gpt-4.1-nano', name: 'GPT-4.1 Nano',  provider: 'openai', aliases: ['4.1-nano', 'nano'],  tier: 'fast',     contextWindow: 1_047_576, supportsVision: true,  supportsTools: true },
  { id: 'o3',           name: 'o3',             provider: 'openai', aliases: [],                    tier: 'capable',  contextWindow: 200_000, supportsVision: false, supportsTools: true },
  { id: 'o3-mini',      name: 'o3 Mini',        provider: 'openai', aliases: [],                    tier: 'balanced', contextWindow: 200_000, supportsVision: false, supportsTools: true },
  { id: 'o4-mini',      name: 'o4 Mini',        provider: 'openai', aliases: [],                    tier: 'balanced', contextWindow: 200_000, supportsVision: true,  supportsTools: true },
  { id: 'codex-mini',   name: 'Codex Mini',     provider: 'openai', aliases: ['codex'],             tier: 'fast',     contextWindow: 200_000, supportsVision: false, supportsTools: true },

  // -- Anthropic --
  { id: 'claude-opus-4-6',            name: 'Opus 4',   provider: 'anthropic', aliases: ['opus', 'opus-4'],     tier: 'capable',  contextWindow: 200_000, supportsVision: true, supportsTools: true },
  { id: 'claude-sonnet-4-6',          name: 'Sonnet 4', provider: 'anthropic', aliases: ['sonnet', 'sonnet-4'], tier: 'balanced', contextWindow: 200_000, supportsVision: true, supportsTools: true },
  { id: 'claude-haiku-4-5-20251213',  name: 'Haiku 4.5', provider: 'anthropic', aliases: ['haiku'],             tier: 'fast',     contextWindow: 200_000, supportsVision: true, supportsTools: true },

  // -- Google Gemini --
  { id: 'gemini-2.5-pro',        name: 'Gemini 2.5 Pro',        provider: 'gemini', aliases: ['gemini-pro', 'gemini'],        tier: 'capable',  contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
  { id: 'gemini-2.5-flash',      name: 'Gemini 2.5 Flash',      provider: 'gemini', aliases: ['gemini-flash', 'flash'],       tier: 'fast',     contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
  { id: 'gemini-2.0-flash',      name: 'Gemini 2.0 Flash',      provider: 'gemini', aliases: ['gemini-2-flash'],              tier: 'fast',     contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
  { id: 'gemini-2.0-flash-lite', name: 'Gemini 2.0 Flash Lite', provider: 'gemini', aliases: ['gemini-lite'],                 tier: 'fast',     contextWindow: 1_000_000, supportsVision: true, supportsTools: true },
  { id: 'gemini-1.5-pro',        name: 'Gemini 1.5 Pro',        provider: 'gemini', aliases: [],                              tier: 'balanced', contextWindow: 2_000_000, supportsVision: true, supportsTools: true },
  { id: 'gemini-1.5-flash',      name: 'Gemini 1.5 Flash',      provider: 'gemini', aliases: [],                              tier: 'fast',     contextWindow: 1_000_000, supportsVision: true, supportsTools: true },

  // -- xAI --
  { id: 'grok-3',      name: 'Grok 3',      provider: 'xai', aliases: ['grok'],      tier: 'capable', contextWindow: 131_072, supportsVision: false, supportsTools: true },
  { id: 'grok-3-mini', name: 'Grok 3 Mini', provider: 'xai', aliases: ['grok-mini'], tier: 'fast',    contextWindow: 131_072, supportsVision: false, supportsTools: true },
  { id: 'grok-2',      name: 'Grok 2',      provider: 'xai', aliases: [],             tier: 'balanced', contextWindow: 131_072, supportsVision: false, supportsTools: true },

  // -- OpenRouter (meta-provider) --
  { id: 'openrouter/auto', name: 'Auto (best available)', provider: 'openrouter', aliases: ['auto'], tier: 'balanced', contextWindow: 200_000, supportsVision: true, supportsTools: true },

  // -- Amazon Bedrock --
  { id: 'bedrock/claude', name: 'Bedrock Claude', provider: 'bedrock', aliases: ['bedrock'], tier: 'capable', contextWindow: 200_000, supportsVision: true, supportsTools: true },

  // -- OpenAnalyst (default wrapper) --
  { id: 'openanalyst-beta', name: 'OpenAnalyst Beta', provider: 'openanalyst', aliases: ['oa-beta', 'default'], tier: 'balanced', contextWindow: 200_000, supportsVision: true, supportsTools: true },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getPrefsPath(): string {
  const configHome = process.env['OPENANALYST_CONFIG_HOME']
    ?? path.join(os.homedir(), '.openanalyst');
  return path.join(configHome, 'preferences.json');
}

function createDefaultPrefs(): ProviderPreferences {
  return {
    defaultProvider: null,
    defaultSetAt: null,
    routing: {},
  };
}

function formatContextWindow(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(0)}M context`;
  return `${Math.round(tokens / 1_000)}K context`;
}

// ---------------------------------------------------------------------------
// ProviderPreferenceManager
// ---------------------------------------------------------------------------

class ProviderPreferenceManager {
  private prefsPath: string;
  private _cache: ProviderPreferences | null = null;

  constructor() {
    this.prefsPath = getPrefsPath();
  }

  // -- Default provider ---------------------------------------------------

  /** Get the user's default provider key (e.g. "anthropic"). */
  getDefaultProvider(): string | null {
    return this.load().defaultProvider;
  }

  /** Set the default provider (called during /login). */
  setDefaultProvider(provider: string): void {
    const prefs = this.load();
    prefs.defaultProvider = provider;
    prefs.defaultSetAt = Date.now();
    this.save(prefs);
  }

  // -- Routing ------------------------------------------------------------

  /** Get routing for a specific action category. */
  getRouting(action: string): RoutingChoice | null {
    const prefs = this.load();
    return prefs.routing[action] ?? null;
  }

  /** Set routing for an action category (from sidebar). */
  setRouting(action: string, provider: string, model: string, tier: string): void {
    const prefs = this.load();
    prefs.routing[action] = { provider, model, tier };
    this.save(prefs);
  }

  // -- Model queries ------------------------------------------------------

  /** Get all models available based on configured API keys. */
  getAvailableModels(): ModelInfo[] {
    const configured = new Set(this.getConfiguredProviders());
    return MODEL_CATALOG.filter((m) => configured.has(m.provider));
  }

  /** Get models for a specific provider. */
  getModelsForProvider(provider: string): ModelInfo[] {
    return MODEL_CATALOG.filter((m) => m.provider === provider);
  }

  /** Get all providers that have valid API keys. */
  getConfiguredProviders(): string[] {
    const providers: string[] = [];
    for (const providerKey of Object.keys(PROVIDER_CONFIG)) {
      const resolved = credentialManager.getApiKey(providerKey);
      if (resolved.key) {
        providers.push(providerKey);
      }
    }
    return providers;
  }

  /** Get the full model catalog (regardless of API key availability). */
  getFullCatalog(): readonly ModelInfo[] {
    return MODEL_CATALOG;
  }

  /** Resolve a model alias to a ModelInfo, or null if not found. */
  resolveAlias(alias: string): ModelInfo | null {
    const lower = alias.toLowerCase();
    return MODEL_CATALOG.find((m) =>
      m.id.toLowerCase() === lower ||
      m.aliases.some((a) => a.toLowerCase() === lower),
    ) ?? null;
  }

  /** Get the default model for a provider (first "balanced" tier, or first model). */
  getDefaultModelForProvider(provider: string): ModelInfo | null {
    const models = this.getModelsForProvider(provider);
    return models.find((m) => m.tier === 'balanced') ?? models[0] ?? null;
  }

  // -- Formatted output ---------------------------------------------------

  /** Format the /models output for display in the chat. */
  formatModelsOutput(currentModelId?: string): string {
    const configured = new Set(this.getConfiguredProviders());
    const defaultProvider = this.getDefaultProvider();
    const lines: string[] = [];

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
      const config = PROVIDER_CONFIG[providerKey]!;
      const models = this.getModelsForProvider(providerKey);
      if (models.length === 0) continue;

      lines.push(`  ${config.displayName} (${config.envVar} \u2713)`);

      for (const model of models) {
        const primaryAlias = model.aliases[0] ?? model.id;
        const aliasCol = primaryAlias.padEnd(16);
        const nameCol = model.name.padEnd(24);
        const tierCol = model.tier.padEnd(10);
        const ctxCol = formatContextWindow(model.contextWindow);
        const isCurrent = currentModelId && (
          model.id === currentModelId ||
          model.aliases.some((a) => a === currentModelId)
        );
        const marker = isCurrent ? '  \u2190 current' : '';
        lines.push(`    ${aliasCol}${nameCol}${tierCol}${ctxCol}${marker}`);
      }
      lines.push('');
    }

    // Show unconfigured providers
    if (unconfiguredProviders.length > 0) {
      lines.push('  \u2500\u2500\u2500 Not configured \u2500\u2500\u2500');
      for (const providerKey of unconfiguredProviders) {
        const config = PROVIDER_CONFIG[providerKey]!;
        const models = this.getModelsForProvider(providerKey);
        if (models.length === 0) continue;
        lines.push(`  ${config.displayName} (${config.envVar} \u2717) \u2014 /login ${providerKey} <key> to add`);
      }
      lines.push('');
    }

    lines.push('Usage: /model <name> to switch (e.g., /model opus)');

    return lines.join('\n');
  }

  // -- Persistence --------------------------------------------------------

  /** Load preferences from disk (cached). */
  private load(): ProviderPreferences {
    if (this._cache) return this._cache;

    try {
      if (fs.existsSync(this.prefsPath)) {
        const raw = fs.readFileSync(this.prefsPath, 'utf-8');
        const parsed = JSON.parse(raw) as Partial<ProviderPreferences>;
        this._cache = {
          defaultProvider: parsed.defaultProvider ?? null,
          defaultSetAt: parsed.defaultSetAt ?? null,
          routing: parsed.routing ?? {},
        };
        return this._cache;
      }
    } catch {
      // Corrupt or unreadable -- start fresh
    }

    this._cache = createDefaultPrefs();
    return this._cache;
  }

  /** Save preferences to disk. */
  private save(prefs: ProviderPreferences): void {
    this._cache = prefs;
    try {
      const dir = path.dirname(this.prefsPath);
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(this.prefsPath, JSON.stringify(prefs, null, 2) + '\n');
    } catch {
      // Ignore write errors -- preference loss is not critical
    }
  }

  /** Invalidate the in-memory cache (e.g. after /login changes keys). */
  invalidateCache(): void {
    this._cache = null;
  }
}

// ---------------------------------------------------------------------------
// Singleton export
// ---------------------------------------------------------------------------

export const providerPreferences = new ProviderPreferenceManager();
export { MODEL_CATALOG };
