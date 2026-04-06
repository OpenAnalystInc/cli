import { jsx as _jsx } from "react/jsx-runtime";
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
import React, { createContext, useCallback, useContext, useEffect, useRef, } from 'react';
import { useInput } from 'ink';
import { Command } from '../key/commands.js';
import { defaultKeyBindings, matchesCommand, } from '../key/keybindings.js';
// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------
const KeypressContext = createContext(null);
export function KeypressProvider({ children, keyBindings = defaultKeyBindings, }) {
    // Mutable ref for subscriber list — avoids re-renders on subscribe/unsubscribe.
    const subscribersRef = useRef([]);
    // Subscribe function — stable identity via useCallback.
    const subscribe = useCallback((handler, priority) => {
        const subscriber = { handler, priority };
        subscribersRef.current.push(subscriber);
        // Keep sorted descending by priority so dispatch is a simple linear scan.
        subscribersRef.current.sort((a, b) => b.priority - a.priority);
        return () => {
            subscribersRef.current = subscribersRef.current.filter((s) => s !== subscriber);
        };
    }, []);
    // Central Ink useInput handler — dispatches to subscribers by priority.
    useInput((input, key) => {
        // Try to resolve a command from the full command enum.
        // We check all commands — the subscriber decides whether the command
        // is relevant to its current mode.
        let resolvedCommand;
        for (const cmd of Object.values(Command)) {
            if (matchesCommand(cmd, input, key, keyBindings)) {
                resolvedCommand = cmd;
                break;
            }
        }
        // Dispatch to subscribers in priority order
        for (const sub of subscribersRef.current) {
            const consumed = sub.handler(input, key, resolvedCommand);
            if (consumed)
                return;
        }
    }, { isActive: true });
    // Raw stdin listener for F-keys and other escape sequences that Ink's
    // useInput doesn't capture. F1-F12 arrive as escape sequences:
    //   F1: \x1bOP or \x1b[11~    F2: \x1bOQ or \x1b[12~
    //   F3: \x1bOR or \x1b[13~    F4: \x1bOS or \x1b[14~
    //   F5: \x1b[15~              F6: \x1b[17~   etc.
    useEffect(() => {
        const stdin = process.stdin;
        if (!stdin || !stdin.readable)
            return;
        const F_KEY_MAP = {
            '\x1bOQ': Command.FOCUS_SIDEBAR, // F2 (xterm)
            '\x1b[12~': Command.FOCUS_SIDEBAR, // F2 (vt100)
            '\x1b[[B': Command.FOCUS_SIDEBAR, // F2 (linux console)
            '\x00<': Command.FOCUS_SIDEBAR, // F2 (Windows cmd)
            '\x1bOP': Command.SCROLL_TO_TOP, // F1 (mapped to help/scroll)
            '\x1b[11~': Command.SCROLL_TO_TOP, // F1 (vt100)
        };
        const onData = (data) => {
            const seq = data.toString('utf8');
            const command = F_KEY_MAP[seq];
            if (!command)
                return;
            // Synthesize a fake InkKey for the dispatch
            const fakeKey = {
                upArrow: false, downArrow: false, leftArrow: false, rightArrow: false,
                pageUp: false, pageDown: false,
                return: false, escape: false, tab: false, backspace: false, delete: false,
                ctrl: false, shift: false, meta: false,
            };
            for (const sub of subscribersRef.current) {
                const consumed = sub.handler('', fakeKey, command);
                if (consumed)
                    return;
            }
        };
        stdin.on('data', onData);
        return () => { stdin.removeListener('data', onData); };
    }, []);
    // Build the stable context value.
    // subscribe is already stable; keyBindings changes only if the prop changes.
    const contextValue = React.useMemo(() => ({ subscribe, keyBindings }), [subscribe, keyBindings]);
    return (_jsx(KeypressContext.Provider, { value: contextValue, children: children }));
}
// ---------------------------------------------------------------------------
// Consumer hook (low-level — most components should use useKeypress instead)
// ---------------------------------------------------------------------------
export function useKeypressContext() {
    const ctx = useContext(KeypressContext);
    if (!ctx) {
        throw new Error('useKeypressContext must be used within a <KeypressProvider>');
    }
    return ctx;
}
//# sourceMappingURL=keypress-context.js.map