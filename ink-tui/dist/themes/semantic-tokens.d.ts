/**
 * OpenAnalyst CLI — Semantic color token types.
 *
 * Every component in the TUI accesses colors through this interface.
 * The concrete values come from `theme.ts` (dark/light themes).
 */
export interface SemanticColors {
    text: {
        /** Main body text */
        primary: string;
        /** Dim / metadata text */
        secondary: string;
        /** Highlight accent */
        accent: string;
        /** Section headings */
        heading: string;
        /** Italic / emphasis */
        emphasis: string;
        /** Bold / strong */
        strong: string;
        /** Inline code spans */
        code: string;
        /** Hyperlinks */
        link: string;
        /** User input prompt icon */
        userPrompt: string;
        /** Slash-command text (e.g. /help) */
        slashCommand: string;
    };
    background: {
        /** Terminal background — usually transparent (empty string) */
        primary: string;
        /** Focused message row in scroll mode */
        focus: string;
        /** Input box background */
        input: string;
        /** Badge backgrounds by purpose */
        badge: {
            mode: string;
            agent: string;
            branch: string;
            model: string;
            contextFile: string;
        };
        /** Diff backgrounds */
        diff: {
            added: string;
            removed: string;
        };
    };
    border: {
        /** Normal borders */
        default: string;
        /** Collapsed / disabled borders */
        dim: string;
        /** Focused panel borders */
        focus: string;
        /** Input-box border by permission mode */
        input: {
            default: string;
            plan: string;
            acceptEdits: string;
            danger: string;
            streaming: string;
            agentRunning: string;
        };
    };
    status: {
        running: string;
        done: string;
        error: string;
        warning: string;
        pending: string;
    };
    spinner: {
        /** Color while work is in progress */
        active: string;
        /** 8-color gradient keyframes for smooth cycling */
        gradient: readonly string[];
    };
    toolCard: {
        running: string;
        completed: string;
        failed: string;
    };
    knowledgeCard: {
        border: string;
        expandedBorder: string;
        tabActive: string;
        tabInactive: string;
        score: string;
        citation: string;
        cache: string;
        graph: string;
        answerDivider: string;
    };
    sidebar: {
        border: string;
        sectionHeader: string;
        itemDefault: string;
        itemSelected: string;
        fileRead: string;
        fileEdited: string;
        fileCreated: string;
    };
    diff: {
        added: string;
        removed: string;
    };
    dialog: {
        border: string;
        allowSelected: string;
        allowUnselected: string;
        denySelected: string;
        denyUnselected: string;
    };
    /** Code syntax highlighting colors (hljs class mapping) */
    syntax: {
        keyword: string;
        string: string;
        number: string;
        comment: string;
        type: string;
        function: string;
        variable: string;
        builtIn: string;
        link: string;
        tag: string;
    };
}
export type ThemeType = 'dark' | 'light';
