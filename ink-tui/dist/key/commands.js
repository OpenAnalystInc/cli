/**
 * Command enum and metadata for all OpenAnalyst TUI keybindings.
 *
 * Source of truth: rust/crates/tui/src/keybindings.rs
 * Every command maps 1:1 to an action the Rust TUI can perform.
 */
// ---------------------------------------------------------------------------
// Command enum
// ---------------------------------------------------------------------------
export var Command;
(function (Command) {
    // --- Global (fire regardless of mode, unless intercepted by higher priority) ---
    Command["QUIT"] = "global.quit";
    Command["CANCEL_AGENT"] = "global.cancelAgent";
    Command["RUN_IN_BACKGROUND"] = "global.runInBackground";
    Command["CYCLE_PERMISSION_MODE"] = "global.cyclePermissionMode";
    Command["TOGGLE_SIDEBAR"] = "global.toggleSidebar";
    Command["FOCUS_SIDEBAR"] = "global.focusSidebar";
    Command["CLEAR_CHAT"] = "global.clearChat";
    Command["SCROLL_TO_TOP"] = "global.scrollToTop";
    Command["SCROLL_TO_BOTTOM"] = "global.scrollToBottom";
    Command["SCROLL_UP_PAGE"] = "global.scrollUpPage";
    Command["SCROLL_DOWN_PAGE"] = "global.scrollDownPage";
    // --- Input mode ---
    Command["SUBMIT"] = "input.submit";
    Command["ENTER_SCROLL_MODE"] = "input.enterScrollMode";
    Command["UNDO_LAST_ACTION"] = "input.undoLastAction";
    Command["START_VOICE_RECORDING"] = "input.startVoiceRecording";
    Command["HISTORY_UP"] = "input.historyUp";
    Command["HISTORY_DOWN"] = "input.historyDown";
    Command["REMOVE_LAST_CONTEXT_FILE"] = "input.removeLastContextFile";
    // --- Scroll mode (vim-like browse) ---
    Command["SCROLL_UP"] = "scroll.up";
    Command["SCROLL_DOWN"] = "scroll.down";
    Command["JUMP_TO_TOP"] = "scroll.jumpToTop";
    Command["JUMP_TO_BOTTOM"] = "scroll.jumpToBottom";
    Command["TOGGLE_EXPAND"] = "scroll.toggleExpand";
    Command["NEXT_TAB"] = "scroll.nextTab";
    Command["PREV_TAB"] = "scroll.prevTab";
    Command["FEEDBACK_POSITIVE"] = "scroll.feedbackPositive";
    Command["FEEDBACK_NEGATIVE"] = "scroll.feedbackNegative";
    Command["EXIT_SCROLL_MODE"] = "scroll.exitScrollMode";
    Command["START_SEARCH"] = "scroll.startSearch";
    // --- Sidebar ---
    Command["SIDEBAR_NEXT_ITEM"] = "sidebar.nextItem";
    Command["SIDEBAR_PREV_ITEM"] = "sidebar.prevItem";
    Command["SIDEBAR_NEXT_SECTION"] = "sidebar.nextSection";
    Command["SIDEBAR_PREV_SECTION"] = "sidebar.prevSection";
    Command["SIDEBAR_ACTION"] = "sidebar.action";
    Command["SIDEBAR_EXIT"] = "sidebar.exit";
    // --- Permission dialog ---
    Command["DIALOG_SWITCH_BUTTON"] = "dialog.switchButton";
    Command["DIALOG_CONFIRM"] = "dialog.confirm";
    Command["DIALOG_ALLOW"] = "dialog.allow";
    Command["DIALOG_DENY"] = "dialog.deny";
    // --- Ask-user dialog ---
    Command["ASK_NEXT_OPTION"] = "ask.nextOption";
    Command["ASK_PREV_OPTION"] = "ask.prevOption";
    Command["ASK_SELECT"] = "ask.select";
    Command["ASK_QUICK_SELECT"] = "ask.quickSelect";
    Command["ASK_SWITCH_TO_TYPE"] = "ask.switchToType";
    Command["ASK_CHAT_ABOUT_IT"] = "ask.chatAboutIt";
    // --- Autocomplete popup ---
    Command["AC_NEXT"] = "ac.next";
    Command["AC_PREV"] = "ac.prev";
    Command["AC_ACCEPT"] = "ac.accept";
    Command["AC_ACCEPT_SUBMIT"] = "ac.acceptSubmit";
    Command["AC_DISMISS"] = "ac.dismiss";
    // --- Voice recording ---
    Command["VOICE_STOP"] = "voice.stop";
})(Command || (Command = {}));
// ---------------------------------------------------------------------------
// Human-readable descriptions (used in help panel and status bar hints)
// ---------------------------------------------------------------------------
export const commandDescriptions = {
    // Global
    [Command.QUIT]: 'Cancel running agent or quit (double-press to exit)',
    [Command.CANCEL_AGENT]: 'Cancel the currently running agent (double-Esc while streaming)',
    [Command.RUN_IN_BACKGROUND]: 'Submit current input to run in background',
    [Command.CYCLE_PERMISSION_MODE]: 'Cycle permission mode: Default > Plan > Accept Edits > Danger',
    [Command.TOGGLE_SIDEBAR]: 'Toggle sidebar visibility',
    [Command.FOCUS_SIDEBAR]: 'Toggle sidebar focus (show/focus/hide cycle)',
    [Command.CLEAR_CHAT]: 'Clear all chat messages',
    [Command.SCROLL_TO_TOP]: 'Scroll chat to the very top',
    [Command.SCROLL_TO_BOTTOM]: 'Scroll chat to the very bottom',
    [Command.SCROLL_UP_PAGE]: 'Scroll chat up by one page',
    [Command.SCROLL_DOWN_PAGE]: 'Scroll chat down by one page',
    // Input
    [Command.SUBMIT]: 'Submit the current prompt',
    [Command.ENTER_SCROLL_MODE]: 'Enter scroll/browse mode (Esc)',
    [Command.UNDO_LAST_ACTION]: 'Undo the last action (double-Esc)',
    [Command.START_VOICE_RECORDING]: 'Start voice recording (Space when input is empty)',
    [Command.HISTORY_UP]: 'Previous entry in prompt history',
    [Command.HISTORY_DOWN]: 'Next entry in prompt history',
    [Command.REMOVE_LAST_CONTEXT_FILE]: 'Remove last context file (Backspace on empty input)',
    // Scroll mode
    [Command.SCROLL_UP]: 'Scroll up one line',
    [Command.SCROLL_DOWN]: 'Scroll down one line',
    [Command.JUMP_TO_TOP]: 'Jump to first message',
    [Command.JUMP_TO_BOTTOM]: 'Jump to last message',
    [Command.TOGGLE_EXPAND]: 'Expand or collapse focused card',
    [Command.NEXT_TAB]: 'Next tab in knowledge card',
    [Command.PREV_TAB]: 'Previous tab in knowledge card',
    [Command.FEEDBACK_POSITIVE]: 'Rate knowledge result as helpful',
    [Command.FEEDBACK_NEGATIVE]: 'Rate knowledge result as unhelpful',
    [Command.EXIT_SCROLL_MODE]: 'Exit scroll mode and return to input',
    [Command.START_SEARCH]: 'Start search (exits scroll mode, types /)',
    // Sidebar
    [Command.SIDEBAR_NEXT_ITEM]: 'Select next item in sidebar section',
    [Command.SIDEBAR_PREV_ITEM]: 'Select previous item in sidebar section',
    [Command.SIDEBAR_NEXT_SECTION]: 'Cycle to next sidebar section',
    [Command.SIDEBAR_PREV_SECTION]: 'Cycle to previous sidebar section',
    [Command.SIDEBAR_ACTION]: 'Perform action on selected sidebar item',
    [Command.SIDEBAR_EXIT]: 'Return focus from sidebar to input',
    // Permission dialog
    [Command.DIALOG_SWITCH_BUTTON]: 'Switch between Allow and Deny buttons',
    [Command.DIALOG_CONFIRM]: 'Confirm the selected dialog button',
    [Command.DIALOG_ALLOW]: 'Quick-allow the permission request',
    [Command.DIALOG_DENY]: 'Quick-deny the permission request',
    // Ask-user dialog
    [Command.ASK_NEXT_OPTION]: 'Move to next option in ask-user dialog',
    [Command.ASK_PREV_OPTION]: 'Move to previous option in ask-user dialog',
    [Command.ASK_SELECT]: 'Select current option or submit typed answer',
    [Command.ASK_QUICK_SELECT]: 'Quick-select option by number (1-9)',
    [Command.ASK_SWITCH_TO_TYPE]: 'Switch to free-text typing mode',
    [Command.ASK_CHAT_ABOUT_IT]: 'Dismiss dialog and discuss in chat',
    // Autocomplete
    [Command.AC_NEXT]: 'Next autocomplete suggestion',
    [Command.AC_PREV]: 'Previous autocomplete suggestion',
    [Command.AC_ACCEPT]: 'Accept suggestion into input',
    [Command.AC_ACCEPT_SUBMIT]: 'Accept suggestion and submit immediately',
    [Command.AC_DISMISS]: 'Dismiss autocomplete popup',
    // Voice
    [Command.VOICE_STOP]: 'Stop voice recording and transcribe',
};
export const commandCategories = [
    {
        title: 'Global',
        commands: [
            Command.QUIT,
            Command.RUN_IN_BACKGROUND,
            Command.CYCLE_PERMISSION_MODE,
            Command.TOGGLE_SIDEBAR,
            Command.FOCUS_SIDEBAR,
            Command.CLEAR_CHAT,
            Command.SCROLL_TO_TOP,
            Command.SCROLL_TO_BOTTOM,
            Command.SCROLL_UP_PAGE,
            Command.SCROLL_DOWN_PAGE,
        ],
    },
    {
        title: 'Input',
        commands: [
            Command.SUBMIT,
            Command.ENTER_SCROLL_MODE,
            Command.UNDO_LAST_ACTION,
            Command.START_VOICE_RECORDING,
            Command.HISTORY_UP,
            Command.HISTORY_DOWN,
            Command.REMOVE_LAST_CONTEXT_FILE,
        ],
    },
    {
        title: 'Scroll Mode',
        commands: [
            Command.SCROLL_UP,
            Command.SCROLL_DOWN,
            Command.JUMP_TO_TOP,
            Command.JUMP_TO_BOTTOM,
            Command.TOGGLE_EXPAND,
            Command.NEXT_TAB,
            Command.PREV_TAB,
            Command.FEEDBACK_POSITIVE,
            Command.FEEDBACK_NEGATIVE,
            Command.EXIT_SCROLL_MODE,
            Command.START_SEARCH,
        ],
    },
    {
        title: 'Sidebar',
        commands: [
            Command.SIDEBAR_NEXT_ITEM,
            Command.SIDEBAR_PREV_ITEM,
            Command.SIDEBAR_NEXT_SECTION,
            Command.SIDEBAR_PREV_SECTION,
            Command.SIDEBAR_ACTION,
            Command.SIDEBAR_EXIT,
        ],
    },
    {
        title: 'Permission Dialog',
        commands: [
            Command.DIALOG_SWITCH_BUTTON,
            Command.DIALOG_CONFIRM,
            Command.DIALOG_ALLOW,
            Command.DIALOG_DENY,
        ],
    },
    {
        title: 'Ask User Dialog',
        commands: [
            Command.ASK_NEXT_OPTION,
            Command.ASK_PREV_OPTION,
            Command.ASK_SELECT,
            Command.ASK_QUICK_SELECT,
            Command.ASK_SWITCH_TO_TYPE,
            Command.ASK_CHAT_ABOUT_IT,
        ],
    },
    {
        title: 'Autocomplete',
        commands: [
            Command.AC_NEXT,
            Command.AC_PREV,
            Command.AC_ACCEPT,
            Command.AC_ACCEPT_SUBMIT,
            Command.AC_DISMISS,
        ],
    },
    {
        title: 'Voice',
        commands: [Command.VOICE_STOP],
    },
];
//# sourceMappingURL=commands.js.map