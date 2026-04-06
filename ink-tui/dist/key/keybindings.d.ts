/**
 * KeyBinding class and default binding map for OpenAnalyst TUI.
 *
 * Design inspired by Google Gemini CLI's KeyBinding class but adapted
 * to map onto Ink's useInput callback signature:
 *   (input: string, key: InkKey) => void
 *
 * Ink's Key type exposes boolean flags for special keys (upArrow, return,
 * escape, ctrl, shift, tab, backspace, delete, meta, pageUp, pageDown)
 * plus the raw `input` string for character keys.
 *
 * Source of truth for *what* each command does: keybindings.rs
 */
import type { Key as InkKey } from 'ink';
import { Command } from './commands.js';
/**
 * A normalised key name used for matching. This bridges between the
 * human-readable pattern string (e.g. "ctrl+c") and Ink's Key object.
 */
export interface NormalisedKey {
    /** Lowercase key name: 'a'..'z', '0'..'9', or special names like
     *  'return', 'escape', 'tab', 'up', 'down', 'left', 'right',
     *  'pageup', 'pagedown', 'home', 'end', 'backspace', 'delete',
     *  'space', 'f1'..'f12', '/', '\\', etc. */
    name: string;
    ctrl: boolean;
    shift: boolean;
    meta: boolean;
}
export declare class KeyBinding {
    readonly name: string;
    readonly ctrl: boolean;
    readonly shift: boolean;
    readonly meta: boolean;
    /** The original pattern string (for display / serialisation). */
    readonly pattern: string;
    constructor(pattern: string);
    /**
     * Check whether an Ink useInput event matches this binding.
     *
     * @param input - The `input` string from Ink's useInput (the character
     *   typed, or '' for special keys).
     * @param key - The `key` object from Ink's useInput.
     */
    matches(input: string, key: InkKey): boolean;
    /** Equality check (same logical binding). */
    equals(other: KeyBinding): boolean;
}
export type KeyBindingConfig = Map<Command, readonly KeyBinding[]>;
export declare const defaultKeyBindings: KeyBindingConfig;
/**
 * Check whether a given Ink key event matches any binding for a command.
 */
export declare function matchesCommand(command: Command, input: string, key: InkKey, config?: KeyBindingConfig): boolean;
/**
 * Find the first Command whose bindings match the given key event,
 * searching only within a provided set of commands.
 */
export declare function resolveCommand(input: string, key: InkKey, candidates: readonly Command[], config?: KeyBindingConfig): Command | undefined;
/**
 * Format a KeyBinding as a human-readable string (e.g. "Ctrl+C").
 */
export declare function formatKeyBinding(binding: KeyBinding): string;
/**
 * Get the primary (first) human-readable shortcut string for a command.
 */
export declare function formatCommand(command: Command, config?: KeyBindingConfig): string;
