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

import { useState, useEffect, useCallback, useRef } from 'react';
import { fetchCredits, clearCreditCache, type CreditInfo } from '../utils/credit-checker.js';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const REFRESH_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useCredits(): UseCreditResult {
  const [info, setInfo] = useState<CreditInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const mountedRef = useRef(true);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const doFetch = useCallback(async (clearCache = false) => {
    if (clearCache) {
      clearCreditCache();
    }
    setLoading(true);
    try {
      const result = await fetchCredits();
      if (mountedRef.current) {
        setInfo(result);
      }
    } catch {
      if (mountedRef.current) {
        setInfo({ balance: 'API credits', provider: 'unknown' });
      }
    } finally {
      if (mountedRef.current) {
        setLoading(false);
      }
    }
  }, []);

  // Initial fetch on mount
  useEffect(() => {
    mountedRef.current = true;
    doFetch();

    // Set up auto-refresh interval
    intervalRef.current = setInterval(() => {
      doFetch();
    }, REFRESH_INTERVAL_MS);

    return () => {
      mountedRef.current = false;
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [doFetch]);

  const refresh = useCallback(() => {
    doFetch(true);
  }, [doFetch]);

  return {
    balance: loading && !info ? 'checking...' : (info?.balance ?? 'API credits'),
    provider: info?.provider ?? 'unknown',
    loading,
    refresh,
  };
}
