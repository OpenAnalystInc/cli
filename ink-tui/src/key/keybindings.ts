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

// ---------------------------------------------------------------------------
// Normalised key representation (internal to the binding system)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// KeyBinding class
// ---------------------------------------------------------------------------

/** Valid long key names that can appear after modifiers in a pattern. */
const VALID_LONG_KEYS = new Set([
  'return', 'enter', 'escape', 'tab', 'backspace', 'delete', 'space',
  'up', 'down', 'left', 'right',
  'pageup', 'pagedown', 'home', 'end',
  ...Array.from({ length: 12 }, (_, i) => `f${i + 1}`),
]);

export class KeyBinding {
  readonly name: string;
  readonly ctrl: boolean;
  readonly shift: boolean;
  readonly meta: boolean;

  /** The original pattern string (for display / serialisation). */
  readonly pattern: string;

  constructor(pattern: string) {
    this.pattern = pattern;
    let remains = pattern.trim();
    let ctrl = false;
    let shift = false;
    let meta = false;

    // Parse modifier prefixes (order-insensitive, repeatable)
    let matched: boolean;
    do {
      matched = false;
      const lower = remains.toLowerCase();
      if (lower.startsWith('ctrl+')) {
        ctrl = true;
        remains = remains.slice(5);
        matched = true;
      } else if (lower.startsWith('shift+')) {
        shift = true;
        remains = remains.slice(6);
        matched = true;
      } else if (lower.startsWith('meta+') || lower.startsWith('cmd+')) {
        meta = true;
        remains = remains.slice(lower.startsWith('meta+') ? 5 : 4);
        matched = true;
      } else if (lower.startsWith('alt+') || lower.startsWith('option+')) {
        // Ink maps Alt/Option to `meta` on most platforms
        meta = true;
        remains = remains.slice(lower.startsWith('alt+') ? 4 : 7);
        matched = true;
      }
    } while (matched);

    const key = remains;
    const isSingleChar = [...key].length === 1;

    // Normalise aliases
    let name = key.toLowerCase();
    if (name === 'enter') name = 'return';

    if (!isSingleChar && !VALID_LONG_KEYS.has(name)) {
      throw new Error(
        `Invalid keybinding key: "${key}" in "${pattern}". ` +
        `Must be a single character or one of: ${[...VALID_LONG_KEYS].join(', ')}`,
      );
    }

    // If the raw character was uppercase (e.g. 'G') and shift was not
    // explicitly declared, infer shift.
    if (isSingleChar && name !== key) {
      shift = true;
    }

    this.name = name;
    this.ctrl = ctrl;
    this.shift = shift;
    this.meta = meta;
  }

  // -----------------------------------------------------------------------
  // Matching against Ink's useInput callback args
  // -----------------------------------------------------------------------

  /**
   * Check whether an Ink useInput event matches this binding.
   *
   * @param input - The `input` string from Ink's useInput (the character
   *   typed, or '' for special keys).
   * @param key - The `key` object from Ink's useInput.
   */
  matches(input: string, key: InkKey): boolean {
    // Modifier checks
    if (this.ctrl !== key.ctrl) return false;
    if (this.shift !== key.shift) return false;
    if (this.meta !== key.meta) return false;

    // Special key matching (Ink uses boolean flags)
    switch (this.name) {
      case 'return':
        return key.return;
      case 'escape':
        return key.escape;
      case 'tab':
        return key.tab;
      case 'backspace':
        return key.backspace;
      case 'delete':
        return key.delete;
      case 'up':
        return key.upArrow;
      case 'down':
        return key.downArrow;
      case 'left':
        return key.leftArrow;
      case 'right':
        return key.rightArrow;
      case 'pageup':
        return key.pageUp;
      case 'pagedown':
        return key.pageDown;
      case 'space':
        return input === ' ';
      // Home/End/F-keys: Ink does not surface dedicated booleans for these.
      // They arrive as escape sequences in the input string. We match on
      // the normalised name which the keypress-context will parse for us.
      // For now, fall through to character matching.
      default:
        break;
    }

    // Function keys — Ink doesn't have dedicated booleans, but the
    // keypress context will normalise them for us via raw stdin parsing.
    // When that is in place, they'll match via the 'name' field on
    // the extended key object in the context. For now, simple char match.

    // Character key matching
    if (this.ctrl) {
      // When ctrl is held, `input` is often a control char. Compare the
      // key name against the expected character.
      return input.toLowerCase() === this.name || input === this.name;
    }

    // Normal character
    return input.toLowerCase() === this.name;
  }

  /** Equality check (same logical binding). */
  equals(other: KeyBinding): boolean {
    return (
      this.name === other.name &&
      this.ctrl === other.ctrl &&
      this.shift === other.shift &&
      this.meta === other.meta
    );
  }
}

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

export type KeyBindingConfig = Map<Command, readonly KeyBinding[]>;

// ---------------------------------------------------------------------------
// Default key binding configuration
// Derived 1:1 from keybindings.rs — every match arm is represented.
// ---------------------------------------------------------------------------

export const defaultKeyBindings: KeyBindingConfig = new Map<Command, readonly KeyBinding[]>([
  // ===== Global =====
  [Command.QUIT, [
    new KeyBinding('ctrl+c'),
  ]],
  [Command.RUN_IN_BACKGROUND, [
    new KeyBinding('ctrl+b'),
  ]],
  [Command.CYCLE_PERMISSION_MODE, [
    new KeyBinding('ctrl+p'),
  ]],
  [Command.TOGGLE_SIDEBAR, [
    new KeyBinding('ctrl+\\'),
    new KeyBinding('ctrl+e'),
  ]],
  [Command.FOCUS_SIDEBAR, [
    new KeyBinding('f2'),
    new KeyBinding('ctrl+e'),
  ]],
  [Command.CLEAR_CHAT, [
    new KeyBinding('ctrl+l'),
  ]],
  [Command.SCROLL_TO_TOP, [
    new KeyBinding('ctrl+home'),
  ]],
  [Command.SCROLL_TO_BOTTOM, [
    new KeyBinding('ctrl+end'),
  ]],
  [Command.SCROLL_UP_PAGE, [
    new KeyBinding('pageup'),
  ]],
  [Command.SCROLL_DOWN_PAGE, [
    new KeyBinding('pagedown'),
  ]],

  // ===== Input mode =====
  [Command.SUBMIT, [
    new KeyBinding('return'),
  ]],
  [Command.ENTER_SCROLL_MODE, [
    new KeyBinding('escape'),
  ]],
  // UNDO_LAST_ACTION is double-Esc — handled by temporal logic in the
  // component, not by a simple key pattern. Listed here for completeness
  // and help display. The double-tap detection lives in the keypress context.
  [Command.UNDO_LAST_ACTION, [
    new KeyBinding('escape'),
  ]],
  [Command.START_VOICE_RECORDING, [
    new KeyBinding('space'),
  ]],
  [Command.HISTORY_UP, [
    new KeyBinding('up'),
  ]],
  [Command.HISTORY_DOWN, [
    new KeyBinding('down'),
  ]],
  [Command.REMOVE_LAST_CONTEXT_FILE, [
    new KeyBinding('backspace'),
  ]],

  // ===== Scroll mode =====
  [Command.SCROLL_DOWN, [
    new KeyBinding('j'),
    new KeyBinding('down'),
  ]],
  [Command.SCROLL_UP, [
    new KeyBinding('k'),
    new KeyBinding('up'),
  ]],
  [Command.JUMP_TO_BOTTOM, [
    new KeyBinding('shift+g'),  // 'G' in vim
  ]],
  [Command.JUMP_TO_TOP, [
    new KeyBinding('g'),
  ]],
  [Command.TOGGLE_EXPAND, [
    new KeyBinding('return'),
  ]],
  [Command.NEXT_TAB, [
    new KeyBinding('tab'),
  ]],
  [Command.PREV_TAB, [
    new KeyBinding('shift+tab'),
  ]],
  [Command.FEEDBACK_POSITIVE, [
    new KeyBinding('y'),
  ]],
  [Command.FEEDBACK_NEGATIVE, [
    new KeyBinding('n'),
  ]],
  [Command.EXIT_SCROLL_MODE, [
    new KeyBinding('i'),
    new KeyBinding('escape'),
  ]],
  [Command.START_SEARCH, [
    new KeyBinding('/'),
  ]],

  // ===== Sidebar =====
  [Command.SIDEBAR_NEXT_ITEM, [
    new KeyBinding('j'),
    new KeyBinding('down'),
  ]],
  [Command.SIDEBAR_PREV_ITEM, [
    new KeyBinding('k'),
    new KeyBinding('up'),
  ]],
  [Command.SIDEBAR_NEXT_SECTION, [
    new KeyBinding('tab'),
  ]],
  [Command.SIDEBAR_PREV_SECTION, [
    new KeyBinding('shift+tab'),
  ]],
  [Command.SIDEBAR_ACTION, [
    new KeyBinding('return'),
  ]],
  [Command.SIDEBAR_EXIT, [
    new KeyBinding('escape'),
    new KeyBinding('i'),
  ]],

  // ===== Permission dialog =====
  [Command.DIALOG_SWITCH_BUTTON, [
    new KeyBinding('tab'),
    new KeyBinding('left'),
    new KeyBinding('right'),
  ]],
  [Command.DIALOG_CONFIRM, [
    new KeyBinding('return'),
  ]],
  [Command.DIALOG_ALLOW, [
    new KeyBinding('y'),
    new KeyBinding('shift+y'),  // Y
  ]],
  [Command.DIALOG_DENY, [
    new KeyBinding('n'),
    new KeyBinding('shift+n'),  // N
    new KeyBinding('escape'),
  ]],

  // ===== Ask-user dialog (choice mode) =====
  [Command.ASK_NEXT_OPTION, [
    new KeyBinding('down'),
    new KeyBinding('j'),
  ]],
  [Command.ASK_PREV_OPTION, [
    new KeyBinding('up'),
    new KeyBinding('k'),
  ]],
  [Command.ASK_SELECT, [
    new KeyBinding('return'),
  ]],
  // Quick-select: 1-9 number keys. Handled by pattern + runtime check
  // in the component (char '1'..'9'). Listing '1' here for help display.
  [Command.ASK_QUICK_SELECT, [
    new KeyBinding('1'),
    new KeyBinding('2'),
    new KeyBinding('3'),
    new KeyBinding('4'),
    new KeyBinding('5'),
    new KeyBinding('6'),
    new KeyBinding('7'),
    new KeyBinding('8'),
    new KeyBinding('9'),
  ]],
  [Command.ASK_SWITCH_TO_TYPE, [
    new KeyBinding('t'),
    new KeyBinding('shift+t'),
  ]],
  [Command.ASK_CHAT_ABOUT_IT, [
    new KeyBinding('c'),
    new KeyBinding('shift+c'),
  ]],

  // ===== Autocomplete =====
  [Command.AC_NEXT, [
    new KeyBinding('down'),
  ]],
  [Command.AC_PREV, [
    new KeyBinding('shift+tab'),
    new KeyBinding('up'),
  ]],
  [Command.AC_ACCEPT, [
    new KeyBinding('tab'),
  ]],
  [Command.AC_ACCEPT_SUBMIT, [
    new KeyBinding('return'),
  ]],
  [Command.AC_DISMISS, [
    new KeyBinding('escape'),
  ]],

  // ===== Voice =====
  [Command.VOICE_STOP, [
    new KeyBinding('space'),
    new KeyBinding('escape'),
    new KeyBinding('return'),
  ]],
]);

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/**
 * Check whether a given Ink key event matches any binding for a command.
 */
export function matchesCommand(
  command: Command,
  input: string,
  key: InkKey,
  config: KeyBindingConfig = defaultKeyBindings,
): boolean {
  const bindings = config.get(command);
  if (!bindings) return false;
  return bindings.some((b) => b.matches(input, key));
}

/**
 * Find the first Command whose bindings match the given key event,
 * searching only within a provided set of commands.
 */
export function resolveCommand(
  input: string,
  key: InkKey,
  candidates: readonly Command[],
  config: KeyBindingConfig = defaultKeyBindings,
): Command | undefined {
  for (const cmd of candidates) {
    if (matchesCommand(cmd, input, key, config)) {
      return cmd;
    }
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

const KEY_DISPLAY_MAP: Record<string, string> = {
  return: 'Enter',
  escape: 'Esc',
  tab: 'Tab',
  backspace: 'Bksp',
  delete: 'Del',
  up: 'Up',
  down: 'Down',
  left: 'Left',
  right: 'Right',
  pageup: 'PgUp',
  pagedown: 'PgDn',
  home: 'Home',
  end: 'End',
  space: 'Space',
};

/**
 * Format a KeyBinding as a human-readable string (e.g. "Ctrl+C").
 */
export function formatKeyBinding(binding: KeyBinding): string {
  const parts: string[] = [];
  if (binding.ctrl) parts.push('Ctrl');
  if (binding.meta) parts.push('Alt');
  if (binding.shift) parts.push('Shift');

  const displayName =
    KEY_DISPLAY_MAP[binding.name] ??
    (binding.name.startsWith('f') && /^f\d+$/.test(binding.name)
      ? binding.name.toUpperCase()
      : binding.name.toUpperCase());

  parts.push(displayName);
  return parts.join('+');
}

/**
 * Get the primary (first) human-readable shortcut string for a command.
 */
export function formatCommand(
  command: Command,
  config: KeyBindingConfig = defaultKeyBindings,
): string {
  const bindings = config.get(command);
  if (!bindings || bindings.length === 0) return '';
  return formatKeyBinding(bindings[0]);
}
