/**
 * OpenAnalyst CLI — Raw color definitions.
 *
 * Every hex value in this file is the single source of truth for the OA palette.
 * Components never import this directly — they go through semantic tokens.
 *
 * ANSI indexed-color equivalents (for documentation):
 *   Indexed(39)  = #00AFFF   Indexed(45)  = #00D7FF
 *   Indexed(240) = #585858   Indexed(236) = #303030
 *   Indexed(252) = #D0D0D0
 */
// ---------------------------------------------------------------------------
// Brand
// ---------------------------------------------------------------------------
export const OA_BLUE = '#3282FF';
export const OA_LIGHT_BLUE = '#50A0FF';
export const OA_CYAN = '#50C8DC';
export const OA_ORANGE = '#FF8C00';
// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------
/** Primary body text — light gray on dark terminals */
export const TEXT_FOREGROUND = '#D0D0D0';
/** Secondary / dim text — metadata, timestamps */
export const TEXT_DIM = '#808080';
/** Headings — cyan, bold by convention */
export const TEXT_HEADING = '#50C8DC';
/** Emphasized / italic text */
export const TEXT_EMPHASIS = '#FF55FF'; // magenta
/** Bold / strong text */
export const TEXT_STRONG = '#FFFF00'; // yellow
/** Inline code spans */
export const TEXT_CODE = '#00FF00'; // green
/** Hyperlinks — blue + underline by convention */
export const TEXT_LINK = '#5F87FF';
/** User prompt icon color */
export const TEXT_USER_PROMPT = '#50C8DC';
/** Slash-command text (e.g. /help, /build) */
export const TEXT_SLASH_COMMAND = '#FF8C00';
// ---------------------------------------------------------------------------
// Borders
// ---------------------------------------------------------------------------
/** Normal borders — bright blue */
export const BORDER_DEFAULT = '#00AFFF';
/** Collapsed / disabled borders — dark gray */
export const BORDER_DIM = '#585858';
/** Focused panel borders — cyan */
export const BORDER_FOCUS = '#50C8DC';
// ---------------------------------------------------------------------------
// Input-box border colors (by permission mode)
// ---------------------------------------------------------------------------
export const INPUT_BORDER_DEFAULT = '#3282FF';
export const INPUT_BORDER_PLAN = '#FFFF00';
export const INPUT_BORDER_ACCEPT_EDITS = '#00FF00';
export const INPUT_BORDER_DANGER = '#FF0000';
export const INPUT_BORDER_STREAMING = '#50A0FF';
export const INPUT_BORDER_AGENT_RUNNING = '#50C8DC';
// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------
export const STATUS_RUNNING = '#3282FF';
export const STATUS_DONE = '#00FF00';
export const STATUS_ERROR = '#FF0000';
export const STATUS_WARNING = '#FFFF00';
export const STATUS_PENDING = '#808080';
// ---------------------------------------------------------------------------
// Backgrounds
// ---------------------------------------------------------------------------
/** Transparent — inherit terminal background */
export const BG_PRIMARY = '';
/** Focused message row in scroll mode */
export const BG_FOCUS = '#303030';
/** Input box background */
export const BG_INPUT = '';
// Badge backgrounds
export const BG_BADGE_MODE = '#3282FF';
export const BG_BADGE_AGENT = '#50C8DC';
export const BG_BADGE_BRANCH = '#585858';
export const BG_BADGE_MODEL = '#585858';
export const BG_BADGE_CONTEXT_FILE = '#00AFFF';
// ---------------------------------------------------------------------------
// Spinner gradient — 8-keyframe brand-color cycle
// ---------------------------------------------------------------------------
/** Interpolated via tinygradient at runtime for smooth cycling. */
export const SPINNER_GRADIENT = [
    '#3282FF', // OA Blue
    '#50A0FF', // Light Blue
    '#50C8DC', // Cyan
    '#3DC8AA', // Teal
    '#00FF00', // Green
    '#CCFF00', // Lime-Yellow
    '#FFFF00', // Yellow
    '#50A0FF', // back toward Blue
];
// ---------------------------------------------------------------------------
// Diff
// ---------------------------------------------------------------------------
export const DIFF_ADDED = '#005F00';
export const DIFF_REMOVED = '#5F0000';
// ---------------------------------------------------------------------------
// Tool cards
// ---------------------------------------------------------------------------
export const TOOL_RUNNING = '#3282FF';
export const TOOL_COMPLETED = '#585858';
export const TOOL_FAILED = '#FF0000';
// ---------------------------------------------------------------------------
// Knowledge cards
// ---------------------------------------------------------------------------
export const KB_BORDER = '#00AFFF';
export const KB_EXPANDED_BORDER = '#50C8DC';
export const KB_TAB_ACTIVE = '#50C8DC';
export const KB_TAB_INACTIVE = '#585858';
export const KB_SCORE = '#FFFF00';
export const KB_CITATION = '#5F87FF';
export const KB_CACHE = '#00FF00';
export const KB_GRAPH = '#FF8C00';
export const KB_ANSWER_DIVIDER = '#585858';
// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------
export const SIDEBAR_BORDER = '#585858';
export const SIDEBAR_SECTION_HEADER = '#50C8DC';
export const SIDEBAR_ITEM_DEFAULT = '#D0D0D0';
export const SIDEBAR_ITEM_SELECTED = '#3282FF';
export const SIDEBAR_FILE_READ = '#5F87FF';
export const SIDEBAR_FILE_EDITED = '#FFFF00';
export const SIDEBAR_FILE_CREATED = '#00FF00';
// ---------------------------------------------------------------------------
// Dialog
// ---------------------------------------------------------------------------
export const DIALOG_BORDER = '#FFFF00';
export const DIALOG_ALLOW_SELECTED = '#005F00';
export const DIALOG_ALLOW_UNSELECTED = '#585858';
export const DIALOG_DENY_SELECTED = '#5F0000';
export const DIALOG_DENY_UNSELECTED = '#585858';
// ---------------------------------------------------------------------------
// Code syntax highlighting (hljs class → color)
// ---------------------------------------------------------------------------
export const SYNTAX_KEYWORD = '#3282FF';
export const SYNTAX_STRING = '#FFFF00';
export const SYNTAX_NUMBER = '#00FF00';
export const SYNTAX_COMMENT = '#808080';
export const SYNTAX_TYPE = '#50C8DC';
export const SYNTAX_FUNCTION = '#D0D0D0';
export const SYNTAX_VARIABLE = '#FF55FF';
export const SYNTAX_BUILT_IN = '#50C8DC';
export const SYNTAX_LINK = '#5F87FF';
export const SYNTAX_TAG = '#808080';
// ---------------------------------------------------------------------------
// Light-terminal overrides
// ---------------------------------------------------------------------------
export const LIGHT = {
    TEXT_FOREGROUND: '#1A1A1A',
    TEXT_DIM: '#6B6B6B',
    TEXT_HEADING: '#006080',
    TEXT_EMPHASIS: '#8B008B',
    TEXT_STRONG: '#8B6914',
    TEXT_CODE: '#006400',
    TEXT_LINK: '#005FAF',
    TEXT_USER_PROMPT: '#006080',
    TEXT_SLASH_COMMAND: '#CC7000',
    BORDER_DEFAULT: '#005FAF',
    BORDER_DIM: '#B0B0B0',
    BORDER_FOCUS: '#006080',
    BG_PRIMARY: '',
    BG_FOCUS: '#E8E8E8',
    BG_INPUT: '',
    BG_BADGE_MODE: '#005FAF',
    BG_BADGE_AGENT: '#006080',
    BG_BADGE_BRANCH: '#B0B0B0',
    BG_BADGE_MODEL: '#B0B0B0',
    BG_BADGE_CONTEXT_FILE: '#005FAF',
    STATUS_RUNNING: '#005FAF',
    STATUS_DONE: '#006400',
    STATUS_ERROR: '#CC0000',
    STATUS_WARNING: '#8B6914',
    STATUS_PENDING: '#6B6B6B',
    DIFF_ADDED: '#D7FFD7',
    DIFF_REMOVED: '#FFD7D7',
    TOOL_RUNNING: '#005FAF',
    TOOL_COMPLETED: '#B0B0B0',
    TOOL_FAILED: '#CC0000',
    DIALOG_BORDER: '#8B6914',
    DIALOG_ALLOW_SELECTED: '#D7FFD7',
    DIALOG_ALLOW_UNSELECTED: '#B0B0B0',
    DIALOG_DENY_SELECTED: '#FFD7D7',
    DIALOG_DENY_UNSELECTED: '#B0B0B0',
    KB_BORDER: '#005FAF',
    KB_EXPANDED_BORDER: '#006080',
    KB_TAB_ACTIVE: '#006080',
    KB_TAB_INACTIVE: '#B0B0B0',
    KB_SCORE: '#8B6914',
    KB_CITATION: '#005FAF',
    KB_CACHE: '#006400',
    KB_GRAPH: '#CC7000',
    KB_ANSWER_DIVIDER: '#B0B0B0',
    SIDEBAR_BORDER: '#B0B0B0',
    SIDEBAR_SECTION_HEADER: '#006080',
    SIDEBAR_ITEM_DEFAULT: '#1A1A1A',
    SIDEBAR_ITEM_SELECTED: '#005FAF',
    SIDEBAR_FILE_READ: '#005FAF',
    SIDEBAR_FILE_EDITED: '#8B6914',
    SIDEBAR_FILE_CREATED: '#006400',
    INPUT_BORDER_DEFAULT: '#005FAF',
    INPUT_BORDER_PLAN: '#8B6914',
    INPUT_BORDER_ACCEPT_EDITS: '#006400',
    INPUT_BORDER_DANGER: '#CC0000',
    INPUT_BORDER_STREAMING: '#005FAF',
    INPUT_BORDER_AGENT_RUNNING: '#006080',
    SYNTAX_KEYWORD: '#005FAF',
    SYNTAX_STRING: '#8B6914',
    SYNTAX_NUMBER: '#006400',
    SYNTAX_COMMENT: '#6B6B6B',
    SYNTAX_TYPE: '#006080',
    SYNTAX_FUNCTION: '#1A1A1A',
    SYNTAX_VARIABLE: '#8B008B',
    SYNTAX_BUILT_IN: '#006080',
    SYNTAX_LINK: '#005FAF',
    SYNTAX_TAG: '#6B6B6B',
    SPINNER_GRADIENT: ['#005FAF', '#0070C0', '#006080', '#006464', '#006400', '#7A8C00', '#8B6914', '#0070C0'],
};
//# sourceMappingURL=colors.js.map