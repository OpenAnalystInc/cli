/**
 * Command enum and metadata for all OpenAnalyst TUI keybindings.
 *
 * Source of truth: rust/crates/tui/src/keybindings.rs
 * Every command maps 1:1 to an action the Rust TUI can perform.
 */
export declare enum Command {
    QUIT = "global.quit",
    CANCEL_AGENT = "global.cancelAgent",
    RUN_IN_BACKGROUND = "global.runInBackground",
    CYCLE_PERMISSION_MODE = "global.cyclePermissionMode",
    TOGGLE_SIDEBAR = "global.toggleSidebar",
    FOCUS_SIDEBAR = "global.focusSidebar",
    CLEAR_CHAT = "global.clearChat",
    SCROLL_TO_TOP = "global.scrollToTop",
    SCROLL_TO_BOTTOM = "global.scrollToBottom",
    SCROLL_UP_PAGE = "global.scrollUpPage",
    SCROLL_DOWN_PAGE = "global.scrollDownPage",
    SUBMIT = "input.submit",
    ENTER_SCROLL_MODE = "input.enterScrollMode",
    UNDO_LAST_ACTION = "input.undoLastAction",
    START_VOICE_RECORDING = "input.startVoiceRecording",
    HISTORY_UP = "input.historyUp",
    HISTORY_DOWN = "input.historyDown",
    REMOVE_LAST_CONTEXT_FILE = "input.removeLastContextFile",
    SCROLL_UP = "scroll.up",
    SCROLL_DOWN = "scroll.down",
    JUMP_TO_TOP = "scroll.jumpToTop",
    JUMP_TO_BOTTOM = "scroll.jumpToBottom",
    TOGGLE_EXPAND = "scroll.toggleExpand",
    NEXT_TAB = "scroll.nextTab",
    PREV_TAB = "scroll.prevTab",
    FEEDBACK_POSITIVE = "scroll.feedbackPositive",
    FEEDBACK_NEGATIVE = "scroll.feedbackNegative",
    EXIT_SCROLL_MODE = "scroll.exitScrollMode",
    START_SEARCH = "scroll.startSearch",
    SIDEBAR_NEXT_ITEM = "sidebar.nextItem",
    SIDEBAR_PREV_ITEM = "sidebar.prevItem",
    SIDEBAR_NEXT_SECTION = "sidebar.nextSection",
    SIDEBAR_PREV_SECTION = "sidebar.prevSection",
    SIDEBAR_ACTION = "sidebar.action",
    SIDEBAR_EXIT = "sidebar.exit",
    DIALOG_SWITCH_BUTTON = "dialog.switchButton",
    DIALOG_CONFIRM = "dialog.confirm",
    DIALOG_ALLOW = "dialog.allow",
    DIALOG_DENY = "dialog.deny",
    ASK_NEXT_OPTION = "ask.nextOption",
    ASK_PREV_OPTION = "ask.prevOption",
    ASK_SELECT = "ask.select",
    ASK_QUICK_SELECT = "ask.quickSelect",
    ASK_SWITCH_TO_TYPE = "ask.switchToType",
    ASK_CHAT_ABOUT_IT = "ask.chatAboutIt",
    AC_NEXT = "ac.next",
    AC_PREV = "ac.prev",
    AC_ACCEPT = "ac.accept",
    AC_ACCEPT_SUBMIT = "ac.acceptSubmit",
    AC_DISMISS = "ac.dismiss",
    VOICE_STOP = "voice.stop"
}
export declare const commandDescriptions: Readonly<Record<Command, string>>;
export interface CommandCategory {
    readonly title: string;
    readonly commands: readonly Command[];
}
export declare const commandCategories: readonly CommandCategory[];
