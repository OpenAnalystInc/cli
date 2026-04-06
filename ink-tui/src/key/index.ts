/**
 * Barrel exports for the keybinding system.
 */

// Command enum and metadata
export { Command, commandDescriptions, commandCategories } from './commands.js';
export type { CommandCategory } from './commands.js';

// KeyBinding class, default config, and utilities
export {
  KeyBinding,
  defaultKeyBindings,
  matchesCommand,
  resolveCommand,
  formatKeyBinding,
  formatCommand,
} from './keybindings.js';
export type { KeyBindingConfig, NormalisedKey } from './keybindings.js';
