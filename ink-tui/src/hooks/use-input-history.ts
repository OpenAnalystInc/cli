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
// Types
// ---------------------------------------------------------------------------

export interface InputHistory {
  /** Navigate to the previous (older) history entry. Pass the current
   *  input text so it can be restored when navigating back down. */
  goUp: (currentText: string) => string | null;

  /** Navigate to the next (newer) history entry, or the draft text. */
  goDown: () => string | null;

  /** Push a submitted prompt into history and reset the cursor. */
  push: (text: string) => void;

  /** Reset history navigation (e.g. after submit or manual edit). */
  reset: () => void;

  /** The current history entry text, or null if not navigating. */
  current: string | null;

  /** Total number of history entries. */
  count: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Maximum number of history entries retained. */
const MAX_HISTORY = 100;

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useInputHistory(): InputHistory {
  const [, setTick] = useState(0);
  const entriesRef = useRef<string[]>([]);
  const cursorRef = useRef(-1); // -1 = not navigating (showing draft)
  const draftRef = useRef('');

  const push = useCallback((text: string): void => {
    const trimmed = text.trim();
    if (!trimmed) return;

    const entries = entriesRef.current;

    // Deduplicate: remove if the same text already exists at the end
    if (entries.length > 0 && entries[entries.length - 1] === trimmed) {
      // Already the most recent entry, skip
    } else {
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

  const goUp = useCallback((currentText: string): string | null => {
    const entries = entriesRef.current;
    if (entries.length === 0) return null;

    if (cursorRef.current === -1) {
      // Starting history navigation — save the current draft
      draftRef.current = currentText;
      cursorRef.current = entries.length - 1;
    } else if (cursorRef.current > 0) {
      cursorRef.current -= 1;
    } else {
      // Already at the oldest entry
      return entries[0]!;
    }

    setTick((t) => t + 1);
    return entries[cursorRef.current]!;
  }, []);

  const goDown = useCallback((): string | null => {
    const entries = entriesRef.current;
    if (cursorRef.current === -1) return null; // Not navigating

    if (cursorRef.current < entries.length - 1) {
      cursorRef.current += 1;
      setTick((t) => t + 1);
      return entries[cursorRef.current]!;
    }

    // Past the last entry — return to draft
    cursorRef.current = -1;
    setTick((t) => t + 1);
    return draftRef.current;
  }, []);

  const reset = useCallback((): void => {
    cursorRef.current = -1;
    draftRef.current = '';
    setTick((t) => t + 1);
  }, []);

  const current =
    cursorRef.current >= 0
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
