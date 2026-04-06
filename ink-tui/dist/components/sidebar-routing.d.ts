/**
 * SidebarRouting -- Model routing table section for the sidebar panel.
 *
 * Matches Ratatui design:
 *   explore   . model-name
 *   research  . model-name
 *   code      . model-name
 *   write     . model-name
 *
 * Category colors: explore=cyan, research=yellow, code=green, write=orange
 * Tier dot color: fast=cyan, balanced=yellow, capable=green
 * Enter cycles the tier for the selected category.
 */
import React from 'react';
import type { RoutingTable } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';
export interface SidebarRoutingProps {
    routing: RoutingTable;
    selectedIndex: number;
    isFocused: boolean;
    colors: SemanticColors;
}
export declare function SidebarRouting({ routing, selectedIndex, isFocused, colors, }: SidebarRoutingProps): React.ReactElement;
