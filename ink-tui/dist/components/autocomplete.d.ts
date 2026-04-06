/**
 * Autocomplete — dropdown popup for `/` slash-command completion.
 *
 * Appears below the input when the user types `/`. Shows a filterable
 * list of commands with descriptions. Supports keyboard navigation.
 *
 * Keybinding priority: 7 (above input at 5, below dialogs at 9).
 *
 * Visual design:
 *   - Max 12 visible items, scrollable
 *   - Selected item: bold with accent background
 *   - Unselected items: dim
 *   - Each item shows: command name + description
 */
import React from 'react';
export interface AutocompleteItem {
    /** Command name (e.g. "/help"). */
    name: string;
    /** Short description (e.g. "Show all commands"). */
    description: string;
}
export interface AutocompleteProps {
    /** List of available items to display. */
    items: AutocompleteItem[];
    /** Currently selected index. */
    selectedIndex: number;
    /** Whether the dropdown is visible. */
    visible: boolean;
    /** Called when an item is accepted (Tab or Enter). */
    onSelect: (item: AutocompleteItem) => void;
    /** Called when the dropdown is dismissed (Esc). */
    onDismiss: () => void;
    /** Maximum visible items before scrolling. Default: 12. */
    maxVisible?: number;
}
export declare function Autocomplete({ items, selectedIndex, visible, maxVisible, }: AutocompleteProps): React.ReactElement | null;
