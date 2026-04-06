/**
 * OpenAnalyst CLI — Theme definitions.
 *
 * Two built-in themes: OADarkTheme (primary) and OALightTheme.
 * Each fully implements SemanticColors so every component has a
 * consistent, type-safe color contract.
 */

import type { SemanticColors, ThemeType } from './semantic-tokens.js';
import * as C from './colors.js';

// ---------------------------------------------------------------------------
// Theme class
// ---------------------------------------------------------------------------

export class OATheme {
  constructor(
    readonly name: string,
    readonly type: ThemeType,
    readonly colors: SemanticColors,
  ) {}
}

// ---------------------------------------------------------------------------
// Dark theme — matches the Rust TUI default
// ---------------------------------------------------------------------------

export const OADarkTheme = new OATheme('oa-dark', 'dark', {
  text: {
    primary: C.TEXT_FOREGROUND,
    secondary: C.TEXT_DIM,
    accent: C.OA_CYAN,
    heading: C.TEXT_HEADING,
    emphasis: C.TEXT_EMPHASIS,
    strong: C.TEXT_STRONG,
    code: C.TEXT_CODE,
    link: C.TEXT_LINK,
    userPrompt: C.TEXT_USER_PROMPT,
    slashCommand: C.TEXT_SLASH_COMMAND,
  },
  background: {
    primary: C.BG_PRIMARY,
    focus: C.BG_FOCUS,
    input: C.BG_INPUT,
    badge: {
      mode: C.BG_BADGE_MODE,
      agent: C.BG_BADGE_AGENT,
      branch: C.BG_BADGE_BRANCH,
      model: C.BG_BADGE_MODEL,
      contextFile: C.BG_BADGE_CONTEXT_FILE,
    },
    diff: {
      added: C.DIFF_ADDED,
      removed: C.DIFF_REMOVED,
    },
  },
  border: {
    default: C.BORDER_DEFAULT,
    dim: C.BORDER_DIM,
    focus: C.BORDER_FOCUS,
    input: {
      default: C.INPUT_BORDER_DEFAULT,
      plan: C.INPUT_BORDER_PLAN,
      acceptEdits: C.INPUT_BORDER_ACCEPT_EDITS,
      danger: C.INPUT_BORDER_DANGER,
      streaming: C.INPUT_BORDER_STREAMING,
      agentRunning: C.INPUT_BORDER_AGENT_RUNNING,
    },
  },
  status: {
    running: C.STATUS_RUNNING,
    done: C.STATUS_DONE,
    error: C.STATUS_ERROR,
    warning: C.STATUS_WARNING,
    pending: C.STATUS_PENDING,
  },
  spinner: {
    active: C.OA_BLUE,
    gradient: C.SPINNER_GRADIENT,
  },
  toolCard: {
    running: C.TOOL_RUNNING,
    completed: C.TOOL_COMPLETED,
    failed: C.TOOL_FAILED,
  },
  knowledgeCard: {
    border: C.KB_BORDER,
    expandedBorder: C.KB_EXPANDED_BORDER,
    tabActive: C.KB_TAB_ACTIVE,
    tabInactive: C.KB_TAB_INACTIVE,
    score: C.KB_SCORE,
    citation: C.KB_CITATION,
    cache: C.KB_CACHE,
    graph: C.KB_GRAPH,
    answerDivider: C.KB_ANSWER_DIVIDER,
  },
  sidebar: {
    border: C.SIDEBAR_BORDER,
    sectionHeader: C.SIDEBAR_SECTION_HEADER,
    itemDefault: C.SIDEBAR_ITEM_DEFAULT,
    itemSelected: C.SIDEBAR_ITEM_SELECTED,
    fileRead: C.SIDEBAR_FILE_READ,
    fileEdited: C.SIDEBAR_FILE_EDITED,
    fileCreated: C.SIDEBAR_FILE_CREATED,
  },
  diff: {
    added: C.DIFF_ADDED,
    removed: C.DIFF_REMOVED,
  },
  dialog: {
    border: C.DIALOG_BORDER,
    allowSelected: C.DIALOG_ALLOW_SELECTED,
    allowUnselected: C.DIALOG_ALLOW_UNSELECTED,
    denySelected: C.DIALOG_DENY_SELECTED,
    denyUnselected: C.DIALOG_DENY_UNSELECTED,
  },
  syntax: {
    keyword: C.SYNTAX_KEYWORD,
    string: C.SYNTAX_STRING,
    number: C.SYNTAX_NUMBER,
    comment: C.SYNTAX_COMMENT,
    type: C.SYNTAX_TYPE,
    function: C.SYNTAX_FUNCTION,
    variable: C.SYNTAX_VARIABLE,
    builtIn: C.SYNTAX_BUILT_IN,
    link: C.SYNTAX_LINK,
    tag: C.SYNTAX_TAG,
  },
});

// ---------------------------------------------------------------------------
// Light theme — for light terminal backgrounds
// ---------------------------------------------------------------------------

const L = C.LIGHT;

export const OALightTheme = new OATheme('oa-light', 'light', {
  text: {
    primary: L.TEXT_FOREGROUND,
    secondary: L.TEXT_DIM,
    accent: L.TEXT_HEADING,
    heading: L.TEXT_HEADING,
    emphasis: L.TEXT_EMPHASIS,
    strong: L.TEXT_STRONG,
    code: L.TEXT_CODE,
    link: L.TEXT_LINK,
    userPrompt: L.TEXT_USER_PROMPT,
    slashCommand: L.TEXT_SLASH_COMMAND,
  },
  background: {
    primary: L.BG_PRIMARY,
    focus: L.BG_FOCUS,
    input: L.BG_INPUT,
    badge: {
      mode: L.BG_BADGE_MODE,
      agent: L.BG_BADGE_AGENT,
      branch: L.BG_BADGE_BRANCH,
      model: L.BG_BADGE_MODEL,
      contextFile: L.BG_BADGE_CONTEXT_FILE,
    },
    diff: {
      added: L.DIFF_ADDED,
      removed: L.DIFF_REMOVED,
    },
  },
  border: {
    default: L.BORDER_DEFAULT,
    dim: L.BORDER_DIM,
    focus: L.BORDER_FOCUS,
    input: {
      default: L.INPUT_BORDER_DEFAULT,
      plan: L.INPUT_BORDER_PLAN,
      acceptEdits: L.INPUT_BORDER_ACCEPT_EDITS,
      danger: L.INPUT_BORDER_DANGER,
      streaming: L.INPUT_BORDER_STREAMING,
      agentRunning: L.INPUT_BORDER_AGENT_RUNNING,
    },
  },
  status: {
    running: L.STATUS_RUNNING,
    done: L.STATUS_DONE,
    error: L.STATUS_ERROR,
    warning: L.STATUS_WARNING,
    pending: L.STATUS_PENDING,
  },
  spinner: {
    active: L.STATUS_RUNNING,
    gradient: L.SPINNER_GRADIENT,
  },
  toolCard: {
    running: L.TOOL_RUNNING,
    completed: L.TOOL_COMPLETED,
    failed: L.TOOL_FAILED,
  },
  knowledgeCard: {
    border: L.KB_BORDER,
    expandedBorder: L.KB_EXPANDED_BORDER,
    tabActive: L.KB_TAB_ACTIVE,
    tabInactive: L.KB_TAB_INACTIVE,
    score: L.KB_SCORE,
    citation: L.KB_CITATION,
    cache: L.KB_CACHE,
    graph: L.KB_GRAPH,
    answerDivider: L.KB_ANSWER_DIVIDER,
  },
  sidebar: {
    border: L.SIDEBAR_BORDER,
    sectionHeader: L.SIDEBAR_SECTION_HEADER,
    itemDefault: L.SIDEBAR_ITEM_DEFAULT,
    itemSelected: L.SIDEBAR_ITEM_SELECTED,
    fileRead: L.SIDEBAR_FILE_READ,
    fileEdited: L.SIDEBAR_FILE_EDITED,
    fileCreated: L.SIDEBAR_FILE_CREATED,
  },
  diff: {
    added: L.DIFF_ADDED,
    removed: L.DIFF_REMOVED,
  },
  dialog: {
    border: L.DIALOG_BORDER,
    allowSelected: L.DIALOG_ALLOW_SELECTED,
    allowUnselected: L.DIALOG_ALLOW_UNSELECTED,
    denySelected: L.DIALOG_DENY_SELECTED,
    denyUnselected: L.DIALOG_DENY_UNSELECTED,
  },
  syntax: {
    keyword: L.SYNTAX_KEYWORD,
    string: L.SYNTAX_STRING,
    number: L.SYNTAX_NUMBER,
    comment: L.SYNTAX_COMMENT,
    type: L.SYNTAX_TYPE,
    function: L.SYNTAX_FUNCTION,
    variable: L.SYNTAX_VARIABLE,
    builtIn: L.SYNTAX_BUILT_IN,
    link: L.SYNTAX_LINK,
    tag: L.SYNTAX_TAG,
  },
});
