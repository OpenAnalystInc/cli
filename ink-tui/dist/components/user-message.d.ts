/**
 * UserMessage — renders a single user prompt in the chat panel.
 *
 * - Cyan `>` prompt icon for normal messages
 * - Orange `>` for /slash commands
 * - Bold text for the user's input
 */
import React from 'react';
export interface UserMessageProps {
    /** The user's input text. */
    text: string;
    /** Whether this is a /slash command (changes prompt color). */
    isSlashCommand: boolean;
    /** Whether this message is currently focused in scroll mode. */
    isFocused?: boolean;
}
export declare const UserMessage: React.MemoExoticComponent<({ text, isSlashCommand, isFocused, }: UserMessageProps) => React.ReactElement>;
