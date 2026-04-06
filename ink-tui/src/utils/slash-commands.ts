/**
 * Slash command parser and registry.
 *
 * Handles the mapping between user-typed slash commands and their
 * dispatch targets (local TUI handlers vs engine bridge).
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Command registry
// ---------------------------------------------------------------------------

const COMMANDS: readonly SlashCommand[] = [
  // ── Local (handled by Ink TUI directly) ──
  { name: 'help',       aliases: ['h', '?'],    description: 'Show available commands',                handler: 'local' },
  { name: 'clear',      aliases: ['cls'],       description: 'Clear chat history',                    handler: 'local' },
  { name: 'resume',     aliases: ['r'],         description: 'Resume last session',                   handler: 'local' },
  { name: 'exit',       aliases: ['quit', 'q'], description: 'Exit OpenAnalyst CLI',                  handler: 'local' },
  { name: 'sidebar',    aliases: [],            description: 'Toggle sidebar visibility',             handler: 'local' },
  { name: 'vim',        aliases: [],            description: 'Toggle vim mode on/off',                handler: 'local' },

  // ── Core engine commands ──
  { name: 'model',      aliases: ['m'],         description: 'Change the AI model',                   handler: 'engine' },
  { name: 'agents',     aliases: ['a'],         description: 'List available agents',                 handler: 'engine' },
  { name: 'skills',     aliases: ['sk'],        description: 'List available skills',                 handler: 'engine' },
  { name: 'memory',     aliases: ['mem'],       description: 'Show memory usage',                     handler: 'engine' },
  { name: 'config',     aliases: ['cfg'],       description: 'Show/edit configuration',               handler: 'engine' },
  { name: 'init',       aliases: [],            description: 'Initialize project',                    handler: 'engine' },
  { name: 'status',     aliases: ['st'],        description: 'Show current status',                   handler: 'engine' },
  { name: 'compact',    aliases: [],            description: 'Compact conversation context',          handler: 'engine' },
  { name: 'feedback',   aliases: ['fb'],        description: 'Submit feedback or correction',         handler: 'engine' },
  { name: 'export',     aliases: [],            description: 'Export session to file',                handler: 'engine' },
  { name: 'version',    aliases: ['v'],         description: 'Show CLI version',                      handler: 'engine' },
  { name: 'cost',       aliases: [],            description: 'Show token usage and cost',             handler: 'engine' },
  { name: 'tokens',     aliases: [],            description: 'Show token count for context',          handler: 'engine' },
  { name: 'session',    aliases: [],            description: 'Session management (list, switch)',     handler: 'engine' },
  { name: 'doctor',     aliases: [],            description: 'Diagnose system and configuration',    handler: 'engine' },
  { name: 'login',      aliases: [],            description: 'Authenticate with a provider (/login <provider> <key>)', handler: 'local' },
  { name: 'logout',     aliases: [],            description: 'Log out of a provider (/logout <provider|all>)', handler: 'local' },
  { name: 'models',     aliases: [],            description: 'List all available models from configured providers', handler: 'local' },
  { name: 'mcp',        aliases: [],            description: 'Manage MCP server connections',        handler: 'engine' },
  { name: 'plugins',    aliases: ['plugin', 'marketplace'], description: 'Browse and install plugins', handler: 'engine' },
  { name: 'hooks',      aliases: [],            description: 'Manage event hooks',                   handler: 'engine' },
  { name: 'trust',      aliases: [],            description: 'Manage trusted directories',           handler: 'engine' },
  { name: 'rules',      aliases: [],            description: 'Manage project rules',                 handler: 'engine' },
  { name: 'openanalyst',aliases: ['oa'],        description: 'OpenAnalyst system info',              handler: 'engine' },

  // ── Diff & code review ──
  { name: 'diff',       aliases: ['d'],         description: 'Show recent diffs',                     handler: 'engine' },
  { name: 'diff-review',aliases: ['dr'],        description: 'AI-powered diff review',               handler: 'engine' },

  // ── Git workflow ──
  { name: 'branch',     aliases: ['br'],        description: 'Create or switch branches',            handler: 'engine' },
  { name: 'worktree',   aliases: ['wt'],        description: 'Git worktree management',              handler: 'engine' },
  { name: 'commit',     aliases: ['ci'],        description: 'Create a commit with AI message',      handler: 'engine' },
  { name: 'commit-push-pr', aliases: ['cpp'],   description: 'Commit, push, and create PR',          handler: 'engine' },
  { name: 'pr',         aliases: [],            description: 'Create or manage pull requests',       handler: 'engine' },
  { name: 'issue',      aliases: [],            description: 'Create or manage GitHub issues',       handler: 'engine' },
  { name: 'changelog',  aliases: [],            description: 'Generate changelog from commits',      handler: 'engine' },

  // ── AI & planning ──
  { name: 'think',      aliases: [],            description: 'Extended thinking mode',               handler: 'engine' },
  { name: 'effort',     aliases: [],            description: 'Set effort budget for next prompt',    handler: 'engine' },
  { name: 'route',      aliases: [],            description: 'Set model routing for action type',    handler: 'engine' },
  { name: 'ultraplan',  aliases: ['up'],        description: 'Deep multi-step planning mode',        handler: 'engine' },
  { name: 'bughunter',  aliases: ['bh'],        description: 'Find and fix bugs in codebase',       handler: 'engine' },
  { name: 'explore',    aliases: [],            description: 'Explore codebase structure',           handler: 'engine' },
  { name: 'swarm',      aliases: [],            description: 'Multi-agent swarm execution',          handler: 'engine' },
  { name: 'teleport',   aliases: [],            description: 'Jump to file or symbol',               handler: 'engine' },

  // ── Context management ──
  { name: 'context',    aliases: ['ctx'],       description: 'View/manage context files',            handler: 'engine' },
  { name: 'add-dir',    aliases: [],            description: 'Add directory to context',             handler: 'engine' },
  { name: 'undo',       aliases: [],            description: 'Undo last action',                     handler: 'engine' },

  // ── Output & generation ──
  { name: 'image',      aliases: ['img'],       description: 'Generate image from prompt',           handler: 'engine' },
  { name: 'voice',      aliases: [],            description: 'Voice input settings',                 handler: 'engine' },
  { name: 'speak',      aliases: [],            description: 'Text-to-speech output',                handler: 'engine' },
  { name: 'diagram',    aliases: ['diag'],      description: 'Generate diagram from description',    handler: 'engine' },
  { name: 'translate',  aliases: ['tr'],        description: 'Translate text between languages',     handler: 'engine' },
  { name: 'json',       aliases: [],            description: 'Output structured JSON response',      handler: 'engine' },
  { name: 'output-style',aliases: ['style'],    description: 'Set output formatting style',          handler: 'engine' },

  // ── Browser & Playwright ──
  { name: 'dev',        aliases: ['browser'],   description: 'Open browser with Playwright',         handler: 'engine' },
  { name: 'scrape',     aliases: [],            description: 'Scrape web page content',              handler: 'engine' },
  { name: 'vision',     aliases: [],            description: 'Analyze image or screenshot',          handler: 'engine' },

  // ── Knowledge base ──
  { name: 'knowledge',  aliases: ['kb'],        description: 'Query knowledge base directly',        handler: 'engine' },

  // ── Misc ──
  { name: 'ask',        aliases: [],            description: 'Ask a question to specific agent',     handler: 'engine' },
  { name: 'user-prompt',aliases: [],            description: 'Set custom system prompt',             handler: 'engine' },
  { name: 'debug-tool-call', aliases: ['dtc'],  description: 'Debug last tool call details',         handler: 'engine' },
];

// Build a lookup map: name/alias -> SlashCommand
const LOOKUP = new Map<string, SlashCommand>();
for (const cmd of COMMANDS) {
  LOOKUP.set(cmd.name, cmd);
  for (const alias of cmd.aliases) {
    LOOKUP.set(alias, cmd);
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Parse a user input string that starts with `/`.
 * Returns null if the input doesn't start with `/` or if the command is unknown.
 */
export function parseSlashCommand(input: string): ParsedCommand | null {
  const trimmed = input.trim();
  if (!trimmed.startsWith('/')) return null;

  const withoutSlash = trimmed.slice(1);
  const spaceIdx = withoutSlash.indexOf(' ');

  const rawName = spaceIdx === -1 ? withoutSlash : withoutSlash.slice(0, spaceIdx);
  const args = spaceIdx === -1 ? '' : withoutSlash.slice(spaceIdx + 1).trim();

  const name = rawName.toLowerCase();
  const cmd = LOOKUP.get(name);

  if (cmd) {
    return {
      name: cmd.name,
      args,
      handler: cmd.handler,
    };
  }

  // Unknown command — forward to engine as a slash command anyway.
  // The Rust engine has its own command registry and may support commands
  // not yet registered in the Ink TUI (future-proofing).
  return {
    name,
    args,
    handler: 'engine',
  };
}

/**
 * Returns the full list of registered slash commands.
 */
export function getCommandList(): readonly SlashCommand[] {
  return COMMANDS;
}

/**
 * Check if a command name (or alias) is a local command.
 */
export function isLocalCommand(name: string): boolean {
  const cmd = LOOKUP.get(name.toLowerCase());
  return cmd?.handler === 'local';
}

/**
 * Format the command list as a displayable string for the /help output.
 */
export function formatHelpText(): string {
  const lines: string[] = [
    'Available commands:',
    '',
  ];

  for (const cmd of COMMANDS) {
    const aliasStr = cmd.aliases.length > 0 ? ` (${cmd.aliases.join(', ')})` : '';
    lines.push(`  /${cmd.name}${aliasStr} -- ${cmd.description}`);
  }

  lines.push('');
  lines.push('Keybindings:');
  lines.push('  Esc         Enter scroll mode');
  lines.push('  Ctrl+C      Cancel agent / quit');
  lines.push('  Ctrl+P      Cycle permission mode');
  lines.push('  F2          Toggle sidebar');
  lines.push('  Ctrl+B      Run in background');
  lines.push('  Ctrl+L      Clear chat');
  lines.push('  j/k         Navigate in scroll mode');
  lines.push('  Enter       Expand/collapse card');
  lines.push('  y           Copy message to clipboard');

  return lines.join('\n');
}
