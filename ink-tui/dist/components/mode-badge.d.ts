/**
 * ModeBadge — small colored label badge rendered in input box borders.
 *
 * Used for: permission mode, active agent, git branch, model name.
 * All colors from caller (sourced from theme context).
 */
import React from 'react';
export interface ModeBadgeProps {
    /** Badge text content (will be padded with spaces). */
    label: string;
    /** Background color — from theme semantic tokens. */
    bgColor: string;
    /** Text color — from theme semantic tokens. */
    textColor: string;
    /** Optional bold styling. Default: true. */
    bold?: boolean;
}
export declare function ModeBadge({ label, bgColor, textColor, bold, }: ModeBadgeProps): React.ReactElement;
