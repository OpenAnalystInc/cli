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
export declare function useInputHistory(): InputHistory;
