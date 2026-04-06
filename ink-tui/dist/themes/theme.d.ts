/**
 * OpenAnalyst CLI — Theme definitions.
 *
 * Two built-in themes: OADarkTheme (primary) and OALightTheme.
 * Each fully implements SemanticColors so every component has a
 * consistent, type-safe color contract.
 */
import type { SemanticColors, ThemeType } from './semantic-tokens.js';
export declare class OATheme {
    readonly name: string;
    readonly type: ThemeType;
    readonly colors: SemanticColors;
    constructor(name: string, type: ThemeType, colors: SemanticColors);
}
export declare const OADarkTheme: OATheme;
export declare const OALightTheme: OATheme;
