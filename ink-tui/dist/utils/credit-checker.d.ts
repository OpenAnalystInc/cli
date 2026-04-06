/**
 * credit-checker -- Fetch real API credit balances from all configured providers.
 *
 * Uses the unified CredentialManager for key detection and credit fetching.
 * Supports all providers: OpenAI, Anthropic, Gemini, xAI, OpenRouter, Bedrock,
 * Stability, and OpenAnalyst.
 *
 * Results are cached for 5 minutes to avoid excessive API calls.
 */
export interface CreditInfo {
    /** Formatted balance: "$12.50" or "Connected" or "No API key" */
    balance: string;
    /** Provider name: "OpenAI" | "Anthropic" | "Google Gemini" | etc. */
    provider: string;
    /** Remaining tokens if available */
    remainingTokens?: number;
}
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
export declare function fetchCredits(): Promise<CreditInfo>;
/**
 * Fetch credit info for ALL configured providers.
 * Returns an array of CreditInfo objects, one per provider with a key.
 */
export declare function fetchAllProviderCredits(): Promise<CreditInfo[]>;
/**
 * Clear the credit cache. Useful when the user changes their API key.
 */
export declare function clearCreditCache(): void;
