/**
 * useKeypress — priority-based keypress hook for OpenAnalyst TUI components.
 *
 * Wraps the KeypressProvider's subscriber system into a convenient hook
 * that auto-subscribes on mount and unsubscribes on unmount (or when
 * isActive becomes false).
 *
 * Priority guide:
 *   10 — Permission dialog (modal)
 *    9 — Ask-user dialog (modal)
 *    8 — Autocomplete popup
 *    7 — Voice recording
 *    5 — Sidebar / Scroll mode
 *    3 — Input mode
 *    0 — Global / fallback
 */
import { useEffect, useRef } from 'react';
import { useKeypressContext } from '../contexts/keypress-context.js';
// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------
/**
 * Subscribe to keypress events with priority-based dispatch.
 *
 * The handler is called only when `isActive` is true. Higher priority
 * handlers fire first. Returning `true` from the handler marks the
 * event as consumed.
 *
 * @example
 * ```tsx
 * useKeypress(
 *   (input, key, command) => {
 *     if (command === Command.DIALOG_ALLOW) {
 *       handleAllow();
 *       return true; // consumed
 *     }
 *     return false;
 *   },
 *   { isActive: dialogVisible, priority: 10 },
 * );
 * ```
 */
export function useKeypress(handler, options) {
    const { subscribe } = useKeypressContext();
    const { isActive, priority } = options;
    // Keep the handler in a ref so we always call the latest closure
    // without needing to re-subscribe on every render.
    const handlerRef = useRef(handler);
    handlerRef.current = handler;
    useEffect(() => {
        if (!isActive)
            return;
        // Stable wrapper that delegates to the latest handler ref.
        const stableHandler = (input, key, command) => handlerRef.current(input, key, command);
        const unsubscribe = subscribe(stableHandler, priority);
        return unsubscribe;
    }, [isActive, priority, subscribe]);
}
//# sourceMappingURL=use-keypress.js.map