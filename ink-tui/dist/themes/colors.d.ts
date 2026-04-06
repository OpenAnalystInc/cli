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
export declare const OA_BLUE = "#3282FF";
export declare const OA_LIGHT_BLUE = "#50A0FF";
export declare const OA_CYAN = "#50C8DC";
export declare const OA_ORANGE = "#FF8C00";
/** Primary body text — light gray on dark terminals */
export declare const TEXT_FOREGROUND = "#D0D0D0";
/** Secondary / dim text — metadata, timestamps */
export declare const TEXT_DIM = "#808080";
/** Headings — cyan, bold by convention */
export declare const TEXT_HEADING = "#50C8DC";
/** Emphasized / italic text */
export declare const TEXT_EMPHASIS = "#FF55FF";
/** Bold / strong text */
export declare const TEXT_STRONG = "#FFFF00";
/** Inline code spans */
export declare const TEXT_CODE = "#00FF00";
/** Hyperlinks — blue + underline by convention */
export declare const TEXT_LINK = "#5F87FF";
/** User prompt icon color */
export declare const TEXT_USER_PROMPT = "#50C8DC";
/** Slash-command text (e.g. /help, /build) */
export declare const TEXT_SLASH_COMMAND = "#FF8C00";
/** Normal borders — bright blue */
export declare const BORDER_DEFAULT = "#00AFFF";
/** Collapsed / disabled borders — dark gray */
export declare const BORDER_DIM = "#585858";
/** Focused panel borders — cyan */
export declare const BORDER_FOCUS = "#50C8DC";
export declare const INPUT_BORDER_DEFAULT = "#3282FF";
export declare const INPUT_BORDER_PLAN = "#FFFF00";
export declare const INPUT_BORDER_ACCEPT_EDITS = "#00FF00";
export declare const INPUT_BORDER_DANGER = "#FF0000";
export declare const INPUT_BORDER_STREAMING = "#50A0FF";
export declare const INPUT_BORDER_AGENT_RUNNING = "#50C8DC";
export declare const STATUS_RUNNING = "#3282FF";
export declare const STATUS_DONE = "#00FF00";
export declare const STATUS_ERROR = "#FF0000";
export declare const STATUS_WARNING = "#FFFF00";
export declare const STATUS_PENDING = "#808080";
/** Transparent — inherit terminal background */
export declare const BG_PRIMARY = "";
/** Focused message row in scroll mode */
export declare const BG_FOCUS = "#303030";
/** Input box background */
export declare const BG_INPUT = "";
export declare const BG_BADGE_MODE = "#3282FF";
export declare const BG_BADGE_AGENT = "#50C8DC";
export declare const BG_BADGE_BRANCH = "#585858";
export declare const BG_BADGE_MODEL = "#585858";
export declare const BG_BADGE_CONTEXT_FILE = "#00AFFF";
/** Interpolated via tinygradient at runtime for smooth cycling. */
export declare const SPINNER_GRADIENT: readonly string[];
export declare const DIFF_ADDED = "#005F00";
export declare const DIFF_REMOVED = "#5F0000";
export declare const TOOL_RUNNING = "#3282FF";
export declare const TOOL_COMPLETED = "#585858";
export declare const TOOL_FAILED = "#FF0000";
export declare const KB_BORDER = "#00AFFF";
export declare const KB_EXPANDED_BORDER = "#50C8DC";
export declare const KB_TAB_ACTIVE = "#50C8DC";
export declare const KB_TAB_INACTIVE = "#585858";
export declare const KB_SCORE = "#FFFF00";
export declare const KB_CITATION = "#5F87FF";
export declare const KB_CACHE = "#00FF00";
export declare const KB_GRAPH = "#FF8C00";
export declare const KB_ANSWER_DIVIDER = "#585858";
export declare const SIDEBAR_BORDER = "#585858";
export declare const SIDEBAR_SECTION_HEADER = "#50C8DC";
export declare const SIDEBAR_ITEM_DEFAULT = "#D0D0D0";
export declare const SIDEBAR_ITEM_SELECTED = "#3282FF";
export declare const SIDEBAR_FILE_READ = "#5F87FF";
export declare const SIDEBAR_FILE_EDITED = "#FFFF00";
export declare const SIDEBAR_FILE_CREATED = "#00FF00";
export declare const DIALOG_BORDER = "#FFFF00";
export declare const DIALOG_ALLOW_SELECTED = "#005F00";
export declare const DIALOG_ALLOW_UNSELECTED = "#585858";
export declare const DIALOG_DENY_SELECTED = "#5F0000";
export declare const DIALOG_DENY_UNSELECTED = "#585858";
export declare const SYNTAX_KEYWORD = "#3282FF";
export declare const SYNTAX_STRING = "#FFFF00";
export declare const SYNTAX_NUMBER = "#00FF00";
export declare const SYNTAX_COMMENT = "#808080";
export declare const SYNTAX_TYPE = "#50C8DC";
export declare const SYNTAX_FUNCTION = "#D0D0D0";
export declare const SYNTAX_VARIABLE = "#FF55FF";
export declare const SYNTAX_BUILT_IN = "#50C8DC";
export declare const SYNTAX_LINK = "#5F87FF";
export declare const SYNTAX_TAG = "#808080";
export declare const LIGHT: {
    readonly TEXT_FOREGROUND: "#1A1A1A";
    readonly TEXT_DIM: "#6B6B6B";
    readonly TEXT_HEADING: "#006080";
    readonly TEXT_EMPHASIS: "#8B008B";
    readonly TEXT_STRONG: "#8B6914";
    readonly TEXT_CODE: "#006400";
    readonly TEXT_LINK: "#005FAF";
    readonly TEXT_USER_PROMPT: "#006080";
    readonly TEXT_SLASH_COMMAND: "#CC7000";
    readonly BORDER_DEFAULT: "#005FAF";
    readonly BORDER_DIM: "#B0B0B0";
    readonly BORDER_FOCUS: "#006080";
    readonly BG_PRIMARY: "";
    readonly BG_FOCUS: "#E8E8E8";
    readonly BG_INPUT: "";
    readonly BG_BADGE_MODE: "#005FAF";
    readonly BG_BADGE_AGENT: "#006080";
    readonly BG_BADGE_BRANCH: "#B0B0B0";
    readonly BG_BADGE_MODEL: "#B0B0B0";
    readonly BG_BADGE_CONTEXT_FILE: "#005FAF";
    readonly STATUS_RUNNING: "#005FAF";
    readonly STATUS_DONE: "#006400";
    readonly STATUS_ERROR: "#CC0000";
    readonly STATUS_WARNING: "#8B6914";
    readonly STATUS_PENDING: "#6B6B6B";
    readonly DIFF_ADDED: "#D7FFD7";
    readonly DIFF_REMOVED: "#FFD7D7";
    readonly TOOL_RUNNING: "#005FAF";
    readonly TOOL_COMPLETED: "#B0B0B0";
    readonly TOOL_FAILED: "#CC0000";
    readonly DIALOG_BORDER: "#8B6914";
    readonly DIALOG_ALLOW_SELECTED: "#D7FFD7";
    readonly DIALOG_ALLOW_UNSELECTED: "#B0B0B0";
    readonly DIALOG_DENY_SELECTED: "#FFD7D7";
    readonly DIALOG_DENY_UNSELECTED: "#B0B0B0";
    readonly KB_BORDER: "#005FAF";
    readonly KB_EXPANDED_BORDER: "#006080";
    readonly KB_TAB_ACTIVE: "#006080";
    readonly KB_TAB_INACTIVE: "#B0B0B0";
    readonly KB_SCORE: "#8B6914";
    readonly KB_CITATION: "#005FAF";
    readonly KB_CACHE: "#006400";
    readonly KB_GRAPH: "#CC7000";
    readonly KB_ANSWER_DIVIDER: "#B0B0B0";
    readonly SIDEBAR_BORDER: "#B0B0B0";
    readonly SIDEBAR_SECTION_HEADER: "#006080";
    readonly SIDEBAR_ITEM_DEFAULT: "#1A1A1A";
    readonly SIDEBAR_ITEM_SELECTED: "#005FAF";
    readonly SIDEBAR_FILE_READ: "#005FAF";
    readonly SIDEBAR_FILE_EDITED: "#8B6914";
    readonly SIDEBAR_FILE_CREATED: "#006400";
    readonly INPUT_BORDER_DEFAULT: "#005FAF";
    readonly INPUT_BORDER_PLAN: "#8B6914";
    readonly INPUT_BORDER_ACCEPT_EDITS: "#006400";
    readonly INPUT_BORDER_DANGER: "#CC0000";
    readonly INPUT_BORDER_STREAMING: "#005FAF";
    readonly INPUT_BORDER_AGENT_RUNNING: "#006080";
    readonly SYNTAX_KEYWORD: "#005FAF";
    readonly SYNTAX_STRING: "#8B6914";
    readonly SYNTAX_NUMBER: "#006400";
    readonly SYNTAX_COMMENT: "#6B6B6B";
    readonly SYNTAX_TYPE: "#006080";
    readonly SYNTAX_FUNCTION: "#1A1A1A";
    readonly SYNTAX_VARIABLE: "#8B008B";
    readonly SYNTAX_BUILT_IN: "#006080";
    readonly SYNTAX_LINK: "#005FAF";
    readonly SYNTAX_TAG: "#6B6B6B";
    readonly SPINNER_GRADIENT: readonly string[];
};
