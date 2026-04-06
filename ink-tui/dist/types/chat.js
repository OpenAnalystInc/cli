/**
 * Chat message types for the message list.
 *
 * These are the *rendered* message types that the ChatPanel displays.
 * They are derived from engine events (StreamDelta, ToolCallStart, etc.)
 * but represent the final display state of each message.
 */
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
let messageCounter = 0;
export function nextMessageId() {
    return `msg-${++messageCounter}`;
}
//# sourceMappingURL=chat.js.map