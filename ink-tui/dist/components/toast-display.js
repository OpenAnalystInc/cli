import { jsx as _jsx } from "react/jsx-runtime";
import { Text } from 'ink';
import { useUIState } from '../contexts/ui-state-context.js';
import { useTheme } from '../contexts/theme-context.js';
export function ToastDisplay() {
    const { toastMessage, toastType } = useUIState();
    const { colors } = useTheme();
    if (!toastMessage)
        return null;
    const colorMap = {
        info: colors.text.accent,
        warning: colors.status.warning,
        error: colors.status.error,
    };
    const color = colorMap[toastType] ?? colors.text.accent;
    return (_jsx(Text, { color: color, children: toastMessage }));
}
//# sourceMappingURL=toast-display.js.map