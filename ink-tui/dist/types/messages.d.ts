/**
 * Complete message type definitions for the Rust engine <-> Ink TUI protocol.
 *
 * Every event (Engine -> TUI) and action (TUI -> Engine) has:
 * 1. A Zod schema for runtime validation
 * 2. A TypeScript type inferred from the schema
 *
 * Schemas are grouped into discriminated unions so a single `parse()` call
 * can validate any incoming message and narrow its type.
 */
import { z } from 'zod';
export declare const AgentTypeSchema: z.ZodEnum<["Primary", "Explore", "Plan", "General"]>;
export type AgentType = z.infer<typeof AgentTypeSchema>;
export declare const AgentStatusSchema: z.ZodEnum<["Pending", "Running", "Completed", "Failed"]>;
export type AgentStatus = z.infer<typeof AgentStatusSchema>;
export declare const DiffLineSchema: z.ZodDiscriminatedUnion<"kind", [z.ZodObject<{
    kind: z.ZodLiteral<"context">;
    text: z.ZodString;
}, "strip", z.ZodTypeAny, {
    kind: "context";
    text: string;
}, {
    kind: "context";
    text: string;
}>, z.ZodObject<{
    kind: z.ZodLiteral<"added">;
    text: z.ZodString;
}, "strip", z.ZodTypeAny, {
    kind: "added";
    text: string;
}, {
    kind: "added";
    text: string;
}>, z.ZodObject<{
    kind: z.ZodLiteral<"removed">;
    text: z.ZodString;
}, "strip", z.ZodTypeAny, {
    kind: "removed";
    text: string;
}, {
    kind: "removed";
    text: string;
}>]>;
export type DiffLine = z.infer<typeof DiffLineSchema>;
export declare const DiffHunkSchema: z.ZodObject<{
    oldStart: z.ZodNumber;
    newStart: z.ZodNumber;
    lines: z.ZodArray<z.ZodDiscriminatedUnion<"kind", [z.ZodObject<{
        kind: z.ZodLiteral<"context">;
        text: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        kind: "context";
        text: string;
    }, {
        kind: "context";
        text: string;
    }>, z.ZodObject<{
        kind: z.ZodLiteral<"added">;
        text: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        kind: "added";
        text: string;
    }, {
        kind: "added";
        text: string;
    }>, z.ZodObject<{
        kind: z.ZodLiteral<"removed">;
        text: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        kind: "removed";
        text: string;
    }, {
        kind: "removed";
        text: string;
    }>]>, "many">;
}, "strip", z.ZodTypeAny, {
    oldStart: number;
    newStart: number;
    lines: ({
        kind: "context";
        text: string;
    } | {
        kind: "added";
        text: string;
    } | {
        kind: "removed";
        text: string;
    })[];
}, {
    oldStart: number;
    newStart: number;
    lines: ({
        kind: "context";
        text: string;
    } | {
        kind: "added";
        text: string;
    } | {
        kind: "removed";
        text: string;
    })[];
}>;
export type DiffHunk = z.infer<typeof DiffHunkSchema>;
export declare const DiffInfoSchema: z.ZodObject<{
    filePath: z.ZodString;
    added: z.ZodNumber;
    removed: z.ZodNumber;
    hunks: z.ZodArray<z.ZodObject<{
        oldStart: z.ZodNumber;
        newStart: z.ZodNumber;
        lines: z.ZodArray<z.ZodDiscriminatedUnion<"kind", [z.ZodObject<{
            kind: z.ZodLiteral<"context">;
            text: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            kind: "context";
            text: string;
        }, {
            kind: "context";
            text: string;
        }>, z.ZodObject<{
            kind: z.ZodLiteral<"added">;
            text: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            kind: "added";
            text: string;
        }, {
            kind: "added";
            text: string;
        }>, z.ZodObject<{
            kind: z.ZodLiteral<"removed">;
            text: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            kind: "removed";
            text: string;
        }, {
            kind: "removed";
            text: string;
        }>]>, "many">;
    }, "strip", z.ZodTypeAny, {
        oldStart: number;
        newStart: number;
        lines: ({
            kind: "context";
            text: string;
        } | {
            kind: "added";
            text: string;
        } | {
            kind: "removed";
            text: string;
        })[];
    }, {
        oldStart: number;
        newStart: number;
        lines: ({
            kind: "context";
            text: string;
        } | {
            kind: "added";
            text: string;
        } | {
            kind: "removed";
            text: string;
        })[];
    }>, "many">;
}, "strip", z.ZodTypeAny, {
    added: number;
    removed: number;
    filePath: string;
    hunks: {
        oldStart: number;
        newStart: number;
        lines: ({
            kind: "context";
            text: string;
        } | {
            kind: "added";
            text: string;
        } | {
            kind: "removed";
            text: string;
        })[];
    }[];
}, {
    added: number;
    removed: number;
    filePath: string;
    hunks: {
        oldStart: number;
        newStart: number;
        lines: ({
            kind: "context";
            text: string;
        } | {
            kind: "added";
            text: string;
        } | {
            kind: "removed";
            text: string;
        })[];
    }[];
}>;
export type DiffInfo = z.infer<typeof DiffInfoSchema>;
export declare const KbChunkResultSchema: z.ZodObject<{
    chunkId: z.ZodString;
    text: z.ZodString;
    snippet: z.ZodString;
    score: z.ZodNumber;
    categoryLabel: z.ZodString;
    contentType: z.ZodString;
    citationLabel: z.ZodString;
    hasTimestamps: z.ZodBoolean;
    graphExpanded: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    text: string;
    chunkId: string;
    snippet: string;
    score: number;
    categoryLabel: string;
    contentType: string;
    citationLabel: string;
    hasTimestamps: boolean;
    graphExpanded: boolean;
}, {
    text: string;
    chunkId: string;
    snippet: string;
    score: number;
    categoryLabel: string;
    contentType: string;
    citationLabel: string;
    hasTimestamps: boolean;
    graphExpanded: boolean;
}>;
export type KbChunkResult = z.infer<typeof KbChunkResultSchema>;
export declare const SubQuestionResultSchema: z.ZodObject<{
    subQuestion: z.ZodString;
    intent: z.ZodString;
    results: z.ZodArray<z.ZodObject<{
        chunkId: z.ZodString;
        text: z.ZodString;
        snippet: z.ZodString;
        score: z.ZodNumber;
        categoryLabel: z.ZodString;
        contentType: z.ZodString;
        citationLabel: z.ZodString;
        hasTimestamps: z.ZodBoolean;
        graphExpanded: z.ZodBoolean;
    }, "strip", z.ZodTypeAny, {
        text: string;
        chunkId: string;
        snippet: string;
        score: number;
        categoryLabel: string;
        contentType: string;
        citationLabel: string;
        hasTimestamps: boolean;
        graphExpanded: boolean;
    }, {
        text: string;
        chunkId: string;
        snippet: string;
        score: number;
        categoryLabel: string;
        contentType: string;
        citationLabel: string;
        hasTimestamps: boolean;
        graphExpanded: boolean;
    }>, "many">;
}, "strip", z.ZodTypeAny, {
    subQuestion: string;
    intent: string;
    results: {
        text: string;
        chunkId: string;
        snippet: string;
        score: number;
        categoryLabel: string;
        contentType: string;
        citationLabel: string;
        hasTimestamps: boolean;
        graphExpanded: boolean;
    }[];
}, {
    subQuestion: string;
    intent: string;
    results: {
        text: string;
        chunkId: string;
        snippet: string;
        score: number;
        categoryLabel: string;
        contentType: string;
        citationLabel: string;
        hasTimestamps: boolean;
        graphExpanded: boolean;
    }[];
}>;
export type SubQuestionResult = z.infer<typeof SubQuestionResultSchema>;
export declare const AgentInfoSchema: z.ZodObject<{
    agentId: z.ZodString;
    agentType: z.ZodEnum<["Primary", "Explore", "Plan", "General"]>;
    taskSummary: z.ZodString;
    status: z.ZodEnum<["Pending", "Running", "Completed", "Failed"]>;
}, "strip", z.ZodTypeAny, {
    status: "Pending" | "Running" | "Completed" | "Failed";
    agentId: string;
    agentType: "Primary" | "Explore" | "Plan" | "General";
    taskSummary: string;
}, {
    status: "Pending" | "Running" | "Completed" | "Failed";
    agentId: string;
    agentType: "Primary" | "Explore" | "Plan" | "General";
    taskSummary: string;
}>;
export type AgentInfo = z.infer<typeof AgentInfoSchema>;
export declare const FileActionSchema: z.ZodEnum<["read", "edited", "created"]>;
export type FileAction = z.infer<typeof FileActionSchema>;
export declare const FileInfoSchema: z.ZodObject<{
    path: z.ZodString;
    action: z.ZodEnum<["read", "edited", "created"]>;
}, "strip", z.ZodTypeAny, {
    path: string;
    action: "read" | "edited" | "created";
}, {
    path: string;
    action: "read" | "edited" | "created";
}>;
export type FileInfo = z.infer<typeof FileInfoSchema>;
export declare const PlanInfoSchema: z.ZodObject<{
    name: z.ZodString;
    status: z.ZodEnum<["todo", "in_progress", "done"]>;
}, "strip", z.ZodTypeAny, {
    status: "todo" | "in_progress" | "done";
    name: string;
}, {
    status: "todo" | "in_progress" | "done";
    name: string;
}>;
export type PlanInfo = z.infer<typeof PlanInfoSchema>;
export declare const ActionCategorySchema: z.ZodEnum<["explore", "research", "code", "write"]>;
export type ActionCategory = z.infer<typeof ActionCategorySchema>;
export declare const RoutingEntrySchema: z.ZodObject<{
    model: z.ZodString;
    tier: z.ZodString;
}, "strip", z.ZodTypeAny, {
    model: string;
    tier: string;
}, {
    model: string;
    tier: string;
}>;
export type RoutingEntry = z.infer<typeof RoutingEntrySchema>;
export declare const RoutingTableSchema: z.ZodObject<{
    explore: z.ZodObject<{
        model: z.ZodString;
        tier: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        model: string;
        tier: string;
    }, {
        model: string;
        tier: string;
    }>;
    research: z.ZodObject<{
        model: z.ZodString;
        tier: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        model: string;
        tier: string;
    }, {
        model: string;
        tier: string;
    }>;
    code: z.ZodObject<{
        model: z.ZodString;
        tier: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        model: string;
        tier: string;
    }, {
        model: string;
        tier: string;
    }>;
    write: z.ZodObject<{
        model: z.ZodString;
        tier: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        model: string;
        tier: string;
    }, {
        model: string;
        tier: string;
    }>;
}, "strip", z.ZodTypeAny, {
    code: {
        model: string;
        tier: string;
    };
    explore: {
        model: string;
        tier: string;
    };
    research: {
        model: string;
        tier: string;
    };
    write: {
        model: string;
        tier: string;
    };
}, {
    code: {
        model: string;
        tier: string;
    };
    explore: {
        model: string;
        tier: string;
    };
    research: {
        model: string;
        tier: string;
    };
    write: {
        model: string;
        tier: string;
    };
}>;
export type RoutingTable = z.infer<typeof RoutingTableSchema>;
export declare const ActivityInfoSchema: z.ZodObject<{
    backgroundTasks: z.ZodNumber;
    toolCallCount: z.ZodNumber;
    mcpServers: z.ZodNumber;
    creditBalance: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    backgroundTasks: number;
    toolCallCount: number;
    mcpServers: number;
    creditBalance?: string | undefined;
}, {
    backgroundTasks: number;
    toolCallCount: number;
    mcpServers: number;
    creditBalance?: string | undefined;
}>;
export type ActivityInfo = z.infer<typeof ActivityInfoSchema>;
export declare const AgentPhaseSchema: z.ZodEnum<["idle", "thinking", "reading_file", "editing_file", "running_bash", "searching", "done", "error"]>;
export type AgentPhase = z.infer<typeof AgentPhaseSchema>;
export declare const PermissionModeSchema: z.ZodEnum<["prompt", "read-only", "workspace-write", "danger-full-access"]>;
export type PermissionMode = z.infer<typeof PermissionModeSchema>;
export declare const SystemLevelSchema: z.ZodEnum<["info", "warning", "error"]>;
export type SystemLevel = z.infer<typeof SystemLevelSchema>;
export declare const StreamDeltaSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"stream_delta">;
    agentId: z.ZodString;
    content: z.ZodString;
    done: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    type: "stream_delta";
    timestamp: number;
    agentId: string;
    done: boolean;
    content: string;
    id?: string | undefined;
}, {
    type: "stream_delta";
    timestamp: number;
    agentId: string;
    done: boolean;
    content: string;
    id?: string | undefined;
}>;
export type StreamDelta = z.infer<typeof StreamDeltaSchema>;
export declare const StreamEndSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"stream_end">;
    agentId: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "stream_end";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}, {
    type: "stream_end";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}>;
export type StreamEnd = z.infer<typeof StreamEndSchema>;
export declare const ToolCallStartSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"tool_call_start">;
    agentId: z.ZodString;
    toolId: z.ZodString;
    toolName: z.ZodString;
    inputPreview: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "tool_call_start";
    timestamp: number;
    agentId: string;
    toolId: string;
    toolName: string;
    inputPreview: string;
    id?: string | undefined;
}, {
    type: "tool_call_start";
    timestamp: number;
    agentId: string;
    toolId: string;
    toolName: string;
    inputPreview: string;
    id?: string | undefined;
}>;
export type ToolCallStart = z.infer<typeof ToolCallStartSchema>;
export declare const ToolCallUpdateSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"tool_call_update">;
    agentId: z.ZodString;
    toolId: z.ZodString;
    output: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "tool_call_update";
    timestamp: number;
    agentId: string;
    toolId: string;
    output: string;
    id?: string | undefined;
}, {
    type: "tool_call_update";
    timestamp: number;
    agentId: string;
    toolId: string;
    output: string;
    id?: string | undefined;
}>;
export type ToolCallUpdate = z.infer<typeof ToolCallUpdateSchema>;
export declare const ToolCallCompleteSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"tool_call_complete">;
    agentId: z.ZodString;
    toolId: z.ZodString;
    status: z.ZodEnum<["completed", "failed"]>;
    output: z.ZodString;
    durationMs: z.ZodNumber;
    diff: z.ZodOptional<z.ZodObject<{
        filePath: z.ZodString;
        added: z.ZodNumber;
        removed: z.ZodNumber;
        hunks: z.ZodArray<z.ZodObject<{
            oldStart: z.ZodNumber;
            newStart: z.ZodNumber;
            lines: z.ZodArray<z.ZodDiscriminatedUnion<"kind", [z.ZodObject<{
                kind: z.ZodLiteral<"context">;
                text: z.ZodString;
            }, "strip", z.ZodTypeAny, {
                kind: "context";
                text: string;
            }, {
                kind: "context";
                text: string;
            }>, z.ZodObject<{
                kind: z.ZodLiteral<"added">;
                text: z.ZodString;
            }, "strip", z.ZodTypeAny, {
                kind: "added";
                text: string;
            }, {
                kind: "added";
                text: string;
            }>, z.ZodObject<{
                kind: z.ZodLiteral<"removed">;
                text: z.ZodString;
            }, "strip", z.ZodTypeAny, {
                kind: "removed";
                text: string;
            }, {
                kind: "removed";
                text: string;
            }>]>, "many">;
        }, "strip", z.ZodTypeAny, {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }, {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }>, "many">;
    }, "strip", z.ZodTypeAny, {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    }, {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    }>>;
}, "strip", z.ZodTypeAny, {
    type: "tool_call_complete";
    timestamp: number;
    status: "completed" | "failed";
    agentId: string;
    toolId: string;
    output: string;
    durationMs: number;
    id?: string | undefined;
    diff?: {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    } | undefined;
}, {
    type: "tool_call_complete";
    timestamp: number;
    status: "completed" | "failed";
    agentId: string;
    toolId: string;
    output: string;
    durationMs: number;
    id?: string | undefined;
    diff?: {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    } | undefined;
}>;
export type ToolCallComplete = z.infer<typeof ToolCallCompleteSchema>;
export declare const PermissionRequestSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"permission_request">;
    requestId: z.ZodString;
    agentId: z.ZodString;
    toolName: z.ZodString;
    toolInput: z.ZodString;
    requiredMode: z.ZodEnum<["prompt", "read-only", "workspace-write", "danger-full-access"]>;
    filePath: z.ZodOptional<z.ZodString>;
    description: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "permission_request";
    timestamp: number;
    agentId: string;
    toolName: string;
    requestId: string;
    toolInput: string;
    requiredMode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
    filePath?: string | undefined;
    description?: string | undefined;
}, {
    type: "permission_request";
    timestamp: number;
    agentId: string;
    toolName: string;
    requestId: string;
    toolInput: string;
    requiredMode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
    filePath?: string | undefined;
    description?: string | undefined;
}>;
export type PermissionRequest = z.infer<typeof PermissionRequestSchema>;
export declare const AskUserRequestSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"ask_user_request">;
    requestId: z.ZodString;
    agentId: z.ZodString;
    question: z.ZodString;
    options: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    defaultValue: z.ZodOptional<z.ZodString>;
    allowFreeText: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    type: "ask_user_request";
    timestamp: number;
    agentId: string;
    requestId: string;
    question: string;
    allowFreeText: boolean;
    id?: string | undefined;
    options?: string[] | undefined;
    defaultValue?: string | undefined;
}, {
    type: "ask_user_request";
    timestamp: number;
    agentId: string;
    requestId: string;
    question: string;
    allowFreeText: boolean;
    id?: string | undefined;
    options?: string[] | undefined;
    defaultValue?: string | undefined;
}>;
export type AskUserRequest = z.infer<typeof AskUserRequestSchema>;
export declare const StatusUpdateSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"status_update">;
    phase: z.ZodEnum<["idle", "thinking", "reading_file", "editing_file", "running_bash", "searching", "done", "error"]>;
    label: z.ZodOptional<z.ZodString>;
    elapsedMs: z.ZodNumber;
    tokensRemaining: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    type: "status_update";
    timestamp: number;
    phase: "error" | "done" | "idle" | "thinking" | "reading_file" | "editing_file" | "running_bash" | "searching";
    elapsedMs: number;
    id?: string | undefined;
    label?: string | undefined;
    tokensRemaining?: number | undefined;
}, {
    type: "status_update";
    timestamp: number;
    phase: "error" | "done" | "idle" | "thinking" | "reading_file" | "editing_file" | "running_bash" | "searching";
    elapsedMs: number;
    id?: string | undefined;
    label?: string | undefined;
    tokensRemaining?: number | undefined;
}>;
export type StatusUpdate = z.infer<typeof StatusUpdateSchema>;
export declare const AgentSpawnedSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_spawned">;
    agentId: z.ZodString;
    parentId: z.ZodOptional<z.ZodString>;
    agentType: z.ZodEnum<["Primary", "Explore", "Plan", "General"]>;
    task: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "agent_spawned";
    timestamp: number;
    agentId: string;
    agentType: "Primary" | "Explore" | "Plan" | "General";
    task: string;
    id?: string | undefined;
    parentId?: string | undefined;
}, {
    type: "agent_spawned";
    timestamp: number;
    agentId: string;
    agentType: "Primary" | "Explore" | "Plan" | "General";
    task: string;
    id?: string | undefined;
    parentId?: string | undefined;
}>;
export type AgentSpawned = z.infer<typeof AgentSpawnedSchema>;
export declare const AgentStatusChangedSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_status_changed">;
    agentId: z.ZodString;
    status: z.ZodEnum<["Pending", "Running", "Completed", "Failed"]>;
}, "strip", z.ZodTypeAny, {
    type: "agent_status_changed";
    timestamp: number;
    status: "Pending" | "Running" | "Completed" | "Failed";
    agentId: string;
    id?: string | undefined;
}, {
    type: "agent_status_changed";
    timestamp: number;
    status: "Pending" | "Running" | "Completed" | "Failed";
    agentId: string;
    id?: string | undefined;
}>;
export type AgentStatusChanged = z.infer<typeof AgentStatusChangedSchema>;
export declare const AgentCompletedSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_completed">;
    agentId: z.ZodString;
    result: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "agent_completed";
    timestamp: number;
    agentId: string;
    result: string;
    id?: string | undefined;
}, {
    type: "agent_completed";
    timestamp: number;
    agentId: string;
    result: string;
    id?: string | undefined;
}>;
export type AgentCompleted = z.infer<typeof AgentCompletedSchema>;
export declare const AgentFailedSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_failed">;
    agentId: z.ZodString;
    error: z.ZodString;
}, "strip", z.ZodTypeAny, {
    error: string;
    type: "agent_failed";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}, {
    error: string;
    type: "agent_failed";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}>;
export type AgentFailed = z.infer<typeof AgentFailedSchema>;
export declare const UsageUpdateSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"usage_update">;
    agentId: z.ZodString;
    inputTokens: z.ZodNumber;
    outputTokens: z.ZodNumber;
}, "strip", z.ZodTypeAny, {
    type: "usage_update";
    timestamp: number;
    agentId: string;
    inputTokens: number;
    outputTokens: number;
    id?: string | undefined;
}, {
    type: "usage_update";
    timestamp: number;
    agentId: string;
    inputTokens: number;
    outputTokens: number;
    id?: string | undefined;
}>;
export type UsageUpdate = z.infer<typeof UsageUpdateSchema>;
export declare const KbResultSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"kb_result">;
    queryId: z.ZodNumber;
    query: z.ZodString;
    intent: z.ZodString;
    subQuestions: z.ZodArray<z.ZodObject<{
        subQuestion: z.ZodString;
        intent: z.ZodString;
        results: z.ZodArray<z.ZodObject<{
            chunkId: z.ZodString;
            text: z.ZodString;
            snippet: z.ZodString;
            score: z.ZodNumber;
            categoryLabel: z.ZodString;
            contentType: z.ZodString;
            citationLabel: z.ZodString;
            hasTimestamps: z.ZodBoolean;
            graphExpanded: z.ZodBoolean;
        }, "strip", z.ZodTypeAny, {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }, {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }>, "many">;
    }, "strip", z.ZodTypeAny, {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }, {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }>, "many">;
    answer: z.ZodOptional<z.ZodString>;
    latencyMs: z.ZodNumber;
    fromCache: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    type: "kb_result";
    timestamp: number;
    intent: string;
    queryId: number;
    query: string;
    subQuestions: {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }[];
    latencyMs: number;
    fromCache: boolean;
    id?: string | undefined;
    answer?: string | undefined;
}, {
    type: "kb_result";
    timestamp: number;
    intent: string;
    queryId: number;
    query: string;
    subQuestions: {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }[];
    latencyMs: number;
    fromCache: boolean;
    id?: string | undefined;
    answer?: string | undefined;
}>;
export type KbResult = z.infer<typeof KbResultSchema>;
export declare const SystemMessageSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"system_message">;
    content: z.ZodString;
    level: z.ZodEnum<["info", "warning", "error"]>;
}, "strip", z.ZodTypeAny, {
    type: "system_message";
    timestamp: number;
    content: string;
    level: "error" | "info" | "warning";
    id?: string | undefined;
}, {
    type: "system_message";
    timestamp: number;
    content: string;
    level: "error" | "info" | "warning";
    id?: string | undefined;
}>;
export type SystemMessage = z.infer<typeof SystemMessageSchema>;
export declare const BannerSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"banner">;
    version: z.ZodString;
    displayName: z.ZodString;
    email: z.ZodOptional<z.ZodString>;
    organization: z.ZodOptional<z.ZodString>;
    provider: z.ZodString;
    modelDisplay: z.ZodString;
    workingDir: z.ZodString;
    credits: z.ZodOptional<z.ZodString>;
    tips: z.ZodArray<z.ZodString, "many">;
}, "strip", z.ZodTypeAny, {
    type: "banner";
    timestamp: number;
    version: string;
    displayName: string;
    provider: string;
    modelDisplay: string;
    workingDir: string;
    tips: string[];
    id?: string | undefined;
    email?: string | undefined;
    organization?: string | undefined;
    credits?: string | undefined;
}, {
    type: "banner";
    timestamp: number;
    version: string;
    displayName: string;
    provider: string;
    modelDisplay: string;
    workingDir: string;
    tips: string[];
    id?: string | undefined;
    email?: string | undefined;
    organization?: string | undefined;
    credits?: string | undefined;
}>;
export type Banner = z.infer<typeof BannerSchema>;
export declare const SidebarUpdateSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"sidebar_update">;
    agents: z.ZodArray<z.ZodObject<{
        agentId: z.ZodString;
        agentType: z.ZodEnum<["Primary", "Explore", "Plan", "General"]>;
        taskSummary: z.ZodString;
        status: z.ZodEnum<["Pending", "Running", "Completed", "Failed"]>;
    }, "strip", z.ZodTypeAny, {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }, {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }>, "many">;
    files: z.ZodArray<z.ZodObject<{
        path: z.ZodString;
        action: z.ZodEnum<["read", "edited", "created"]>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        action: "read" | "edited" | "created";
    }, {
        path: string;
        action: "read" | "edited" | "created";
    }>, "many">;
    plans: z.ZodArray<z.ZodObject<{
        name: z.ZodString;
        status: z.ZodEnum<["todo", "in_progress", "done"]>;
    }, "strip", z.ZodTypeAny, {
        status: "todo" | "in_progress" | "done";
        name: string;
    }, {
        status: "todo" | "in_progress" | "done";
        name: string;
    }>, "many">;
    routing: z.ZodObject<{
        explore: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
        research: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
        code: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
        write: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
    }, "strip", z.ZodTypeAny, {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    }, {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    }>;
    activity: z.ZodObject<{
        backgroundTasks: z.ZodNumber;
        toolCallCount: z.ZodNumber;
        mcpServers: z.ZodNumber;
        creditBalance: z.ZodOptional<z.ZodString>;
    }, "strip", z.ZodTypeAny, {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    }, {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    }>;
}, "strip", z.ZodTypeAny, {
    type: "sidebar_update";
    timestamp: number;
    agents: {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }[];
    files: {
        path: string;
        action: "read" | "edited" | "created";
    }[];
    plans: {
        status: "todo" | "in_progress" | "done";
        name: string;
    }[];
    routing: {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    };
    activity: {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    };
    id?: string | undefined;
}, {
    type: "sidebar_update";
    timestamp: number;
    agents: {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }[];
    files: {
        path: string;
        action: "read" | "edited" | "created";
    }[];
    plans: {
        status: "todo" | "in_progress" | "done";
        name: string;
    }[];
    routing: {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    };
    activity: {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    };
    id?: string | undefined;
}>;
export type SidebarUpdate = z.infer<typeof SidebarUpdateSchema>;
export declare const ModelInfoSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"model_info">;
    name: z.ZodString;
    provider: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "model_info";
    timestamp: number;
    name: string;
    provider: string;
    id?: string | undefined;
}, {
    type: "model_info";
    timestamp: number;
    name: string;
    provider: string;
    id?: string | undefined;
}>;
export type ModelInfo = z.infer<typeof ModelInfoSchema>;
export declare const ContextFilesUpdateSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"context_files_update">;
    files: z.ZodArray<z.ZodObject<{
        path: z.ZodString;
        action: z.ZodEnum<["added", "removed"]>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        action: "added" | "removed";
    }, {
        path: string;
        action: "added" | "removed";
    }>, "many">;
}, "strip", z.ZodTypeAny, {
    type: "context_files_update";
    timestamp: number;
    files: {
        path: string;
        action: "added" | "removed";
    }[];
    id?: string | undefined;
}, {
    type: "context_files_update";
    timestamp: number;
    files: {
        path: string;
        action: "added" | "removed";
    }[];
    id?: string | undefined;
}>;
export type ContextFilesUpdate = z.infer<typeof ContextFilesUpdateSchema>;
export declare const EngineEventSchema: z.ZodDiscriminatedUnion<"type", [z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"stream_delta">;
    agentId: z.ZodString;
    content: z.ZodString;
    done: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    type: "stream_delta";
    timestamp: number;
    agentId: string;
    done: boolean;
    content: string;
    id?: string | undefined;
}, {
    type: "stream_delta";
    timestamp: number;
    agentId: string;
    done: boolean;
    content: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"stream_end">;
    agentId: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "stream_end";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}, {
    type: "stream_end";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"tool_call_start">;
    agentId: z.ZodString;
    toolId: z.ZodString;
    toolName: z.ZodString;
    inputPreview: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "tool_call_start";
    timestamp: number;
    agentId: string;
    toolId: string;
    toolName: string;
    inputPreview: string;
    id?: string | undefined;
}, {
    type: "tool_call_start";
    timestamp: number;
    agentId: string;
    toolId: string;
    toolName: string;
    inputPreview: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"tool_call_update">;
    agentId: z.ZodString;
    toolId: z.ZodString;
    output: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "tool_call_update";
    timestamp: number;
    agentId: string;
    toolId: string;
    output: string;
    id?: string | undefined;
}, {
    type: "tool_call_update";
    timestamp: number;
    agentId: string;
    toolId: string;
    output: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"tool_call_complete">;
    agentId: z.ZodString;
    toolId: z.ZodString;
    status: z.ZodEnum<["completed", "failed"]>;
    output: z.ZodString;
    durationMs: z.ZodNumber;
    diff: z.ZodOptional<z.ZodObject<{
        filePath: z.ZodString;
        added: z.ZodNumber;
        removed: z.ZodNumber;
        hunks: z.ZodArray<z.ZodObject<{
            oldStart: z.ZodNumber;
            newStart: z.ZodNumber;
            lines: z.ZodArray<z.ZodDiscriminatedUnion<"kind", [z.ZodObject<{
                kind: z.ZodLiteral<"context">;
                text: z.ZodString;
            }, "strip", z.ZodTypeAny, {
                kind: "context";
                text: string;
            }, {
                kind: "context";
                text: string;
            }>, z.ZodObject<{
                kind: z.ZodLiteral<"added">;
                text: z.ZodString;
            }, "strip", z.ZodTypeAny, {
                kind: "added";
                text: string;
            }, {
                kind: "added";
                text: string;
            }>, z.ZodObject<{
                kind: z.ZodLiteral<"removed">;
                text: z.ZodString;
            }, "strip", z.ZodTypeAny, {
                kind: "removed";
                text: string;
            }, {
                kind: "removed";
                text: string;
            }>]>, "many">;
        }, "strip", z.ZodTypeAny, {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }, {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }>, "many">;
    }, "strip", z.ZodTypeAny, {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    }, {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    }>>;
}, "strip", z.ZodTypeAny, {
    type: "tool_call_complete";
    timestamp: number;
    status: "completed" | "failed";
    agentId: string;
    toolId: string;
    output: string;
    durationMs: number;
    id?: string | undefined;
    diff?: {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    } | undefined;
}, {
    type: "tool_call_complete";
    timestamp: number;
    status: "completed" | "failed";
    agentId: string;
    toolId: string;
    output: string;
    durationMs: number;
    id?: string | undefined;
    diff?: {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    } | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"permission_request">;
    requestId: z.ZodString;
    agentId: z.ZodString;
    toolName: z.ZodString;
    toolInput: z.ZodString;
    requiredMode: z.ZodEnum<["prompt", "read-only", "workspace-write", "danger-full-access"]>;
    filePath: z.ZodOptional<z.ZodString>;
    description: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "permission_request";
    timestamp: number;
    agentId: string;
    toolName: string;
    requestId: string;
    toolInput: string;
    requiredMode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
    filePath?: string | undefined;
    description?: string | undefined;
}, {
    type: "permission_request";
    timestamp: number;
    agentId: string;
    toolName: string;
    requestId: string;
    toolInput: string;
    requiredMode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
    filePath?: string | undefined;
    description?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"ask_user_request">;
    requestId: z.ZodString;
    agentId: z.ZodString;
    question: z.ZodString;
    options: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    defaultValue: z.ZodOptional<z.ZodString>;
    allowFreeText: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    type: "ask_user_request";
    timestamp: number;
    agentId: string;
    requestId: string;
    question: string;
    allowFreeText: boolean;
    id?: string | undefined;
    options?: string[] | undefined;
    defaultValue?: string | undefined;
}, {
    type: "ask_user_request";
    timestamp: number;
    agentId: string;
    requestId: string;
    question: string;
    allowFreeText: boolean;
    id?: string | undefined;
    options?: string[] | undefined;
    defaultValue?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"status_update">;
    phase: z.ZodEnum<["idle", "thinking", "reading_file", "editing_file", "running_bash", "searching", "done", "error"]>;
    label: z.ZodOptional<z.ZodString>;
    elapsedMs: z.ZodNumber;
    tokensRemaining: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    type: "status_update";
    timestamp: number;
    phase: "error" | "done" | "idle" | "thinking" | "reading_file" | "editing_file" | "running_bash" | "searching";
    elapsedMs: number;
    id?: string | undefined;
    label?: string | undefined;
    tokensRemaining?: number | undefined;
}, {
    type: "status_update";
    timestamp: number;
    phase: "error" | "done" | "idle" | "thinking" | "reading_file" | "editing_file" | "running_bash" | "searching";
    elapsedMs: number;
    id?: string | undefined;
    label?: string | undefined;
    tokensRemaining?: number | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_spawned">;
    agentId: z.ZodString;
    parentId: z.ZodOptional<z.ZodString>;
    agentType: z.ZodEnum<["Primary", "Explore", "Plan", "General"]>;
    task: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "agent_spawned";
    timestamp: number;
    agentId: string;
    agentType: "Primary" | "Explore" | "Plan" | "General";
    task: string;
    id?: string | undefined;
    parentId?: string | undefined;
}, {
    type: "agent_spawned";
    timestamp: number;
    agentId: string;
    agentType: "Primary" | "Explore" | "Plan" | "General";
    task: string;
    id?: string | undefined;
    parentId?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_status_changed">;
    agentId: z.ZodString;
    status: z.ZodEnum<["Pending", "Running", "Completed", "Failed"]>;
}, "strip", z.ZodTypeAny, {
    type: "agent_status_changed";
    timestamp: number;
    status: "Pending" | "Running" | "Completed" | "Failed";
    agentId: string;
    id?: string | undefined;
}, {
    type: "agent_status_changed";
    timestamp: number;
    status: "Pending" | "Running" | "Completed" | "Failed";
    agentId: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_completed">;
    agentId: z.ZodString;
    result: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "agent_completed";
    timestamp: number;
    agentId: string;
    result: string;
    id?: string | undefined;
}, {
    type: "agent_completed";
    timestamp: number;
    agentId: string;
    result: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_failed">;
    agentId: z.ZodString;
    error: z.ZodString;
}, "strip", z.ZodTypeAny, {
    error: string;
    type: "agent_failed";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}, {
    error: string;
    type: "agent_failed";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"usage_update">;
    agentId: z.ZodString;
    inputTokens: z.ZodNumber;
    outputTokens: z.ZodNumber;
}, "strip", z.ZodTypeAny, {
    type: "usage_update";
    timestamp: number;
    agentId: string;
    inputTokens: number;
    outputTokens: number;
    id?: string | undefined;
}, {
    type: "usage_update";
    timestamp: number;
    agentId: string;
    inputTokens: number;
    outputTokens: number;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"kb_result">;
    queryId: z.ZodNumber;
    query: z.ZodString;
    intent: z.ZodString;
    subQuestions: z.ZodArray<z.ZodObject<{
        subQuestion: z.ZodString;
        intent: z.ZodString;
        results: z.ZodArray<z.ZodObject<{
            chunkId: z.ZodString;
            text: z.ZodString;
            snippet: z.ZodString;
            score: z.ZodNumber;
            categoryLabel: z.ZodString;
            contentType: z.ZodString;
            citationLabel: z.ZodString;
            hasTimestamps: z.ZodBoolean;
            graphExpanded: z.ZodBoolean;
        }, "strip", z.ZodTypeAny, {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }, {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }>, "many">;
    }, "strip", z.ZodTypeAny, {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }, {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }>, "many">;
    answer: z.ZodOptional<z.ZodString>;
    latencyMs: z.ZodNumber;
    fromCache: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    type: "kb_result";
    timestamp: number;
    intent: string;
    queryId: number;
    query: string;
    subQuestions: {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }[];
    latencyMs: number;
    fromCache: boolean;
    id?: string | undefined;
    answer?: string | undefined;
}, {
    type: "kb_result";
    timestamp: number;
    intent: string;
    queryId: number;
    query: string;
    subQuestions: {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }[];
    latencyMs: number;
    fromCache: boolean;
    id?: string | undefined;
    answer?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"system_message">;
    content: z.ZodString;
    level: z.ZodEnum<["info", "warning", "error"]>;
}, "strip", z.ZodTypeAny, {
    type: "system_message";
    timestamp: number;
    content: string;
    level: "error" | "info" | "warning";
    id?: string | undefined;
}, {
    type: "system_message";
    timestamp: number;
    content: string;
    level: "error" | "info" | "warning";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"banner">;
    version: z.ZodString;
    displayName: z.ZodString;
    email: z.ZodOptional<z.ZodString>;
    organization: z.ZodOptional<z.ZodString>;
    provider: z.ZodString;
    modelDisplay: z.ZodString;
    workingDir: z.ZodString;
    credits: z.ZodOptional<z.ZodString>;
    tips: z.ZodArray<z.ZodString, "many">;
}, "strip", z.ZodTypeAny, {
    type: "banner";
    timestamp: number;
    version: string;
    displayName: string;
    provider: string;
    modelDisplay: string;
    workingDir: string;
    tips: string[];
    id?: string | undefined;
    email?: string | undefined;
    organization?: string | undefined;
    credits?: string | undefined;
}, {
    type: "banner";
    timestamp: number;
    version: string;
    displayName: string;
    provider: string;
    modelDisplay: string;
    workingDir: string;
    tips: string[];
    id?: string | undefined;
    email?: string | undefined;
    organization?: string | undefined;
    credits?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"sidebar_update">;
    agents: z.ZodArray<z.ZodObject<{
        agentId: z.ZodString;
        agentType: z.ZodEnum<["Primary", "Explore", "Plan", "General"]>;
        taskSummary: z.ZodString;
        status: z.ZodEnum<["Pending", "Running", "Completed", "Failed"]>;
    }, "strip", z.ZodTypeAny, {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }, {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }>, "many">;
    files: z.ZodArray<z.ZodObject<{
        path: z.ZodString;
        action: z.ZodEnum<["read", "edited", "created"]>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        action: "read" | "edited" | "created";
    }, {
        path: string;
        action: "read" | "edited" | "created";
    }>, "many">;
    plans: z.ZodArray<z.ZodObject<{
        name: z.ZodString;
        status: z.ZodEnum<["todo", "in_progress", "done"]>;
    }, "strip", z.ZodTypeAny, {
        status: "todo" | "in_progress" | "done";
        name: string;
    }, {
        status: "todo" | "in_progress" | "done";
        name: string;
    }>, "many">;
    routing: z.ZodObject<{
        explore: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
        research: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
        code: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
        write: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
    }, "strip", z.ZodTypeAny, {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    }, {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    }>;
    activity: z.ZodObject<{
        backgroundTasks: z.ZodNumber;
        toolCallCount: z.ZodNumber;
        mcpServers: z.ZodNumber;
        creditBalance: z.ZodOptional<z.ZodString>;
    }, "strip", z.ZodTypeAny, {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    }, {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    }>;
}, "strip", z.ZodTypeAny, {
    type: "sidebar_update";
    timestamp: number;
    agents: {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }[];
    files: {
        path: string;
        action: "read" | "edited" | "created";
    }[];
    plans: {
        status: "todo" | "in_progress" | "done";
        name: string;
    }[];
    routing: {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    };
    activity: {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    };
    id?: string | undefined;
}, {
    type: "sidebar_update";
    timestamp: number;
    agents: {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }[];
    files: {
        path: string;
        action: "read" | "edited" | "created";
    }[];
    plans: {
        status: "todo" | "in_progress" | "done";
        name: string;
    }[];
    routing: {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    };
    activity: {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    };
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"model_info">;
    name: z.ZodString;
    provider: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "model_info";
    timestamp: number;
    name: string;
    provider: string;
    id?: string | undefined;
}, {
    type: "model_info";
    timestamp: number;
    name: string;
    provider: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"context_files_update">;
    files: z.ZodArray<z.ZodObject<{
        path: z.ZodString;
        action: z.ZodEnum<["added", "removed"]>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        action: "added" | "removed";
    }, {
        path: string;
        action: "added" | "removed";
    }>, "many">;
}, "strip", z.ZodTypeAny, {
    type: "context_files_update";
    timestamp: number;
    files: {
        path: string;
        action: "added" | "removed";
    }[];
    id?: string | undefined;
}, {
    type: "context_files_update";
    timestamp: number;
    files: {
        path: string;
        action: "added" | "removed";
    }[];
    id?: string | undefined;
}>]>;
export type EngineEvent = z.infer<typeof EngineEventSchema>;
export declare const SubmitPromptSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"submit_prompt">;
    text: z.ZodString;
    effortBudget: z.ZodOptional<z.ZodNumber>;
    modelOverride: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "submit_prompt";
    timestamp: number;
    text: string;
    id?: string | undefined;
    effortBudget?: number | undefined;
    modelOverride?: string | undefined;
}, {
    type: "submit_prompt";
    timestamp: number;
    text: string;
    id?: string | undefined;
    effortBudget?: number | undefined;
    modelOverride?: string | undefined;
}>;
export type SubmitPrompt = z.infer<typeof SubmitPromptSchema>;
export declare const RunInBackgroundSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"run_in_background">;
    text: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "run_in_background";
    timestamp: number;
    text: string;
    id?: string | undefined;
}, {
    type: "run_in_background";
    timestamp: number;
    text: string;
    id?: string | undefined;
}>;
export type RunInBackground = z.infer<typeof RunInBackgroundSchema>;
export declare const CancelAgentSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"cancel_agent">;
    agentId: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "cancel_agent";
    timestamp: number;
    id?: string | undefined;
    agentId?: string | undefined;
}, {
    type: "cancel_agent";
    timestamp: number;
    id?: string | undefined;
    agentId?: string | undefined;
}>;
export type CancelAgent = z.infer<typeof CancelAgentSchema>;
export declare const ResolvePermissionSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"resolve_permission">;
    requestId: z.ZodString;
    decision: z.ZodEnum<["allow", "deny"]>;
}, "strip", z.ZodTypeAny, {
    type: "resolve_permission";
    timestamp: number;
    requestId: string;
    decision: "allow" | "deny";
    id?: string | undefined;
}, {
    type: "resolve_permission";
    timestamp: number;
    requestId: string;
    decision: "allow" | "deny";
    id?: string | undefined;
}>;
export type ResolvePermission = z.infer<typeof ResolvePermissionSchema>;
export declare const ResolveAskUserSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"resolve_ask_user">;
    requestId: z.ZodString;
    answer: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "resolve_ask_user";
    timestamp: number;
    requestId: string;
    answer: string;
    id?: string | undefined;
}, {
    type: "resolve_ask_user";
    timestamp: number;
    requestId: string;
    answer: string;
    id?: string | undefined;
}>;
export type ResolveAskUser = z.infer<typeof ResolveAskUserSchema>;
export declare const KbFeedbackSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"kb_feedback">;
    queryId: z.ZodNumber;
    rating: z.ZodEnum<["positive", "negative", "corrected"]>;
    comment: z.ZodOptional<z.ZodString>;
    correction: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "kb_feedback";
    timestamp: number;
    queryId: number;
    rating: "positive" | "negative" | "corrected";
    id?: string | undefined;
    comment?: string | undefined;
    correction?: string | undefined;
}, {
    type: "kb_feedback";
    timestamp: number;
    queryId: number;
    rating: "positive" | "negative" | "corrected";
    id?: string | undefined;
    comment?: string | undefined;
    correction?: string | undefined;
}>;
export type KbFeedback = z.infer<typeof KbFeedbackSchema>;
export declare const ChangePermissionModeSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"change_permission_mode">;
    mode: z.ZodEnum<["prompt", "read-only", "workspace-write", "danger-full-access"]>;
}, "strip", z.ZodTypeAny, {
    type: "change_permission_mode";
    timestamp: number;
    mode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
}, {
    type: "change_permission_mode";
    timestamp: number;
    mode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
}>;
export type ChangePermissionMode = z.infer<typeof ChangePermissionModeSchema>;
export declare const ToggleContextFileSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"toggle_context_file">;
    path: z.ZodString;
    action: z.ZodEnum<["add", "remove"]>;
}, "strip", z.ZodTypeAny, {
    type: "toggle_context_file";
    timestamp: number;
    path: string;
    action: "add" | "remove";
    id?: string | undefined;
}, {
    type: "toggle_context_file";
    timestamp: number;
    path: string;
    action: "add" | "remove";
    id?: string | undefined;
}>;
export type ToggleContextFile = z.infer<typeof ToggleContextFileSchema>;
export declare const ChangeRoutingSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"change_routing">;
    category: z.ZodEnum<["explore", "research", "code", "write"]>;
    tier: z.ZodEnum<["fast", "balanced", "capable"]>;
}, "strip", z.ZodTypeAny, {
    type: "change_routing";
    timestamp: number;
    tier: "fast" | "balanced" | "capable";
    category: "code" | "explore" | "research" | "write";
    id?: string | undefined;
}, {
    type: "change_routing";
    timestamp: number;
    tier: "fast" | "balanced" | "capable";
    category: "code" | "explore" | "research" | "write";
    id?: string | undefined;
}>;
export type ChangeRouting = z.infer<typeof ChangeRoutingSchema>;
export declare const ClearChatSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"clear_chat">;
}, "strip", z.ZodTypeAny, {
    type: "clear_chat";
    timestamp: number;
    id?: string | undefined;
}, {
    type: "clear_chat";
    timestamp: number;
    id?: string | undefined;
}>;
export type ClearChat = z.infer<typeof ClearChatSchema>;
export declare const SlashCommandSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"slash_command">;
    command: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "slash_command";
    timestamp: number;
    command: string;
    id?: string | undefined;
}, {
    type: "slash_command";
    timestamp: number;
    command: string;
    id?: string | undefined;
}>;
export type SlashCommand = z.infer<typeof SlashCommandSchema>;
export declare const UpdateModelSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"update_model">;
    model: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "update_model";
    timestamp: number;
    model: string;
    id?: string | undefined;
}, {
    type: "update_model";
    timestamp: number;
    model: string;
    id?: string | undefined;
}>;
export type UpdateModel = z.infer<typeof UpdateModelSchema>;
export declare const MoeDispatchSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"moe_dispatch">;
    commands: z.ZodArray<z.ZodString, "many">;
}, "strip", z.ZodTypeAny, {
    type: "moe_dispatch";
    timestamp: number;
    commands: string[];
    id?: string | undefined;
}, {
    type: "moe_dispatch";
    timestamp: number;
    commands: string[];
    id?: string | undefined;
}>;
export type MoeDispatch = z.infer<typeof MoeDispatchSchema>;
export declare const InjectSkillSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"inject_skill">;
    command: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "inject_skill";
    timestamp: number;
    command: string;
    id?: string | undefined;
}, {
    type: "inject_skill";
    timestamp: number;
    command: string;
    id?: string | undefined;
}>;
export type InjectSkill = z.infer<typeof InjectSkillSchema>;
export declare const QuitSchema: z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"quit">;
}, "strip", z.ZodTypeAny, {
    type: "quit";
    timestamp: number;
    id?: string | undefined;
}, {
    type: "quit";
    timestamp: number;
    id?: string | undefined;
}>;
export type Quit = z.infer<typeof QuitSchema>;
export declare const TuiActionSchema: z.ZodDiscriminatedUnion<"type", [z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"submit_prompt">;
    text: z.ZodString;
    effortBudget: z.ZodOptional<z.ZodNumber>;
    modelOverride: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "submit_prompt";
    timestamp: number;
    text: string;
    id?: string | undefined;
    effortBudget?: number | undefined;
    modelOverride?: string | undefined;
}, {
    type: "submit_prompt";
    timestamp: number;
    text: string;
    id?: string | undefined;
    effortBudget?: number | undefined;
    modelOverride?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"run_in_background">;
    text: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "run_in_background";
    timestamp: number;
    text: string;
    id?: string | undefined;
}, {
    type: "run_in_background";
    timestamp: number;
    text: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"cancel_agent">;
    agentId: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "cancel_agent";
    timestamp: number;
    id?: string | undefined;
    agentId?: string | undefined;
}, {
    type: "cancel_agent";
    timestamp: number;
    id?: string | undefined;
    agentId?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"resolve_permission">;
    requestId: z.ZodString;
    decision: z.ZodEnum<["allow", "deny"]>;
}, "strip", z.ZodTypeAny, {
    type: "resolve_permission";
    timestamp: number;
    requestId: string;
    decision: "allow" | "deny";
    id?: string | undefined;
}, {
    type: "resolve_permission";
    timestamp: number;
    requestId: string;
    decision: "allow" | "deny";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"resolve_ask_user">;
    requestId: z.ZodString;
    answer: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "resolve_ask_user";
    timestamp: number;
    requestId: string;
    answer: string;
    id?: string | undefined;
}, {
    type: "resolve_ask_user";
    timestamp: number;
    requestId: string;
    answer: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"kb_feedback">;
    queryId: z.ZodNumber;
    rating: z.ZodEnum<["positive", "negative", "corrected"]>;
    comment: z.ZodOptional<z.ZodString>;
    correction: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "kb_feedback";
    timestamp: number;
    queryId: number;
    rating: "positive" | "negative" | "corrected";
    id?: string | undefined;
    comment?: string | undefined;
    correction?: string | undefined;
}, {
    type: "kb_feedback";
    timestamp: number;
    queryId: number;
    rating: "positive" | "negative" | "corrected";
    id?: string | undefined;
    comment?: string | undefined;
    correction?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"change_permission_mode">;
    mode: z.ZodEnum<["prompt", "read-only", "workspace-write", "danger-full-access"]>;
}, "strip", z.ZodTypeAny, {
    type: "change_permission_mode";
    timestamp: number;
    mode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
}, {
    type: "change_permission_mode";
    timestamp: number;
    mode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"toggle_context_file">;
    path: z.ZodString;
    action: z.ZodEnum<["add", "remove"]>;
}, "strip", z.ZodTypeAny, {
    type: "toggle_context_file";
    timestamp: number;
    path: string;
    action: "add" | "remove";
    id?: string | undefined;
}, {
    type: "toggle_context_file";
    timestamp: number;
    path: string;
    action: "add" | "remove";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"change_routing">;
    category: z.ZodEnum<["explore", "research", "code", "write"]>;
    tier: z.ZodEnum<["fast", "balanced", "capable"]>;
}, "strip", z.ZodTypeAny, {
    type: "change_routing";
    timestamp: number;
    tier: "fast" | "balanced" | "capable";
    category: "code" | "explore" | "research" | "write";
    id?: string | undefined;
}, {
    type: "change_routing";
    timestamp: number;
    tier: "fast" | "balanced" | "capable";
    category: "code" | "explore" | "research" | "write";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"clear_chat">;
}, "strip", z.ZodTypeAny, {
    type: "clear_chat";
    timestamp: number;
    id?: string | undefined;
}, {
    type: "clear_chat";
    timestamp: number;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"slash_command">;
    command: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "slash_command";
    timestamp: number;
    command: string;
    id?: string | undefined;
}, {
    type: "slash_command";
    timestamp: number;
    command: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"update_model">;
    model: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "update_model";
    timestamp: number;
    model: string;
    id?: string | undefined;
}, {
    type: "update_model";
    timestamp: number;
    model: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"moe_dispatch">;
    commands: z.ZodArray<z.ZodString, "many">;
}, "strip", z.ZodTypeAny, {
    type: "moe_dispatch";
    timestamp: number;
    commands: string[];
    id?: string | undefined;
}, {
    type: "moe_dispatch";
    timestamp: number;
    commands: string[];
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"inject_skill">;
    command: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "inject_skill";
    timestamp: number;
    command: string;
    id?: string | undefined;
}, {
    type: "inject_skill";
    timestamp: number;
    command: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"quit">;
}, "strip", z.ZodTypeAny, {
    type: "quit";
    timestamp: number;
    id?: string | undefined;
}, {
    type: "quit";
    timestamp: number;
    id?: string | undefined;
}>]>;
export type TuiAction = z.infer<typeof TuiActionSchema>;
export declare const AnyMessageSchema: z.ZodUnion<[z.ZodDiscriminatedUnion<"type", [z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"stream_delta">;
    agentId: z.ZodString;
    content: z.ZodString;
    done: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    type: "stream_delta";
    timestamp: number;
    agentId: string;
    done: boolean;
    content: string;
    id?: string | undefined;
}, {
    type: "stream_delta";
    timestamp: number;
    agentId: string;
    done: boolean;
    content: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"stream_end">;
    agentId: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "stream_end";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}, {
    type: "stream_end";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"tool_call_start">;
    agentId: z.ZodString;
    toolId: z.ZodString;
    toolName: z.ZodString;
    inputPreview: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "tool_call_start";
    timestamp: number;
    agentId: string;
    toolId: string;
    toolName: string;
    inputPreview: string;
    id?: string | undefined;
}, {
    type: "tool_call_start";
    timestamp: number;
    agentId: string;
    toolId: string;
    toolName: string;
    inputPreview: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"tool_call_update">;
    agentId: z.ZodString;
    toolId: z.ZodString;
    output: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "tool_call_update";
    timestamp: number;
    agentId: string;
    toolId: string;
    output: string;
    id?: string | undefined;
}, {
    type: "tool_call_update";
    timestamp: number;
    agentId: string;
    toolId: string;
    output: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"tool_call_complete">;
    agentId: z.ZodString;
    toolId: z.ZodString;
    status: z.ZodEnum<["completed", "failed"]>;
    output: z.ZodString;
    durationMs: z.ZodNumber;
    diff: z.ZodOptional<z.ZodObject<{
        filePath: z.ZodString;
        added: z.ZodNumber;
        removed: z.ZodNumber;
        hunks: z.ZodArray<z.ZodObject<{
            oldStart: z.ZodNumber;
            newStart: z.ZodNumber;
            lines: z.ZodArray<z.ZodDiscriminatedUnion<"kind", [z.ZodObject<{
                kind: z.ZodLiteral<"context">;
                text: z.ZodString;
            }, "strip", z.ZodTypeAny, {
                kind: "context";
                text: string;
            }, {
                kind: "context";
                text: string;
            }>, z.ZodObject<{
                kind: z.ZodLiteral<"added">;
                text: z.ZodString;
            }, "strip", z.ZodTypeAny, {
                kind: "added";
                text: string;
            }, {
                kind: "added";
                text: string;
            }>, z.ZodObject<{
                kind: z.ZodLiteral<"removed">;
                text: z.ZodString;
            }, "strip", z.ZodTypeAny, {
                kind: "removed";
                text: string;
            }, {
                kind: "removed";
                text: string;
            }>]>, "many">;
        }, "strip", z.ZodTypeAny, {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }, {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }>, "many">;
    }, "strip", z.ZodTypeAny, {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    }, {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    }>>;
}, "strip", z.ZodTypeAny, {
    type: "tool_call_complete";
    timestamp: number;
    status: "completed" | "failed";
    agentId: string;
    toolId: string;
    output: string;
    durationMs: number;
    id?: string | undefined;
    diff?: {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    } | undefined;
}, {
    type: "tool_call_complete";
    timestamp: number;
    status: "completed" | "failed";
    agentId: string;
    toolId: string;
    output: string;
    durationMs: number;
    id?: string | undefined;
    diff?: {
        added: number;
        removed: number;
        filePath: string;
        hunks: {
            oldStart: number;
            newStart: number;
            lines: ({
                kind: "context";
                text: string;
            } | {
                kind: "added";
                text: string;
            } | {
                kind: "removed";
                text: string;
            })[];
        }[];
    } | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"permission_request">;
    requestId: z.ZodString;
    agentId: z.ZodString;
    toolName: z.ZodString;
    toolInput: z.ZodString;
    requiredMode: z.ZodEnum<["prompt", "read-only", "workspace-write", "danger-full-access"]>;
    filePath: z.ZodOptional<z.ZodString>;
    description: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "permission_request";
    timestamp: number;
    agentId: string;
    toolName: string;
    requestId: string;
    toolInput: string;
    requiredMode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
    filePath?: string | undefined;
    description?: string | undefined;
}, {
    type: "permission_request";
    timestamp: number;
    agentId: string;
    toolName: string;
    requestId: string;
    toolInput: string;
    requiredMode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
    filePath?: string | undefined;
    description?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"ask_user_request">;
    requestId: z.ZodString;
    agentId: z.ZodString;
    question: z.ZodString;
    options: z.ZodOptional<z.ZodArray<z.ZodString, "many">>;
    defaultValue: z.ZodOptional<z.ZodString>;
    allowFreeText: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    type: "ask_user_request";
    timestamp: number;
    agentId: string;
    requestId: string;
    question: string;
    allowFreeText: boolean;
    id?: string | undefined;
    options?: string[] | undefined;
    defaultValue?: string | undefined;
}, {
    type: "ask_user_request";
    timestamp: number;
    agentId: string;
    requestId: string;
    question: string;
    allowFreeText: boolean;
    id?: string | undefined;
    options?: string[] | undefined;
    defaultValue?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"status_update">;
    phase: z.ZodEnum<["idle", "thinking", "reading_file", "editing_file", "running_bash", "searching", "done", "error"]>;
    label: z.ZodOptional<z.ZodString>;
    elapsedMs: z.ZodNumber;
    tokensRemaining: z.ZodOptional<z.ZodNumber>;
}, "strip", z.ZodTypeAny, {
    type: "status_update";
    timestamp: number;
    phase: "error" | "done" | "idle" | "thinking" | "reading_file" | "editing_file" | "running_bash" | "searching";
    elapsedMs: number;
    id?: string | undefined;
    label?: string | undefined;
    tokensRemaining?: number | undefined;
}, {
    type: "status_update";
    timestamp: number;
    phase: "error" | "done" | "idle" | "thinking" | "reading_file" | "editing_file" | "running_bash" | "searching";
    elapsedMs: number;
    id?: string | undefined;
    label?: string | undefined;
    tokensRemaining?: number | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_spawned">;
    agentId: z.ZodString;
    parentId: z.ZodOptional<z.ZodString>;
    agentType: z.ZodEnum<["Primary", "Explore", "Plan", "General"]>;
    task: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "agent_spawned";
    timestamp: number;
    agentId: string;
    agentType: "Primary" | "Explore" | "Plan" | "General";
    task: string;
    id?: string | undefined;
    parentId?: string | undefined;
}, {
    type: "agent_spawned";
    timestamp: number;
    agentId: string;
    agentType: "Primary" | "Explore" | "Plan" | "General";
    task: string;
    id?: string | undefined;
    parentId?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_status_changed">;
    agentId: z.ZodString;
    status: z.ZodEnum<["Pending", "Running", "Completed", "Failed"]>;
}, "strip", z.ZodTypeAny, {
    type: "agent_status_changed";
    timestamp: number;
    status: "Pending" | "Running" | "Completed" | "Failed";
    agentId: string;
    id?: string | undefined;
}, {
    type: "agent_status_changed";
    timestamp: number;
    status: "Pending" | "Running" | "Completed" | "Failed";
    agentId: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_completed">;
    agentId: z.ZodString;
    result: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "agent_completed";
    timestamp: number;
    agentId: string;
    result: string;
    id?: string | undefined;
}, {
    type: "agent_completed";
    timestamp: number;
    agentId: string;
    result: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"agent_failed">;
    agentId: z.ZodString;
    error: z.ZodString;
}, "strip", z.ZodTypeAny, {
    error: string;
    type: "agent_failed";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}, {
    error: string;
    type: "agent_failed";
    timestamp: number;
    agentId: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"usage_update">;
    agentId: z.ZodString;
    inputTokens: z.ZodNumber;
    outputTokens: z.ZodNumber;
}, "strip", z.ZodTypeAny, {
    type: "usage_update";
    timestamp: number;
    agentId: string;
    inputTokens: number;
    outputTokens: number;
    id?: string | undefined;
}, {
    type: "usage_update";
    timestamp: number;
    agentId: string;
    inputTokens: number;
    outputTokens: number;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"kb_result">;
    queryId: z.ZodNumber;
    query: z.ZodString;
    intent: z.ZodString;
    subQuestions: z.ZodArray<z.ZodObject<{
        subQuestion: z.ZodString;
        intent: z.ZodString;
        results: z.ZodArray<z.ZodObject<{
            chunkId: z.ZodString;
            text: z.ZodString;
            snippet: z.ZodString;
            score: z.ZodNumber;
            categoryLabel: z.ZodString;
            contentType: z.ZodString;
            citationLabel: z.ZodString;
            hasTimestamps: z.ZodBoolean;
            graphExpanded: z.ZodBoolean;
        }, "strip", z.ZodTypeAny, {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }, {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }>, "many">;
    }, "strip", z.ZodTypeAny, {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }, {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }>, "many">;
    answer: z.ZodOptional<z.ZodString>;
    latencyMs: z.ZodNumber;
    fromCache: z.ZodBoolean;
}, "strip", z.ZodTypeAny, {
    type: "kb_result";
    timestamp: number;
    intent: string;
    queryId: number;
    query: string;
    subQuestions: {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }[];
    latencyMs: number;
    fromCache: boolean;
    id?: string | undefined;
    answer?: string | undefined;
}, {
    type: "kb_result";
    timestamp: number;
    intent: string;
    queryId: number;
    query: string;
    subQuestions: {
        subQuestion: string;
        intent: string;
        results: {
            text: string;
            chunkId: string;
            snippet: string;
            score: number;
            categoryLabel: string;
            contentType: string;
            citationLabel: string;
            hasTimestamps: boolean;
            graphExpanded: boolean;
        }[];
    }[];
    latencyMs: number;
    fromCache: boolean;
    id?: string | undefined;
    answer?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"system_message">;
    content: z.ZodString;
    level: z.ZodEnum<["info", "warning", "error"]>;
}, "strip", z.ZodTypeAny, {
    type: "system_message";
    timestamp: number;
    content: string;
    level: "error" | "info" | "warning";
    id?: string | undefined;
}, {
    type: "system_message";
    timestamp: number;
    content: string;
    level: "error" | "info" | "warning";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"banner">;
    version: z.ZodString;
    displayName: z.ZodString;
    email: z.ZodOptional<z.ZodString>;
    organization: z.ZodOptional<z.ZodString>;
    provider: z.ZodString;
    modelDisplay: z.ZodString;
    workingDir: z.ZodString;
    credits: z.ZodOptional<z.ZodString>;
    tips: z.ZodArray<z.ZodString, "many">;
}, "strip", z.ZodTypeAny, {
    type: "banner";
    timestamp: number;
    version: string;
    displayName: string;
    provider: string;
    modelDisplay: string;
    workingDir: string;
    tips: string[];
    id?: string | undefined;
    email?: string | undefined;
    organization?: string | undefined;
    credits?: string | undefined;
}, {
    type: "banner";
    timestamp: number;
    version: string;
    displayName: string;
    provider: string;
    modelDisplay: string;
    workingDir: string;
    tips: string[];
    id?: string | undefined;
    email?: string | undefined;
    organization?: string | undefined;
    credits?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"sidebar_update">;
    agents: z.ZodArray<z.ZodObject<{
        agentId: z.ZodString;
        agentType: z.ZodEnum<["Primary", "Explore", "Plan", "General"]>;
        taskSummary: z.ZodString;
        status: z.ZodEnum<["Pending", "Running", "Completed", "Failed"]>;
    }, "strip", z.ZodTypeAny, {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }, {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }>, "many">;
    files: z.ZodArray<z.ZodObject<{
        path: z.ZodString;
        action: z.ZodEnum<["read", "edited", "created"]>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        action: "read" | "edited" | "created";
    }, {
        path: string;
        action: "read" | "edited" | "created";
    }>, "many">;
    plans: z.ZodArray<z.ZodObject<{
        name: z.ZodString;
        status: z.ZodEnum<["todo", "in_progress", "done"]>;
    }, "strip", z.ZodTypeAny, {
        status: "todo" | "in_progress" | "done";
        name: string;
    }, {
        status: "todo" | "in_progress" | "done";
        name: string;
    }>, "many">;
    routing: z.ZodObject<{
        explore: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
        research: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
        code: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
        write: z.ZodObject<{
            model: z.ZodString;
            tier: z.ZodString;
        }, "strip", z.ZodTypeAny, {
            model: string;
            tier: string;
        }, {
            model: string;
            tier: string;
        }>;
    }, "strip", z.ZodTypeAny, {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    }, {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    }>;
    activity: z.ZodObject<{
        backgroundTasks: z.ZodNumber;
        toolCallCount: z.ZodNumber;
        mcpServers: z.ZodNumber;
        creditBalance: z.ZodOptional<z.ZodString>;
    }, "strip", z.ZodTypeAny, {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    }, {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    }>;
}, "strip", z.ZodTypeAny, {
    type: "sidebar_update";
    timestamp: number;
    agents: {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }[];
    files: {
        path: string;
        action: "read" | "edited" | "created";
    }[];
    plans: {
        status: "todo" | "in_progress" | "done";
        name: string;
    }[];
    routing: {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    };
    activity: {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    };
    id?: string | undefined;
}, {
    type: "sidebar_update";
    timestamp: number;
    agents: {
        status: "Pending" | "Running" | "Completed" | "Failed";
        agentId: string;
        agentType: "Primary" | "Explore" | "Plan" | "General";
        taskSummary: string;
    }[];
    files: {
        path: string;
        action: "read" | "edited" | "created";
    }[];
    plans: {
        status: "todo" | "in_progress" | "done";
        name: string;
    }[];
    routing: {
        code: {
            model: string;
            tier: string;
        };
        explore: {
            model: string;
            tier: string;
        };
        research: {
            model: string;
            tier: string;
        };
        write: {
            model: string;
            tier: string;
        };
    };
    activity: {
        backgroundTasks: number;
        toolCallCount: number;
        mcpServers: number;
        creditBalance?: string | undefined;
    };
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"model_info">;
    name: z.ZodString;
    provider: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "model_info";
    timestamp: number;
    name: string;
    provider: string;
    id?: string | undefined;
}, {
    type: "model_info";
    timestamp: number;
    name: string;
    provider: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"context_files_update">;
    files: z.ZodArray<z.ZodObject<{
        path: z.ZodString;
        action: z.ZodEnum<["added", "removed"]>;
    }, "strip", z.ZodTypeAny, {
        path: string;
        action: "added" | "removed";
    }, {
        path: string;
        action: "added" | "removed";
    }>, "many">;
}, "strip", z.ZodTypeAny, {
    type: "context_files_update";
    timestamp: number;
    files: {
        path: string;
        action: "added" | "removed";
    }[];
    id?: string | undefined;
}, {
    type: "context_files_update";
    timestamp: number;
    files: {
        path: string;
        action: "added" | "removed";
    }[];
    id?: string | undefined;
}>]>, z.ZodDiscriminatedUnion<"type", [z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"submit_prompt">;
    text: z.ZodString;
    effortBudget: z.ZodOptional<z.ZodNumber>;
    modelOverride: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "submit_prompt";
    timestamp: number;
    text: string;
    id?: string | undefined;
    effortBudget?: number | undefined;
    modelOverride?: string | undefined;
}, {
    type: "submit_prompt";
    timestamp: number;
    text: string;
    id?: string | undefined;
    effortBudget?: number | undefined;
    modelOverride?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"run_in_background">;
    text: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "run_in_background";
    timestamp: number;
    text: string;
    id?: string | undefined;
}, {
    type: "run_in_background";
    timestamp: number;
    text: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"cancel_agent">;
    agentId: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "cancel_agent";
    timestamp: number;
    id?: string | undefined;
    agentId?: string | undefined;
}, {
    type: "cancel_agent";
    timestamp: number;
    id?: string | undefined;
    agentId?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"resolve_permission">;
    requestId: z.ZodString;
    decision: z.ZodEnum<["allow", "deny"]>;
}, "strip", z.ZodTypeAny, {
    type: "resolve_permission";
    timestamp: number;
    requestId: string;
    decision: "allow" | "deny";
    id?: string | undefined;
}, {
    type: "resolve_permission";
    timestamp: number;
    requestId: string;
    decision: "allow" | "deny";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"resolve_ask_user">;
    requestId: z.ZodString;
    answer: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "resolve_ask_user";
    timestamp: number;
    requestId: string;
    answer: string;
    id?: string | undefined;
}, {
    type: "resolve_ask_user";
    timestamp: number;
    requestId: string;
    answer: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"kb_feedback">;
    queryId: z.ZodNumber;
    rating: z.ZodEnum<["positive", "negative", "corrected"]>;
    comment: z.ZodOptional<z.ZodString>;
    correction: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    type: "kb_feedback";
    timestamp: number;
    queryId: number;
    rating: "positive" | "negative" | "corrected";
    id?: string | undefined;
    comment?: string | undefined;
    correction?: string | undefined;
}, {
    type: "kb_feedback";
    timestamp: number;
    queryId: number;
    rating: "positive" | "negative" | "corrected";
    id?: string | undefined;
    comment?: string | undefined;
    correction?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"change_permission_mode">;
    mode: z.ZodEnum<["prompt", "read-only", "workspace-write", "danger-full-access"]>;
}, "strip", z.ZodTypeAny, {
    type: "change_permission_mode";
    timestamp: number;
    mode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
}, {
    type: "change_permission_mode";
    timestamp: number;
    mode: "prompt" | "read-only" | "workspace-write" | "danger-full-access";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"toggle_context_file">;
    path: z.ZodString;
    action: z.ZodEnum<["add", "remove"]>;
}, "strip", z.ZodTypeAny, {
    type: "toggle_context_file";
    timestamp: number;
    path: string;
    action: "add" | "remove";
    id?: string | undefined;
}, {
    type: "toggle_context_file";
    timestamp: number;
    path: string;
    action: "add" | "remove";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"change_routing">;
    category: z.ZodEnum<["explore", "research", "code", "write"]>;
    tier: z.ZodEnum<["fast", "balanced", "capable"]>;
}, "strip", z.ZodTypeAny, {
    type: "change_routing";
    timestamp: number;
    tier: "fast" | "balanced" | "capable";
    category: "code" | "explore" | "research" | "write";
    id?: string | undefined;
}, {
    type: "change_routing";
    timestamp: number;
    tier: "fast" | "balanced" | "capable";
    category: "code" | "explore" | "research" | "write";
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"clear_chat">;
}, "strip", z.ZodTypeAny, {
    type: "clear_chat";
    timestamp: number;
    id?: string | undefined;
}, {
    type: "clear_chat";
    timestamp: number;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"slash_command">;
    command: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "slash_command";
    timestamp: number;
    command: string;
    id?: string | undefined;
}, {
    type: "slash_command";
    timestamp: number;
    command: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"update_model">;
    model: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "update_model";
    timestamp: number;
    model: string;
    id?: string | undefined;
}, {
    type: "update_model";
    timestamp: number;
    model: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"moe_dispatch">;
    commands: z.ZodArray<z.ZodString, "many">;
}, "strip", z.ZodTypeAny, {
    type: "moe_dispatch";
    timestamp: number;
    commands: string[];
    id?: string | undefined;
}, {
    type: "moe_dispatch";
    timestamp: number;
    commands: string[];
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"inject_skill">;
    command: z.ZodString;
}, "strip", z.ZodTypeAny, {
    type: "inject_skill";
    timestamp: number;
    command: string;
    id?: string | undefined;
}, {
    type: "inject_skill";
    timestamp: number;
    command: string;
    id?: string | undefined;
}>, z.ZodObject<{
    id: z.ZodOptional<z.ZodString>;
    timestamp: z.ZodNumber;
} & {
    type: z.ZodLiteral<"quit">;
}, "strip", z.ZodTypeAny, {
    type: "quit";
    timestamp: number;
    id?: string | undefined;
}, {
    type: "quit";
    timestamp: number;
    id?: string | undefined;
}>]>]>;
export type AnyMessage = z.infer<typeof AnyMessageSchema>;
