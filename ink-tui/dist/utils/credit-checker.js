/**
 * credit-checker -- Fetch real API credit balances from all configured providers.
 *
 * Uses the unified CredentialManager for key detection and credit fetching.
 * Supports all providers: OpenAI, Anthropic, Gemini, xAI, OpenRouter, Bedrock,
 * Stability, and OpenAnalyst.
 *
 * Results are cached for 5 minutes to avoid excessive API calls.
 */
import { credentialManager, PROVIDER_CONFIG } from './credential-manager.js';
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes
let cachedResult = null;
// ---------------------------------------------------------------------------
// Main fetch function
// ---------------------------------------------------------------------------
/**
 * Fetch the user's API credit balance from their primary provider.
 *
 * Checks all providers in priority order (the first one with a key wins):
 *   1. Anthropic (sk-ant-* prefix is most specific)
 *   2. OpenAI (sk-* prefix)
 *   3. Gemini
 *   4. xAI
 *   5. OpenRouter
 *   6. OpenAnalyst
 *   7. Stability
 *   8. Bedrock
 *
 * Results are cached for 5 minutes.
 */
export async function fetchCredits() {
    // Check cache
    if (cachedResult && Date.now() - cachedResult.fetchedAt < CACHE_TTL_MS) {
        return cachedResult.info;
    }
    let info;
    try {
        // Check providers in priority order
        const checkOrder = [
            'anthropic',
            'openai',
            'gemini',
            'xai',
            'openrouter',
            'openanalyst',
            'stability',
            'bedrock',
        ];
        let foundProvider = null;
        for (const providerKey of checkOrder) {
            const resolved = credentialManager.getApiKey(providerKey);
            if (resolved.key) {
                foundProvider = providerKey;
                break;
            }
        }
        if (foundProvider) {
            const config = PROVIDER_CONFIG[foundProvider];
            const creditStr = await credentialManager.fetchCredits(foundProvider);
            info = {
                balance: creditStr,
                provider: config.displayName,
            };
        }
        else {
            info = { balance: 'No API key', provider: 'unknown' };
        }
    }
    catch {
        info = { balance: 'API credits', provider: 'unknown' };
    }
    cachedResult = { info, fetchedAt: Date.now() };
    return info;
}
/**
 * Fetch credit info for ALL configured providers.
 * Returns an array of CreditInfo objects, one per provider with a key.
 */
export async function fetchAllProviderCredits() {
    const results = [];
    const allCredits = await credentialManager.fetchAllCredits();
    for (const [providerKey, creditStr] of Object.entries(allCredits)) {
        const config = PROVIDER_CONFIG[providerKey];
        if (config) {
            results.push({
                balance: creditStr,
                provider: config.displayName,
            });
        }
    }
    return results;
}
/**
 * Clear the credit cache. Useful when the user changes their API key.
 */
export function clearCreditCache() {
    cachedResult = null;
}
//# sourceMappingURL=credit-checker.js.map