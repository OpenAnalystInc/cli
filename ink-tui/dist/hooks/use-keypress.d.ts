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
import type { Key as InkKey } from 'ink';
import type { Command } from '../key/commands.js';
/**
 * Handler signature for useKeypress.
 *
 * @param input  - Raw character string from Ink's useInput.
 * @param key    - Ink Key object with boolean flags for special keys.
 * @param command - If the key event matched a known Command, it is provided.
 *                  Undefined when no command binding matches.
 * @returns `true` if the event was consumed (prevents lower-priority handlers).
 */
export type KeypressHandler = (input: string, key: InkKey, command: Command | undefined) => boolean;
export interface UseKeypressOptions {
    /**
     * When false, the handler is unsubscribed and receives no events.
     * Use this to gate on modal visibility, focus state, etc.
     */
    isActive: boolean;
    /**
     * Higher priority handlers fire first. If they return true, lower
     * priority handlers never see the event.
     *
     * Recommended values:
     *   10 = permission dialog
     *    9 = ask-user dialog
     *    8 = autocomplete
     *    7 = voice recording
     *    5 = sidebar / scroll mode
     *    3 = input box
     *    0 = global / fallback
     */
    priority: number;
}
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
export declare function useKeypress(handler: KeypressHandler, options: UseKeypressOptions): void;
export type { Command } from '../key/commands.js';
export type { InkKey } from '../contexts/keypress-context.js';
