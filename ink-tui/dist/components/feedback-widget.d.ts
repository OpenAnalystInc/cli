/**
 * FeedbackWidget — inline feedback prompt rendered below a KnowledgeCard.
 *
 * Mirrors the Rust tui-widgets/feedback_dialog.rs:
 *   Was this helpful?  [Y] [N] [Esc dismiss]
 *
 * Selected button has bold text + background color.
 * Unselected buttons have plain colored text.
 *
 * Keybinding: y/n/Esc in scroll mode (connected via useKeypress).
 * All colors from useTheme() semantic tokens.
 */
import React from 'react';
export interface FeedbackWidgetProps {
    /** Query ID to attach feedback to. */
    queryId: string;
    /** Currently selected button: 0=positive, 1=negative, 2=dismiss. */
    selectedIndex: number;
    /** Callback when a selection is made. */
    onSelect: (rating: 'positive' | 'negative' | 'dismiss') => void;
    /** Callback when selection changes (for cycling). */
    onSelectionChange: (index: number) => void;
    /** Whether this widget is active (receives keypresses). */
    isActive: boolean;
}
export declare function FeedbackWidget({ queryId, selectedIndex, onSelect, onSelectionChange, isActive, }: FeedbackWidgetProps): React.ReactElement;
