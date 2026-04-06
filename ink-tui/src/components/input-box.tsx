/**
 * InputBox -- minimal input area matching the Ratatui design.
 *
 * Visual structure:
 *
 *   [icon] Enter to send . Ctrl+P mode  [I] --------[No-Git]
 *   |  user types here...
 *   |                                           API credits
 *
 * Features:
 *   - Top line: prompt icon + hint text + vim indicator + horizontal rule + branch badge
 *   - The horizontal rule color changes by permission mode (blue/yellow/green/red)
 *   - Below: clean text input area with NO box border
 *   - Bottom-right: credit balance + MCP count
 *   - Multi-line text input with basic editing
 *   - Vim mode: normal / insert mode tracking
 *   - History navigation (Up/Down)
 *   - Enter to submit, dynamic height 3-8 lines
 *   - Disabled state during streaming/agent running
 *
 * All colors from useTheme() semantic tokens.
 * Keypress subscription at priority 3 (input mode).
 */

import React, { useState, useCallback, useRef, useEffect } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import { useUIState, useUIActions } from '../contexts/ui-state-context.js';
import { useTerminal } from '../contexts/terminal-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
import { useInputHistory } from '../hooks/use-input-history.js';
import { useChatActions } from '../contexts/chat-context.js';
import { useEngine } from '../engine/engine-context.js';
import { Command } from '../key/commands.js';
import { ModeBadge } from './mode-badge.js';
import { ContextFileTags } from './context-file-tags.js';
import { parseSlashCommand, formatHelpText, getCommandList } from '../utils/slash-commands.js';
import { Autocomplete, type AutocompleteItem } from './autocomplete.js';
import { useSessionContext } from '../contexts/session-context.js';
import { useVoice } from '../hooks/use-voice.js';
import { useCredits } from '../hooks/use-credits.js';
import { credentialManager, PROVIDER_CONFIG } from '../utils/credential-manager.js';
import { clearCreditCache } from '../utils/credit-checker.js';
import { providerPreferences } from '../utils/provider-preferences.js';
import type { PermissionMode } from '../types/messages.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const INPUT_MIN_HEIGHT = 3;
const INPUT_MAX_HEIGHT = 8;

// ---------------------------------------------------------------------------
// Vim mode
// ---------------------------------------------------------------------------

type VimMode = 'normal' | 'insert';

// ---------------------------------------------------------------------------
// Permission mode helpers
// ---------------------------------------------------------------------------

interface PermissionConfig {
  icon: string;
  label: string;
  borderColorKey: keyof typeof BORDER_KEY_MAP;
}

const BORDER_KEY_MAP = {
  default: 'default',
  plan: 'plan',
  acceptEdits: 'acceptEdits',
  danger: 'danger',
  streaming: 'streaming',
  agentRunning: 'agentRunning',
} as const;

const PERMISSION_CONFIGS: Record<PermissionMode, PermissionConfig> = {
  'prompt': { icon: '\u276F', label: 'Default', borderColorKey: 'default' },
  'read-only': { icon: '\u25C8', label: 'Plan', borderColorKey: 'plan' },
  'workspace-write': { icon: '\u270E', label: 'Accept Edits', borderColorKey: 'acceptEdits' },
  'danger-full-access': { icon: '\u26A1', label: 'Danger', borderColorKey: 'danger' },
};

// ---------------------------------------------------------------------------
// /login and /logout command handlers
// ---------------------------------------------------------------------------

interface ChatActionsLike {
  pushSystem(text: string, type: 'info' | 'error'): void;
}

interface CreditsLike {
  refresh(): void;
}

interface EngineLike {
  bridge: { slashCommand(cmd: string): void };
}

function handleLoginCommand(
  args: string,
  chatActions: ChatActionsLike,
  credits: CreditsLike,
  engine: EngineLike,
): void {
  // /login status -- show all saved credentials
  if (args === 'status') {
    const creds = credentialManager.listCredentials();
    if (creds.length === 0) {
      chatActions.pushSystem(
        'No API keys configured.\n\nUse /login <provider> <api-key> to add one.',
        'info',
      );
      return;
    }

    const lines = ['Saved credentials:', ''];
    for (const cred of creds) {
      const config = PROVIDER_CONFIG[cred.provider];
      const masked =
        cred.apiKey.length > 8
          ? `${cred.apiKey.slice(0, 4)}...${cred.apiKey.slice(-4)}`
          : '****';
      lines.push(
        `  ${config?.displayName ?? cred.provider}: ${masked} (${cred.envVarName}, source: ${cred.source})`,
      );
    }
    lines.push('');
    lines.push('Checking credits...');
    chatActions.pushSystem(lines.join('\n'), 'info');

    // Fetch credits asynchronously and display
    void credentialManager.fetchAllCredits().then((creditMap) => {
      const creditLines = ['Provider credits:', ''];
      for (const [provider, balance] of Object.entries(creditMap)) {
        const config = PROVIDER_CONFIG[provider];
        creditLines.push(`  ${config?.displayName ?? provider}: ${balance}`);
      }
      chatActions.pushSystem(creditLines.join('\n'), 'info');
    });
    return;
  }

  // /login gemini oauth -- delegate to Rust engine for browser-based OAuth
  if (args === 'gemini oauth') {
    engine.bridge.slashCommand('/login gemini oauth');
    return;
  }

  // /login (no args) -- show usage
  if (!args) {
    const lines = [
      'Available providers:',
      '',
      '  1. openai      (OPENAI_API_KEY)     -- sk-...',
      '  2. anthropic   (ANTHROPIC_API_KEY)   -- sk-ant-...',
      '  3. gemini      (GEMINI_API_KEY)      -- AIza... or OAuth',
      '  4. xai         (XAI_API_KEY)         -- xai-...',
      '  5. openrouter  (OPENROUTER_API_KEY)  -- sk-or-...',
      '  6. bedrock     (BEDROCK_API_KEY)',
      '  7. stability   (STABILITY_API_KEY)',
      '  8. openanalyst (OPENANALYST_AUTH_TOKEN)',
      '',
      'Usage:',
      '  /login <provider> <api-key>  Save an API key',
      '  /login <api-key>             Auto-detect provider from key prefix',
      '  /login gemini oauth          Browser-based OAuth for Google Gemini',
      '  /login status                Show all saved credentials',
      '',
      'Example:',
      '  /login openai sk-abc123...',
      '  /login sk-ant-abc123...       (auto-detects Anthropic)',
    ];
    chatActions.pushSystem(lines.join('\n'), 'info');
    return;
  }

  // /login <provider> <key> or /login <key>
  const parts = args.split(/\s+/);
  let provider: string | null = null;
  let apiKey: string;

  if (parts.length >= 2) {
    // Explicit provider + key
    const providerArg = parts[0]!.toLowerCase();
    apiKey = parts.slice(1).join('');

    if (PROVIDER_CONFIG[providerArg]) {
      provider = providerArg;
    } else {
      chatActions.pushSystem(
        `Unknown provider: "${providerArg}". Use /login to see available providers.`,
        'error',
      );
      return;
    }
  } else {
    // Auto-detect from key prefix
    apiKey = parts[0]!;
    provider = credentialManager.detectProvider(apiKey);

    if (!provider) {
      chatActions.pushSystem(
        'Could not auto-detect provider from key prefix.\nUse /login <provider> <key> instead.',
        'error',
      );
      return;
    }
  }

  const config = PROVIDER_CONFIG[provider]!;

  // Save the key and set as default provider
  void credentialManager.saveApiKey(provider, apiKey).then(() => {
    clearCreditCache();
    credits.refresh();

    // Set this provider as the user's default
    providerPreferences.setDefaultProvider(provider);
    providerPreferences.invalidateCache();

    const masked =
      apiKey.length > 8
        ? `${apiKey.slice(0, 4)}...${apiKey.slice(-4)}`
        : '****';

    chatActions.pushSystem(
      `${config.displayName} API key saved to global .env, SQLite, and credentials.json.\n` +
        `  Key: ${masked}\n` +
        `  Env: ${config.envVar}\n\n` +
        `\u2713 ${config.displayName} set as your default AI provider\n\n` +
        `Checking credits...`,
      'info',
    );

    // Fetch and display credits
    void credentialManager.fetchCredits(provider).then((creditStr) => {
      chatActions.pushSystem(
        `${config.displayName}: ${creditStr}`,
        'info',
      );
    });
  }).catch((err: unknown) => {
    chatActions.pushSystem(
      `Failed to save API key: ${err instanceof Error ? err.message : String(err)}`,
      'error',
    );
  });
}

function handleLogoutCommand(
  args: string,
  chatActions: ChatActionsLike,
  credits: CreditsLike,
): void {
  if (!args) {
    chatActions.pushSystem(
      'Usage:\n  /logout <provider>  Remove a specific provider\n  /logout all         Remove all credentials (keeps Gemini OAuth)',
      'info',
    );
    return;
  }

  if (args === 'all') {
    void credentialManager.removeAll().then(() => {
      clearCreditCache();
      credits.refresh();
      chatActions.pushSystem(
        'All credentials removed.\n\nYou need an API key to use OpenAnalyst.\nRun /login <provider> <key> to configure a new one.\n\nOr type /exit to return to your terminal.',
        'error',
      );
    });
    return;
  }

  const provider = args.toLowerCase();
  if (!PROVIDER_CONFIG[provider]) {
    chatActions.pushSystem(
      `Unknown provider: "${args}". Use /login status to see configured providers.`,
      'error',
    );
    return;
  }

  void credentialManager.removeCredential(provider).then(() => {
    clearCreditCache();
    credits.refresh();
    chatActions.pushSystem(
      `${PROVIDER_CONFIG[provider]!.displayName} credentials removed from all locations.`,
      'info',
    );
  });
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface InputBoxProps {
  /** Callback when the user submits a prompt. */
  onSubmit?: (text: string) => void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function InputBox({ onSubmit }: InputBoxProps): React.ReactElement {
  const { colors } = useTheme();
  const ui = useUIState();
  const actions = useUIActions();
  const terminal = useTerminal();
  const history = useInputHistory();
  const chatActions = useChatActions();
  const engine = useEngine();
  const session = useSessionContext();
  const voice = useVoice();
  const credits = useCredits();

  // Text state
  const [text, setText] = useState('');
  const [cursorPos, setCursorPos] = useState(0);
  const [vimEnabled, setVimEnabled] = useState(false);
  const [vimMode, setVimMode] = useState<VimMode>('insert');

  // Track whether we are actively navigating history
  const isNavigatingHistory = useRef(false);

  // Message queue — when user types while AI is running, queue it
  const [queuedMessage, setQueuedMessage] = useState<string | null>(null);

  // Autocomplete state for slash commands
  const [acVisible, setAcVisible] = useState(false);
  const [acItems, setAcItems] = useState<AutocompleteItem[]>([]);
  const [acIndex, setAcIndex] = useState(0);

  // Build the full autocomplete list from slash commands (memoized)
  const allSlashCommands = React.useMemo<AutocompleteItem[]>(() => {
    return getCommandList().map((cmd) => ({
      name: `/${cmd.name}`,
      description: cmd.description,
    }));
  }, []);

  // Derived state
  const isAgentBusy = ui.inputMode === 'agent_running' || ui.inputMode === 'streaming' || ui.inputMode === 'plan_running';
  const permConfig = PERMISSION_CONFIGS[ui.permissionMode];

  // Determine line color (the horizontal rule and prompt icon)
  const lineColor = (() => {
    if (ui.inputMode === 'streaming') return colors.border.input.streaming;
    if (ui.inputMode === 'agent_running') return colors.border.input.agentRunning;
    if (ui.inputMode === 'plan_running') return colors.border.input.plan;
    return colors.border.input[permConfig.borderColorKey];
  })();

  // Dynamic height based on content lines
  const lineCount = text.split('\n').length;
  const dynamicHeight = Math.min(Math.max(lineCount + 2, INPUT_MIN_HEIGHT), INPUT_MAX_HEIGHT);

  // Update autocomplete dropdown as user types
  useEffect(() => {
    if (text.startsWith('/') && !text.includes('\n')) {
      const query = text.toLowerCase();
      const filtered = allSlashCommands.filter((cmd) =>
        cmd.name.toLowerCase().startsWith(query),
      );
      if (filtered.length > 0 && text.length > 1) {
        setAcItems(filtered);
        setAcIndex(0);
        setAcVisible(true);
      } else if (text === '/') {
        // Show all commands when just "/" is typed
        setAcItems(allSlashCommands);
        setAcIndex(0);
        setAcVisible(true);
      } else {
        setAcVisible(false);
      }
    } else {
      setAcVisible(false);
    }
  }, [text, allSlashCommands]);

  // Auto-send queued message when agent finishes
  useEffect(() => {
    if (!isAgentBusy && queuedMessage) {
      const queued = queuedMessage;
      setQueuedMessage(null);
      // Small delay to let UI settle
      setTimeout(() => {
        onSubmit?.(queued);
      }, 100);
    }
  }, [isAgentBusy, queuedMessage, onSubmit]);

  // Submit handler -- routes slash commands locally or to engine
  const handleSubmit = useCallback(() => {
    const trimmed = text.trim();
    if (!trimmed) return;

    // If agent is busy, queue the message for later
    if (isAgentBusy) {
      setQueuedMessage(trimmed);
      chatActions.pushSystem(`Message queued: "${trimmed.length > 60 ? trimmed.slice(0, 60) + '...' : trimmed}"`, 'info');
      setText('');
      setCursorPos(0);
      return;
    }

    history.push(trimmed);

    // Check for slash commands — support multiple commands in one input
    // e.g., "/clear /model gpt-4o" or "/compact /status"
    if (trimmed.startsWith('/')) {
      // Split multiple commands: each /command starts a new command
      const commandParts = trimmed.match(/\/[^\\/]+/g) || [trimmed];
      const isMulti = commandParts.length > 1;

      // Show the full user input in chat once
      chatActions.pushUser(trimmed);

      for (const cmdPart of commandParts) {
        const singleCmd = cmdPart.trim();
        if (!singleCmd.startsWith('/')) continue;

        const parsed = parseSlashCommand(singleCmd);
        if (!parsed) continue;

        if (isMulti) {
          // Show each sub-command as a system message for clarity
          chatActions.pushSystem(`Running: ${singleCmd}`, 'info');
        }

        if (parsed.handler === 'local') {
          // Handle locally
          switch (parsed.name) {
            case 'help':
              chatActions.pushSystem(formatHelpText(), 'info');
              break;
            case 'clear':
              engine.clearChat();
              break;
            case 'login': {
              const args = parsed.args.trim();
              handleLoginCommand(args, chatActions, credits, engine);
              break;
            }
            case 'logout': {
              const args = parsed.args.trim();
              handleLogoutCommand(args, chatActions, credits);
              break;
            }
            case 'models': {
              const output = providerPreferences.formatModelsOutput(ui.currentModel);
              chatActions.pushSystem(output, 'info');
              break;
            }
            case 'exit':
            case 'quit':
              engine.quit();
              process.exit(0);
              break;

            case 'sidebar':
              actions.toggleSidebar();
              break;

            case 'permissions':
            case 'perm': {
              const permArgs = parsed.args.trim();
              if (permArgs) {
                // Cycle to the requested mode
                const modeNames: Record<string, string> = {
                  'default': 'prompt', 'prompt': 'prompt',
                  'plan': 'read-only', 'read-only': 'read-only',
                  'accept-edits': 'workspace-write', 'workspace': 'workspace-write', 'workspace-write': 'workspace-write',
                  'danger': 'danger-full-access', 'full': 'danger-full-access', 'full-access': 'danger-full-access', 'allow': 'danger-full-access',
                };
                const target = modeNames[permArgs.toLowerCase()];
                if (target) {
                  // Cycle until we reach the target mode (max 4 cycles)
                  for (let i = 0; i < 4; i++) {
                    if (ui.permissionMode === target) break;
                    actions.cyclePermissionMode();
                  }
                  chatActions.pushSystem(`Permission mode: ${permArgs}`, 'info');
                } else {
                  chatActions.pushSystem(
                    `Unknown mode: "${permArgs}"\n\nAvailable modes:\n  default — Ask before running tools\n  plan — Read-only, planning only\n  accept-edits — Auto-approve file edits\n  danger — Full auto-approve (no prompts)\n\nOr use Ctrl+P to cycle.`,
                    'error',
                  );
                }
              } else {
                // No args — show current and cycle
                const modeLabels: Record<string, string> = {
                  'prompt': 'Default (ask before tools)',
                  'read-only': 'Plan (read-only)',
                  'workspace-write': 'Accept Edits (auto-approve file changes)',
                  'danger-full-access': 'Danger (full auto-approve)',
                };
                const current = modeLabels[ui.permissionMode] || ui.permissionMode;
                actions.cyclePermissionMode();
                chatActions.pushSystem(`Permission mode: ${current}\nCycled to next mode. Use Ctrl+P to cycle, or /permissions <mode> to set directly.`, 'info');
              }
              break;
            }

            case 'vim': {
              const newVimState = !vimEnabled;
              setVimEnabled(newVimState);
              if (!newVimState) {
                setVimMode('insert'); // Reset to insert when disabling
              }
              chatActions.pushSystem(
                newVimState
                  ? 'Vim mode: ON\n\nControls:\n  Esc — NORMAL mode\n  i — INSERT mode\n  h/j/k/l — cursor movement\n  0/$ — start/end of line\n  x — delete char\n  o — new line below'
                  : 'Vim mode: OFF\n\nInput behaves like a normal text box.',
                'info',
              );
              break;
            }

            case 'resume': {
              const args = parsed.args.trim();
              if (args === 'list') {
                // Show recent sessions
                const sessions = session.listSessions();
                if (sessions.length === 0) {
                  chatActions.pushSystem('No saved sessions found.', 'info');
                } else {
                  const lines = ['Recent sessions:', ''];
                  for (const s of sessions.slice(0, 10)) {
                    const date = new Date(s.updatedAt).toLocaleString();
                    lines.push(`  ${s.id}`);
                    lines.push(`    ${s.summary.slice(0, 80)} (${s.messageCount} messages, ${date})`);
                    lines.push('');
                  }
                  lines.push('Use /resume <session-id> to load a specific session.');
                  chatActions.pushSystem(lines.join('\n'), 'info');
                }
              } else if (args.length > 0) {
                // Load specific session
                const loaded = session.loadSession(args);
                if (loaded) {
                  chatActions.pushSystem(`Session resumed: ${args}`, 'info');
                } else {
                  chatActions.pushSystem(`Session not found: ${args}`, 'error');
                }
              } else {
                // Resume most recent
                const resumed = session.resumeLatest();
                if (resumed) {
                  chatActions.pushSystem('Resumed most recent session.', 'info');
                } else {
                  chatActions.pushSystem('No saved sessions found. Start chatting to create one.', 'info');
                }
              }
              break;
            }
          }
        } else {
          // Send to engine via bridge
          engine.bridge.slashCommand(singleCmd);
        }
      } // end for loop over commandParts

      setText('');
      setCursorPos(0);
      setVimMode('insert');
      isNavigatingHistory.current = false;
      return;
    }

    // Check if any API key is configured before sending to AI
    const hasKey = credentialManager.listCredentials().length > 0;
    if (!hasKey) {
      chatActions.pushUser(trimmed);
      chatActions.pushSystem(
        'No API key configured. Run /login <provider> <key> to get started.\n\nExample:\n  /login openai sk-abc123...\n  /login anthropic sk-ant-abc123...',
        'error',
      );
      setText('');
      setCursorPos(0);
      return;
    }

    onSubmit?.(trimmed);
    setText('');
    setCursorPos(0);
    setVimMode('insert');
    isNavigatingHistory.current = false;
  }, [text, isAgentBusy, history, onSubmit, chatActions, engine]);

  // Insert a character at cursor position
  const insertChar = useCallback((char: string) => {
    setText((prev) => {
      const next = prev.slice(0, cursorPos) + char + prev.slice(cursorPos);
      setCursorPos((c) => c + char.length);
      return next;
    });
    isNavigatingHistory.current = false;
  }, [cursorPos]);

  // Backspace
  const handleBackspace = useCallback(() => {
    if (cursorPos === 0) {
      // If input is empty and there are context files, remove the last one
      if (text.length === 0 && ui.contextFiles.length > 0) {
        const lastFile = ui.contextFiles[ui.contextFiles.length - 1]!;
        actions.removeContextFile(lastFile);
      }
      return;
    }
    setText((prev) => prev.slice(0, cursorPos - 1) + prev.slice(cursorPos));
    setCursorPos((c) => Math.max(0, c - 1));
    isNavigatingHistory.current = false;
  }, [cursorPos, text, ui.contextFiles, actions]);

  // Delete key
  const handleDelete = useCallback(() => {
    if (cursorPos >= text.length) return;
    setText((prev) => prev.slice(0, cursorPos) + prev.slice(cursorPos + 1));
  }, [cursorPos, text.length]);

  // Cursor movement
  const moveLeft = useCallback(() => setCursorPos((c) => Math.max(0, c - 1)), []);
  const moveRight = useCallback(() => setCursorPos((c) => Math.min(text.length, c + 1)), [text.length]);
  const moveHome = useCallback(() => {
    // Move to start of current line
    const lineStart = text.lastIndexOf('\n', cursorPos - 1) + 1;
    setCursorPos(lineStart);
  }, [text, cursorPos]);
  const moveEnd = useCallback(() => {
    // Move to end of current line
    let lineEnd = text.indexOf('\n', cursorPos);
    if (lineEnd === -1) lineEnd = text.length;
    setCursorPos(lineEnd);
  }, [text, cursorPos]);

  // History navigation
  const historyUp = useCallback(() => {
    const entry = history.goUp(text);
    if (entry !== null) {
      setText(entry);
      setCursorPos(entry.length);
      isNavigatingHistory.current = true;
    }
  }, [history, text]);

  const historyDown = useCallback(() => {
    const entry = history.goDown();
    if (entry !== null) {
      setText(entry);
      setCursorPos(entry.length);
      isNavigatingHistory.current = true;
    } else {
      isNavigatingHistory.current = false;
    }
  }, [history]);

  // Keypress handler at priority 3 (input mode)
  useKeypress(
    useCallback((input, key, command) => {
      // When agent is busy, still allow typing but block submit (handled in handleSubmit)
      // Don't block keypresses — user can type and queue messages

      // --- Vim normal mode handling (only when vim is enabled) ---
      if (vimEnabled && vimMode === 'normal') {
        // Mode transitions
        if (input === 'i') { setVimMode('insert'); return true; }
        if (input === 'I') { moveHome(); setVimMode('insert'); return true; }
        if (input === 'a') { moveRight(); setVimMode('insert'); return true; }
        if (input === 'A') { moveEnd(); setVimMode('insert'); return true; }
        if (input === 'o') {
          // Insert newline below and enter insert mode
          moveEnd();
          insertChar('\n');
          setVimMode('insert');
          return true;
        }

        // Navigation
        if (input === 'h' || key.leftArrow) { moveLeft(); return true; }
        if (input === 'l' || key.rightArrow) { moveRight(); return true; }
        if (input === 'j' || key.downArrow) { historyDown(); return true; }
        if (input === 'k' || key.upArrow) { historyUp(); return true; }
        if (input === '0') { moveHome(); return true; }
        if (input === '$') { moveEnd(); return true; }

        // Editing in normal mode
        if (input === 'x') { handleDelete(); return true; }
        if (input === 'X') { handleBackspace(); return true; }

        // Word-level delete (dw) -- simplified: delete to next space
        if (input === 'd') {
          // For simplicity, 'dd' clears the line
          // TODO: Full vim motion support
          return true;
        }

        // Don't consume other keys in normal mode -- let globals handle them
        return false;
      }

      // --- Insert mode handling ---

      // Escape — always propagate (scroll mode, etc.)
      if (key.escape) {
        return false;
      }

      // Ctrl+M — toggle vim normal/insert mode (only when vim enabled)
      if (key.ctrl && input === 'm' && vimEnabled) {
        setVimMode(vimMode === 'normal' ? 'insert' : 'normal');
        return true;
      }

      // Command-based handling
      if (command === Command.SUBMIT) {
        handleSubmit();
        return true;
      }

      if (command === Command.HISTORY_UP) {
        // Only navigate history if text is single-line
        if (!text.includes('\n')) {
          historyUp();
          return true;
        }
        return false;
      }

      if (command === Command.HISTORY_DOWN) {
        if (!text.includes('\n')) {
          historyDown();
          return true;
        }
        return false;
      }

      if (command === Command.REMOVE_LAST_CONTEXT_FILE) {
        if (text.length === 0 && ui.contextFiles.length > 0) {
          const lastFile = ui.contextFiles[ui.contextFiles.length - 1]!;
          actions.removeContextFile(lastFile);
          return true;
        }
      }

      // Ctrl+B: run current input in background
      if (command === Command.RUN_IN_BACKGROUND) {
        if (text.trim().length > 0) {
          const trimmed = text.trim();
          history.push(trimmed);
          chatActions.pushUser(trimmed);
          engine.bridge.sendAction('run_in_background', { text: trimmed });
          chatActions.pushSystem('Running in background...', 'info');
          setText('');
          setCursorPos(0);
          setVimMode('insert');
          isNavigatingHistory.current = false;
          return true;
        }
        // Empty input -- let global handler deal with it
        return false;
      }

      // Voice recording: Space on empty input starts recording
      if (command === Command.START_VOICE_RECORDING) {
        if (text.length === 0 && !voice.isRecording) {
          voice.startRecording();
          return true;
        }
      }

      // Special keys
      if (key.backspace || key.delete) {
        if (key.backspace) handleBackspace();
        else handleDelete();
        return true;
      }

      if (key.leftArrow) { moveLeft(); return true; }
      if (key.rightArrow) { moveRight(); return true; }
      if (key.upArrow) {
        if (!text.includes('\n')) { historyUp(); return true; }
        // TODO: multi-line cursor up
        return true;
      }
      if (key.downArrow) {
        if (!text.includes('\n')) { historyDown(); return true; }
        // TODO: multi-line cursor down
        return true;
      }

      // Return with shift -> newline
      if (key.return && key.shift) {
        insertChar('\n');
        return true;
      }

      // Regular Return -> submit
      if (key.return) {
        handleSubmit();
        return true;
      }

      // Tab -> insert spaces (or future autocomplete trigger)
      if (key.tab) {
        insertChar('  ');
        return true;
      }

      // Printable characters
      if (input && !key.ctrl && !key.meta) {
        insertChar(input);
        return true;
      }

      // Ctrl+U -> clear line
      if (key.ctrl && input === 'u') {
        setText('');
        setCursorPos(0);
        return true;
      }

      // Ctrl+A -> move to start
      if (key.ctrl && input === 'a') {
        setCursorPos(0);
        return true;
      }

      // Ctrl+E -> toggle sidebar (handled by global handler, don't consume here)

      // Ctrl+W -> delete word backward
      if (key.ctrl && input === 'w') {
        const before = text.slice(0, cursorPos);
        const trimmed = before.replace(/\s+$/, '');
        const wordStart = Math.max(0, trimmed.lastIndexOf(' ') + 1);
        setText(text.slice(0, wordStart) + text.slice(cursorPos));
        setCursorPos(wordStart);
        return true;
      }

      return false;
    }, [
      isAgentBusy, vimMode, text, cursorPos, ui.contextFiles,
      handleSubmit, insertChar, handleBackspace, handleDelete,
      moveLeft, moveRight, moveHome, moveEnd,
      historyUp, historyDown, actions, voice,
    ]),
    { isActive: true, priority: 3 },
  );

  // Voice recording keypress handler (priority 7 -- intercepts during recording)
  useKeypress(
    useCallback((input, key, command) => {
      // VOICE_STOP is bound to Space, Escape, and Return, but the
      // command resolver maps these to earlier-in-enum commands
      // (START_VOICE_RECORDING, ENTER_SCROLL_MODE, SUBMIT). Check
      // raw keys instead of the resolved command.
      if (key.escape) {
        // Esc cancels without transcribing
        voice.cancelRecording();
        return true;
      }
      if (input === ' ' || key.return) {
        // Space or Enter: stop + transcribe
        voice.stopRecording().then((transcript) => {
          if (transcript) {
            setText(transcript);
            setCursorPos(transcript.length);
            setVimMode('insert');
          }
        });
        return true;
      }
      // Block all other keys during voice recording
      return true;
    }, [voice]),
    { isActive: voice.isRecording, priority: 7 },
  );

  // Autocomplete keypress handler (priority 5 -- above input, below dialogs)
  const handleAcSelect = useCallback((item: AutocompleteItem) => {
    // Replace the current text with the selected command
    const hasSpace = item.name.length < text.length;
    setText(item.name + (hasSpace ? '' : ' '));
    setCursorPos(item.name.length + 1);
    setAcVisible(false);
  }, [text]);

  useKeypress(
    useCallback((_input, key, command) => {
      if (command === Command.AC_NEXT || key.downArrow) {
        setAcIndex((prev) => Math.min(acItems.length - 1, prev + 1));
        return true;
      }
      if (command === Command.AC_PREV || key.upArrow) {
        setAcIndex((prev) => Math.max(0, prev - 1));
        return true;
      }
      if (command === Command.AC_ACCEPT || key.tab) {
        const item = acItems[acIndex];
        if (item) {
          setText(item.name + ' ');
          setCursorPos(item.name.length + 1);
          setAcVisible(false);
        }
        return true;
      }
      if (command === Command.AC_ACCEPT_SUBMIT || key.return) {
        const item = acItems[acIndex];
        if (item) {
          // Enter on autocomplete = submit the command directly
          setText(item.name);
          setAcVisible(false);
          // Submit after a tick so state updates
          setTimeout(() => handleSubmit(), 0);
        }
        return true;
      }
      if (command === Command.AC_DISMISS || key.escape) {
        setAcVisible(false);
        return true;
      }
      return false;
    }, [acItems, acIndex]),
    { isActive: acVisible, priority: 5 },
  );

  // -------------------------------------------------------------------------
  // Render helpers
  // -------------------------------------------------------------------------

  // Helper: render text with slash commands highlighted in orange bold
  const slashCmdColor = colors.text.slashCommand; // OA Orange

  const renderColorizedText = (txt: string, defaultColor: string): React.ReactElement[] => {
    // Find /command patterns and colorize them
    const parts: React.ReactElement[] = [];
    const regex = /(\/[a-zA-Z][\w-]*)/g;
    let lastIdx = 0;
    let matchArr: RegExpExecArray | null;
    let partIdx = 0;

    while ((matchArr = regex.exec(txt)) !== null) {
      // Text before the command
      if (matchArr.index > lastIdx) {
        parts.push(<Text key={`t${partIdx++}`} color={defaultColor}>{txt.slice(lastIdx, matchArr.index)}</Text>);
      }
      // The /command itself — orange bold
      parts.push(<Text key={`c${partIdx++}`} color={slashCmdColor} bold>{matchArr[1]}</Text>);
      lastIdx = matchArr.index + matchArr[1]!.length;
    }
    // Remaining text after last command
    if (lastIdx < txt.length) {
      parts.push(<Text key={`t${partIdx++}`} color={defaultColor}>{txt.slice(lastIdx)}</Text>);
    }
    if (parts.length === 0) {
      parts.push(<Text key="t0" color={defaultColor}>{txt}</Text>);
    }
    return parts;
  };

  // Get the visible text with cursor indicator
  const renderTextWithCursor = (): React.ReactElement[] => {
    if (text.length === 0) {
      // Show arrow prompt + cursor block + placeholder text
      return [
        <Text key="placeholder">
          <Text color={lineColor} bold>{'\u276F'} </Text>
          <Text backgroundColor={colors.text.accent} color="#000000">{' '}</Text>
          <Text color={colors.text.secondary} dimColor> Type a message or </Text>
          <Text color={slashCmdColor} dimColor>/command</Text>
        </Text>,
      ];
    }

    // Render arrow prompt + text with a visible cursor
    const lines = text.split('\n');
    const elements: React.ReactElement[] = [];

    let charIndex = 0;
    for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
      const line = lines[lineIdx]!;
      const lineStart = charIndex;
      const lineEnd = lineStart + line.length;

      if (cursorPos >= lineStart && cursorPos <= lineEnd) {
        // Cursor is on this line — split around cursor for highlighting
        const localCursor = cursorPos - lineStart;
        const before = line.slice(0, localCursor);
        const cursorChar = line[localCursor] ?? ' ';
        const after = line.slice(localCursor + 1);

        const arrow = lineIdx === 0 ? '\u276F ' : '  ';
        elements.push(
          <Text key={`line-${lineIdx}`}>
            <Text color={lineColor} bold>{arrow}</Text>
            {renderColorizedText(before, colors.text.primary)}
            <Text color={vimMode === 'normal' ? '#000000' : colors.text.primary}
                  backgroundColor={vimMode === 'normal' ? colors.text.primary : colors.text.accent}>
              {cursorChar}
            </Text>
            {renderColorizedText(after, colors.text.primary)}
          </Text>,
        );
      } else {
        const arrow = lineIdx === 0 ? '\u276F ' : '  ';
        elements.push(
          <Text key={`line-${lineIdx}`}>
            <Text color={lineColor} bold>{arrow}</Text>
            {renderColorizedText(line, colors.text.primary)}
          </Text>,
        );
      }

      charIndex = lineEnd + 1;
    }

    return elements;
  };

  // -------------------------------------------------------------------------
  // Top prompt line (the main visual change from the old design)
  // -------------------------------------------------------------------------

  const renderPromptLine = (): React.ReactElement => {
    // Build left portion: icon + hint + vim mode
    const leftParts: React.ReactElement[] = [];

    if (isAgentBusy) {
      const icon = ui.inputMode === 'streaming' ? '\u2819' : '\u25CF';
      const hintColor = ui.inputMode === 'streaming'
        ? colors.border.input.streaming
        : colors.border.input.agentRunning;
      const hintText = ui.inputMode === 'streaming'
        ? 'Responding... \u00B7 Ctrl+C to cancel'
        : `${ui.inputLabel || 'Running'} \u00B7 Ctrl+C to cancel`;

      leftParts.push(
        <Text key="disabled-icon" color={hintColor} bold>{icon} </Text>,
      );
      leftParts.push(
        <Text key="disabled-hint" color={colors.text.secondary}>{hintText} </Text>,
      );
    } else {
      leftParts.push(
        <Text key="icon" color={lineColor} bold>{permConfig.icon} </Text>,
      );
      leftParts.push(
        <Text key="hint" color={colors.text.secondary}>Enter to send {'\u00B7'} Ctrl+P mode </Text>,
      );
    }

    // Queued message indicator
    if (queuedMessage) {
      leftParts.push(
        <Text key="queued" color={colors.status.warning} bold> [queued] </Text>,
      );
    }

    // Build right portion: colorful badges embedded in the horizontal rule
    // Like Ratatui: ──── [⚡ Danger] [model] [No-Git] ────
    const rightParts: React.ReactElement[] = [];

    // Horizontal rule fills space between left text and badges
    rightParts.push(
      <Text key="rule" color={lineColor}>{'\u2500'.repeat(3)} </Text>,
    );

    // Permission mode badge — colorful, always shown
    rightParts.push(
      <ModeBadge
        key="mode"
        label={`${permConfig.icon} ${permConfig.label}`}
        bgColor={lineColor}
        textColor="#000000"
      />,
    );
    rightParts.push(<Text key="mode-sep"> </Text>);

    // Model badge — gray background, only if configured (not "Run /login")
    if (ui.currentModel && !ui.currentModel.startsWith('Run')) {
      rightParts.push(
        <ModeBadge
          key="model"
          label={ui.currentModel}
          bgColor={colors.background.badge.model}
          textColor={colors.text.primary}
        />,
      );
      rightParts.push(<Text key="model-sep"> </Text>);
    }

    // Active agent badge — purple
    if (ui.activeAgent) {
      rightParts.push(
        <ModeBadge
          key="agent"
          label={ui.activeAgent}
          bgColor={colors.background.badge.agent}
          textColor="#000000"
        />,
      );
      rightParts.push(<Text key="agent-sep"> </Text>);
    }

    // Git branch badge — blue background (like Ratatui)
    const branchText = ui.currentBranch || 'No-Git';
    rightParts.push(
      <ModeBadge
        key="branch"
        label={branchText}
        bgColor={colors.text.accent}
        textColor="#000000"
      />,
    );

    return (
      <Box justifyContent="space-between" width={terminal.width}>
        <Box flexShrink={1}>
          {leftParts}
        </Box>
        <Box flexShrink={0}>
          {rightParts}
        </Box>
      </Box>
    );
  };

  // -------------------------------------------------------------------------
  // Horizontal rule between prompt line and text area
  // -------------------------------------------------------------------------

  const renderHorizontalRule = (): React.ReactElement => {
    const ruleWidth = Math.max(0, terminal.width);
    return (
      <Text color={lineColor}>{'\u2500'.repeat(ruleWidth)}</Text>
    );
  };

  // -------------------------------------------------------------------------
  // Bottom status line
  // -------------------------------------------------------------------------

  const renderBottomLine = (): React.ReactElement => {
    const leftParts: React.ReactElement[] = [];

    // Context file tags
    if (ui.contextFiles.length > 0) {
      leftParts.push(
        <ContextFileTags
          key="ctx"
          files={ui.contextFiles}
          maxWidth={Math.floor(terminal.width * 0.6)}
        />,
      );
    }

    // Build cost display
    const modelEntries = Object.entries(ui.modelCosts);
    const totalCost = modelEntries.reduce((sum, [, v]) => sum + v.cost, 0);
    const totalTokens = ui.totalInputTokens + ui.totalOutputTokens;

    const rightParts: React.ReactElement[] = [];

    if (totalCost > 0) {
      // Show per-model costs: "Sonnet 4: $2.65, GPT-4o: $0.12 · Total: $2.77"
      const costParts = modelEntries
        .filter(([, v]) => v.cost > 0.001)
        .sort((a, b) => b[1].cost - a[1].cost)
        .map(([model, v]) => `${model}: $${v.cost.toFixed(2)}`)
        .join(', ');

      rightParts.push(
        <Text key="model-costs" color={colors.text.secondary} dimColor>
          {costParts}
        </Text>,
      );
      rightParts.push(
        <Text key="total" color={colors.status.done}>
          {' \u00B7 Total: $'}{totalCost.toFixed(2)}
        </Text>,
      );
    } else if (totalTokens > 0) {
      // No cost data yet but tokens used
      const formatted = totalTokens >= 1000
        ? `${(totalTokens / 1000).toFixed(1)}k tokens`
        : `${totalTokens} tokens`;
      rightParts.push(
        <Text key="tokens" color={colors.text.secondary} dimColor>
          {formatted}
        </Text>,
      );
    } else {
      // No usage yet — show API credits or balance
      const creditLabel = credits.loading
        ? 'checking\u2026'
        : credits.provider !== 'unknown' && credits.balance.startsWith('$')
          ? credits.balance
          : 'API credits';
      rightParts.push(
        <Text key="credits" color={colors.text.secondary} dimColor>
          {creditLabel}
        </Text>,
      );
    }

    if (ui.mcpServerCount > 0) {
      rightParts.push(
        <Text key="mcp" color={colors.status.done} dimColor>
          {' \u00B7 MCP:'}{ui.mcpServerCount}
        </Text>,
      );
    }

    return (
      <Box justifyContent="space-between" width={terminal.width}>
        <Box flexShrink={1}>{leftParts}</Box>
        <Box flexShrink={0}>{rightParts}</Box>
      </Box>
    );
  };

  // -------------------------------------------------------------------------
  // Main render -- Single-line bordered box that changes color with mode
  // -------------------------------------------------------------------------

  return (
    <Box flexDirection="column" width={terminal.width}>
      {/* -- Autocomplete dropdown (appears above prompt line) -- */}
      {acVisible && acItems.length > 0 && (
        <Autocomplete
          items={acItems}
          selectedIndex={acIndex}
          visible={acVisible}
          onSelect={handleAcSelect}
          onDismiss={() => setAcVisible(false)}
          maxVisible={8}
        />
      )}

      {/* -- Input box with badges embedded in the top border line -- */}
      {/* Top border: ╭─ icon hint ──────── [Mode] [Model] [Branch] ─╮ */}
      {(() => {
        // Build the left title: "icon Enter to send · Ctrl+P mode"
        const icon = isAgentBusy
          ? (ui.inputMode === 'streaming' ? '\u2819' : '\u25CF')
          : permConfig.icon;
        const hintText = isAgentBusy
          ? (ui.inputMode === 'streaming' ? 'Responding...' : `${ui.inputLabel || 'Running'}`)
          : 'Enter to send \u00B7 Ctrl+P mode';
        const leftTitle = `${icon} ${hintText}`;

        // Build badge strings for width calculation
        const modeBadge = ` ${permConfig.icon} ${permConfig.label} `;
        const modelBadge = (ui.currentModel && !ui.currentModel.startsWith('Run'))
          ? ` ${ui.currentModel} `
          : '';
        const branchBadge = ` ${ui.currentBranch || 'No-Git'} `;

        // Calculate fill dashes
        const badgesWidth = modeBadge.length + (modelBadge ? modelBadge.length + 1 : 0) + branchBadge.length + 2;
        const leftTitleWidth = leftTitle.length + 2; // +2 for ╭─ prefix
        const fillWidth = Math.max(1, terminal.width - leftTitleWidth - badgesWidth - 3);

        return (
          <Text>
            <Text color={lineColor}>{'\u256D\u2500'}</Text>
            <Text color={isAgentBusy ? colors.text.secondary : lineColor} bold={!isAgentBusy}>{` ${leftTitle} `}</Text>
            <Text color={lineColor}>{'\u2500'.repeat(fillWidth)}</Text>
            <Text backgroundColor={lineColor} color="#000000" bold>{modeBadge}</Text>
            {modelBadge ? <Text> </Text> : null}
            {modelBadge ? <Text backgroundColor={colors.background.badge.model} color={colors.text.primary} bold>{modelBadge}</Text> : null}
            <Text> </Text>
            <Text backgroundColor={colors.text.accent} color="#000000" bold>{branchBadge}</Text>
            <Text color={lineColor}>{'\u2500\u256E'}</Text>
          </Text>
        );
      })()}

      {/* Input area with side borders — minimum 4 lines tall */}
      {(() => {
        const inputLines = renderTextWithCursor();
        const minRows = 4;
        const padRows = Math.max(0, minRows - inputLines.length);
        const innerW = Math.max(0, terminal.width - 4); // 2 borders + 2 padding

        return (
          <Box flexDirection="column">
            {/* Content rows */}
            {inputLines.map((line, i) => (
              <Box key={`row-${i}`}>
                <Text color={lineColor}>{'\u2502'} </Text>
                <Box flexGrow={1}>{line}</Box>
                <Text color={lineColor}> {'\u2502'}</Text>
              </Box>
            ))}
            {/* Empty padding rows to fill minimum height */}
            {Array.from({ length: padRows }, (_, i) => (
              <Text key={`pad-${i}`}>
                <Text color={lineColor}>{'\u2502'}</Text>
                <Text>{' '.repeat(innerW + 2)}</Text>
                <Text color={lineColor}>{'\u2502'}</Text>
              </Text>
            ))}
          </Box>
        );
      })()}

      {/* Bottom border */}
      <Text color={lineColor}>{'\u2570'}{'\u2500'.repeat(Math.max(0, terminal.width - 2))}{'\u256F'}</Text>

      {/* -- Bottom line BELOW the box: context files left, credits right -- */}
      {renderBottomLine()}
    </Box>
  );
}
