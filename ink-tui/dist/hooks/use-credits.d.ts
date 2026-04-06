/**
 * useCredits — React hook for fetching and displaying API credit balance.
 *
 * Features:
 *   - Fetches on mount
 *   - Auto-refreshes every 5 minutes
 *   - Shows "checking..." while loading
 *   - Returns formatted balance, provider name, and loading state
 *   - Manual refresh via refresh()
 *
 * The hook updates the UI state context's creditBalance field so that
 * other components (InputBox, Banner) can read it.
 */
export interface UseCreditResult {
    /** Formatted balance string: "$12.50", "Anthropic connected", "No API key", etc. */
    balance: string;
    /** Provider name: "OpenAI" | "Anthropic" | "unknown" */
    provider: string;
    /** True while the initial or refresh fetch is in progress */
    loading: boolean;
    /** Trigger a manual refresh (clears cache first) */
    refresh: () => void;
}
export declare function useCredits(): UseCreditResult;
