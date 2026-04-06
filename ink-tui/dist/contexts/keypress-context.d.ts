/**
 * KeypressProvider — priority-based keypress dispatcher.
 *
 * Wraps Ink's useInput at the top level and fans out key events to
 * subscribers ordered by descending priority. The first subscriber
 * that returns `true` from its handler "consumes" the event; lower
 * priority subscribers do not see it.
 *
 * Priority guide (from keybindings.rs dispatch order):
 *   10 — Permission dialog (modal, highest)
 *    9 — Ask-user dialog (modal)
 *    8 — Autocomplete popup
 *    7 — Voice recording
 *    5 — Sidebar (when focused)
 *    5 — Scroll mode
 *    3 — Input mode (default)
 *    0 — Fallback / global shortcuts
 */
import React, { type ReactNode } from 'react';
import { type Key as InkKey } from 'ink';
import { Command } from '../key/commands.js';
import { type KeyBindingConfig } from '../key/keybindings.js';
export type { InkKey };
/**
 * A keypress handler returns `true` if it consumed the event.
 * When consumed, no lower-priority handlers are called.
 */
export type KeypressHandler = (input: string, key: InkKey, command: Command | undefined) => boolean;
export interface Subscriber {
    handler: KeypressHandler;
    priority: number;
}
export interface KeypressContextValue {
    /**
     * Register a keypress subscriber. Returns an unsubscribe function.
     * Subscribers with higher `priority` fire first.
     */
    subscribe: (handler: KeypressHandler, priority: number) => () => void;
    /**
     * The active keybinding config (allows runtime customisation).
     */
    keyBindings: KeyBindingConfig;
}
export interface KeypressProviderProps {
    children: ReactNode;
    /** Override default key bindings (e.g. after loading user customisations). */
    keyBindings?: KeyBindingConfig;
}
export declare function KeypressProvider({ children, keyBindings, }: KeypressProviderProps): React.JSX.Element;
export declare function useKeypressContext(): KeypressContextValue;
