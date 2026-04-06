/**
 * useInputHistory — prompt history hook for the input box.
 *
 * Stores past submissions in an array and provides Up/Down navigation.
 * When navigating, the original draft text is preserved so the user
 * can return to it by pressing Down past the last entry.
 *
 * Usage:
 *   const history = useInputHistory();
 *   // On submit:  history.push(text);
 *   // On Up:      const prev = history.goUp(currentText);
 *   // On Down:    const next = history.goDown();
 */
import { useCallback, useRef, useState } from 'react';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
/** Maximum number of history entries retained. */
const MAX_HISTORY = 100;
// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------
export function useInputHistory() {
    const [, setTick] = useState(0);
    const entriesRef = useRef([]);
    const cursorRef = useRef(-1); // -1 = not navigating (showing draft)
    const draftRef = useRef('');
    const push = useCallback((text) => {
        const trimmed = text.trim();
        if (!trimmed)
            return;
        const entries = entriesRef.current;
        // Deduplicate: remove if the same text already exists at the end
        if (entries.length > 0 && entries[entries.length - 1] === trimmed) {
            // Already the most recent entry, skip
        }
        else {
            entries.push(trimmed);
            if (entries.length > MAX_HISTORY) {
                entries.shift();
            }
        }
        // Reset navigation state
        cursorRef.current = -1;
        draftRef.current = '';
        setTick((t) => t + 1);
    }, []);
    const goUp = useCallback((currentText) => {
        const entries = entriesRef.current;
        if (entries.length === 0)
            return null;
        if (cursorRef.current === -1) {
            // Starting history navigation — save the current draft
            draftRef.current = currentText;
            cursorRef.current = entries.length - 1;
        }
        else if (cursorRef.current > 0) {
            cursorRef.current -= 1;
        }
        else {
            // Already at the oldest entry
            return entries[0];
        }
        setTick((t) => t + 1);
        return entries[cursorRef.current];
    }, []);
    const goDown = useCallback(() => {
        const entries = entriesRef.current;
        if (cursorRef.current === -1)
            return null; // Not navigating
        if (cursorRef.current < entries.length - 1) {
            cursorRef.current += 1;
            setTick((t) => t + 1);
            return entries[cursorRef.current];
        }
        // Past the last entry — return to draft
        cursorRef.current = -1;
        setTick((t) => t + 1);
        return draftRef.current;
    }, []);
    const reset = useCallback(() => {
        cursorRef.current = -1;
        draftRef.current = '';
        setTick((t) => t + 1);
    }, []);
    const current = cursorRef.current >= 0
        ? entriesRef.current[cursorRef.current] ?? null
        : null;
    return {
        goUp,
        goDown,
        push,
        reset,
        current,
        count: entriesRef.current.length,
    };
}
//# sourceMappingURL=use-input-history.js.map