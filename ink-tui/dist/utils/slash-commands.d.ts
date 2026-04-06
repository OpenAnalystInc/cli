/**
 * Slash command parser and registry.
 *
 * Handles the mapping between user-typed slash commands and their
 * dispatch targets (local TUI handlers vs engine bridge).
 */
export interface SlashCommand {
    readonly name: string;
    readonly aliases: readonly string[];
    readonly description: string;
    readonly handler: 'local' | 'engine';
}
export interface ParsedCommand {
    /** The canonical command name (not the alias). */
    name: string;
    /** Everything after the command name, trimmed. */
    args: string;
    /** Whether this command should be handled locally or by the engine. */
    handler: 'local' | 'engine';
}
/**
 * Parse a user input string that starts with `/`.
 * Returns null if the input doesn't start with `/` or if the command is unknown.
 */
export declare function parseSlashCommand(input: string): ParsedCommand | null;
/**
 * Returns the full list of registered slash commands.
 */
export declare function getCommandList(): readonly SlashCommand[];
/**
 * Check if a command name (or alias) is a local command.
 */
export declare function isLocalCommand(name: string): boolean;
/**
 * Format the command list as a displayable string for the /help output.
 */
export declare function formatHelpText(): string;
