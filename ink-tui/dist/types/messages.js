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
import { messageSchema } from './protocol.js';
// ═══════════════════════════════════════════════════════════════════════════
// Shared / reusable schemas
// ═══════════════════════════════════════════════════════════════════════════
// -- Agent types (mirrors Rust `events::AgentType`) --
export const AgentTypeSchema = z.enum(['Primary', 'Explore', 'Plan', 'General']);
// -- Agent status (mirrors Rust `events::AgentStatus`) --
export const AgentStatusSchema = z.enum(['Pending', 'Running', 'Completed', 'Failed']);
// -- Diff structures (mirrors Rust `events::DiffInfo`) --
export const DiffLineSchema = z.discriminatedUnion('kind', [
    z.object({ kind: z.literal('context'), text: z.string() }),
    z.object({ kind: z.literal('added'), text: z.string() }),
    z.object({ kind: z.literal('removed'), text: z.string() }),
]);
export const DiffHunkSchema = z.object({
    oldStart: z.number(),
    newStart: z.number(),
    lines: z.array(DiffLineSchema),
});
export const DiffInfoSchema = z.object({
    filePath: z.string(),
    added: z.number(),
    removed: z.number(),
    hunks: z.array(DiffHunkSchema),
});
// -- Knowledge base result entry --
export const KbChunkResultSchema = z.object({
    chunkId: z.string(),
    text: z.string(),
    snippet: z.string(),
    score: z.number().min(0).max(1),
    categoryLabel: z.string(),
    contentType: z.string(),
    citationLabel: z.string(),
    hasTimestamps: z.boolean(),
    graphExpanded: z.boolean(),
});
export const SubQuestionResultSchema = z.object({
    subQuestion: z.string(),
    intent: z.string(),
    results: z.array(KbChunkResultSchema),
});
// -- Agent info (sidebar) --
export const AgentInfoSchema = z.object({
    agentId: z.string(),
    agentType: AgentTypeSchema,
    taskSummary: z.string(),
    status: AgentStatusSchema,
});
// -- File info (sidebar) --
export const FileActionSchema = z.enum(['read', 'edited', 'created']);
export const FileInfoSchema = z.object({
    path: z.string(),
    action: FileActionSchema,
});
// -- Plan info (sidebar) --
export const PlanInfoSchema = z.object({
    name: z.string(),
    status: z.enum(['todo', 'in_progress', 'done']),
});
// -- Routing table (sidebar) --
export const ActionCategorySchema = z.enum(['explore', 'research', 'code', 'write']);
export const RoutingEntrySchema = z.object({
    model: z.string(),
    tier: z.string(),
});
export const RoutingTableSchema = z.object({
    explore: RoutingEntrySchema,
    research: RoutingEntrySchema,
    code: RoutingEntrySchema,
    write: RoutingEntrySchema,
});
// -- Activity info (sidebar) --
export const ActivityInfoSchema = z.object({
    backgroundTasks: z.number(),
    toolCallCount: z.number(),
    mcpServers: z.number(),
    creditBalance: z.string().optional(),
});
// -- Status phase (mirrors Rust `status_bar::AgentPhase`) --
export const AgentPhaseSchema = z.enum([
    'idle',
    'thinking',
    'reading_file',
    'editing_file',
    'running_bash',
    'searching',
    'done',
    'error',
]);
// -- Permission mode --
export const PermissionModeSchema = z.enum([
    'prompt',
    'read-only',
    'workspace-write',
    'danger-full-access',
]);
// -- System message level --
export const SystemLevelSchema = z.enum(['info', 'warning', 'error']);
// ═══════════════════════════════════════════════════════════════════════════
// Engine -> TUI events
// ═══════════════════════════════════════════════════════════════════════════
// 1. stream_delta — LLM response chunk
// NOTE: Rust sends { "text": "..." } not { "content": "..." }.
// Rust does NOT send a "done" field — the stream_end event signals completion.
export const StreamDeltaSchema = messageSchema('stream_delta', {
    agentId: z.string(),
    text: z.string(),
});
// 2. stream_end — assistant finished streaming
export const StreamEndSchema = messageSchema('stream_end', {
    agentId: z.string(),
});
// 3. tool_call_start — tool execution begins
// NOTE: Rust sends "callId" not "toolId".
export const ToolCallStartSchema = messageSchema('tool_call_start', {
    agentId: z.string(),
    callId: z.string(),
    toolName: z.string(),
    inputPreview: z.string(),
});
// 4. tool_call_update — tool progress (partial output)
// NOTE: This event does NOT exist in Rust yet. Kept for future use.
export const ToolCallUpdateSchema = messageSchema('tool_call_update', {
    agentId: z.string(),
    callId: z.string(),
    output: z.string(),
});
// 5. tool_call_end — tool finished
// NOTE: Rust sends "tool_call_end" not "tool_call_complete".
// Rust sends "callId" (not "toolId"), "isError" (bool, not "status" enum),
// and "duration" (ms number, not "durationMs").
export const ToolCallEndSchema = messageSchema('tool_call_end', {
    agentId: z.string(),
    callId: z.string(),
    output: z.string(),
    isError: z.boolean(),
    duration: z.number(),
    diff: DiffInfoSchema.optional().nullable(),
});
// Keep the old name as an alias for backward compatibility with mock mode
export const ToolCallCompleteSchema = ToolCallEndSchema;
// 6. permission_request — needs user approval
// NOTE: Rust sends "input" not "toolInput", and "requiredMode" as a free string.
// Rust does NOT send "filePath" or "description".
export const PermissionRequestSchema = messageSchema('permission_request', {
    requestId: z.string(),
    agentId: z.string(),
    toolName: z.string(),
    input: z.string(),
    requiredMode: z.string(),
});
// 7. ask_user_request — needs user choice or text input
// NOTE: Rust sends "default" not "defaultValue", and does NOT send "allowFreeText".
// Rust "options" is Option<Vec<String>> which serializes as null or array.
export const AskUserRequestSchema = messageSchema('ask_user_request', {
    requestId: z.string(),
    agentId: z.string(),
    question: z.string(),
    options: z.array(z.string()).optional().nullable(),
    default: z.string().optional().nullable(),
});
// 8. status_update — phase change
export const StatusUpdateSchema = messageSchema('status_update', {
    phase: AgentPhaseSchema,
    label: z.string().optional(),
    elapsedMs: z.number(),
    tokensRemaining: z.number().optional(),
});
// 9. agent_spawned — new agent created
export const AgentSpawnedSchema = messageSchema('agent_spawned', {
    agentId: z.string(),
    parentId: z.string().optional(),
    agentType: AgentTypeSchema,
    task: z.string(),
});
// 10. agent_status_changed — agent lifecycle update
export const AgentStatusChangedSchema = messageSchema('agent_status_changed', {
    agentId: z.string(),
    status: AgentStatusSchema,
});
// 11. agent_completed — agent finished successfully
export const AgentCompletedSchema = messageSchema('agent_completed', {
    agentId: z.string(),
    result: z.string(),
});
// 12. agent_failed — agent errored out
export const AgentFailedSchema = messageSchema('agent_failed', {
    agentId: z.string(),
    error: z.string(),
});
// 13. usage_update — token usage from a streaming response
export const UsageUpdateSchema = messageSchema('usage_update', {
    agentId: z.string(),
    inputTokens: z.number(),
    outputTokens: z.number(),
});
// 14. knowledge_result — knowledge base query result
// NOTE: Rust sends "knowledge_result" (from enum variant KnowledgeResult), not "kb_result".
export const KbResultSchema = messageSchema('knowledge_result', {
    queryId: z.number(),
    query: z.string(),
    intent: z.string(),
    subQuestions: z.array(SubQuestionResultSchema),
    answer: z.string().optional(),
    latencyMs: z.number(),
    fromCache: z.boolean(),
});
// 15. system_message — system or error notice
export const SystemMessageSchema = messageSchema('system_message', {
    content: z.string(),
    level: SystemLevelSchema,
});
// 16. banner — startup banner data
export const BannerSchema = messageSchema('banner', {
    version: z.string(),
    displayName: z.string(),
    email: z.string().optional(),
    organization: z.string().optional(),
    provider: z.string(),
    modelDisplay: z.string(),
    workingDir: z.string(),
    credits: z.string().optional(),
    tips: z.array(z.string()),
});
// 17. sidebar_update — sidebar data refresh
export const SidebarUpdateSchema = messageSchema('sidebar_update', {
    agents: z.array(AgentInfoSchema),
    files: z.array(FileInfoSchema),
    plans: z.array(PlanInfoSchema),
    routing: RoutingTableSchema,
    activity: ActivityInfoSchema,
});
// 18. model_info — current model details
export const ModelInfoSchema = messageSchema('model_info', {
    name: z.string(),
    provider: z.string(),
});
// 19. context_files_update — context file list changed
export const ContextFilesUpdateSchema = messageSchema('context_files_update', {
    files: z.array(z.object({
        path: z.string(),
        action: z.enum(['added', 'removed']),
    })),
});
// ═══════════════════════════════════════════════════════════════════════════
// Discriminated union of ALL engine events
// ═══════════════════════════════════════════════════════════════════════════
export const EngineEventSchema = z.discriminatedUnion('type', [
    StreamDeltaSchema,
    StreamEndSchema,
    ToolCallStartSchema,
    ToolCallUpdateSchema,
    ToolCallEndSchema,
    PermissionRequestSchema,
    AskUserRequestSchema,
    StatusUpdateSchema,
    AgentSpawnedSchema,
    AgentStatusChangedSchema,
    AgentCompletedSchema,
    AgentFailedSchema,
    UsageUpdateSchema,
    KbResultSchema,
    SystemMessageSchema,
    BannerSchema,
    SidebarUpdateSchema,
    ModelInfoSchema,
    ContextFilesUpdateSchema,
]);
// ═══════════════════════════════════════════════════════════════════════════
// TUI -> Engine actions
// ═══════════════════════════════════════════════════════════════════════════
// 1. submit_prompt — user sends a message
export const SubmitPromptSchema = messageSchema('submit_prompt', {
    text: z.string(),
    effortBudget: z.number().optional(),
    modelOverride: z.string().optional(),
});
// 2. run_in_background — submit async
export const RunInBackgroundSchema = messageSchema('run_in_background', {
    text: z.string(),
});
// 3. cancel_agent — stop running agent
export const CancelAgentSchema = messageSchema('cancel_agent', {
    agentId: z.string().optional(),
});
// 4. permission_response — answer permission dialog
// NOTE: Rust expects "permission_response" (not "resolve_permission"),
// with "requestId" and "allow" (boolean), not "decision" string.
export const ResolvePermissionSchema = messageSchema('permission_response', {
    requestId: z.string(),
    allow: z.boolean(),
});
// 5. ask_user_response — answer ask-user dialog
// NOTE: Rust expects "ask_user_response" (not "resolve_ask_user"),
// with "requestId" and "response" (not "answer").
export const ResolveAskUserSchema = messageSchema('ask_user_response', {
    requestId: z.string(),
    response: z.string(),
});
// 6. knowledge_feedback — rate KB result
// NOTE: Rust expects "knowledge_feedback" (not "kb_feedback"),
// and "comment"/"correction" are required strings in Rust.
export const KbFeedbackSchema = messageSchema('knowledge_feedback', {
    queryId: z.number(),
    rating: z.enum(['positive', 'negative', 'corrected']),
    comment: z.string(),
    correction: z.string(),
});
// 7. update_permissions — Ctrl+P mode cycle
// NOTE: Rust expects "update_permissions" (not "change_permission_mode"),
// and it's a newtype variant wrapping a String. Internally tagged serde
// cannot deserialize newtype(String) -- this is a KNOWN Rust-side limitation.
// For now we send the object form and note this needs a Rust-side fix.
export const ChangePermissionModeSchema = messageSchema('update_permissions', {
    mode: PermissionModeSchema,
});
// 8. toggle_context_file — add/remove context file
export const ToggleContextFileSchema = messageSchema('toggle_context_file', {
    path: z.string(),
    action: z.enum(['add', 'remove']),
});
// 9. change_routing — change model routing for a category
export const ChangeRoutingSchema = messageSchema('change_routing', {
    category: ActionCategorySchema,
    tier: z.enum(['fast', 'balanced', 'capable']),
});
// 10. clear_chat — Ctrl+L
export const ClearChatSchema = messageSchema('clear_chat', {});
// 11. slash_command — user issued a slash command
// NOTE: Rust Action::SlashCommand(String) is a newtype variant.
// Internally tagged serde cannot deserialize newtype(String).
// This is a KNOWN Rust-side limitation — needs Rust fix to use struct variant.
export const SlashCommandSchema = messageSchema('slash_command', {
    command: z.string(),
});
// 12. update_model — change default model
// NOTE: Rust Action::UpdateModel(String) is a newtype variant.
// Internally tagged serde cannot deserialize newtype(String).
// This is a KNOWN Rust-side limitation — needs Rust fix to use struct variant.
export const UpdateModelSchema = messageSchema('update_model', {
    model: z.string(),
});
// 13. moe_dispatch — parallel agent dispatch
export const MoeDispatchSchema = messageSchema('moe_dispatch', {
    commands: z.array(z.string()),
});
// 14. inject_skill — mid-task skill injection
export const InjectSkillSchema = messageSchema('inject_skill', {
    command: z.string(),
});
// 15. voice_transcribed — voice transcription completed
// NOTE: Rust Action::VoiceTranscribed { text } — handled in TUI event loop.
export const VoiceTranscribedSchema = messageSchema('voice_transcribed', {
    text: z.string(),
});
// 16. quit — user wants to exit
export const QuitSchema = messageSchema('quit', {});
// ═══════════════════════════════════════════════════════════════════════════
// Discriminated union of ALL TUI actions
// ═══════════════════════════════════════════════════════════════════════════
export const TuiActionSchema = z.discriminatedUnion('type', [
    SubmitPromptSchema,
    RunInBackgroundSchema,
    CancelAgentSchema,
    ResolvePermissionSchema,
    ResolveAskUserSchema,
    KbFeedbackSchema,
    ChangePermissionModeSchema,
    ToggleContextFileSchema,
    ChangeRoutingSchema,
    ClearChatSchema,
    SlashCommandSchema,
    UpdateModelSchema,
    MoeDispatchSchema,
    InjectSkillSchema,
    VoiceTranscribedSchema,
    QuitSchema,
]);
// ═══════════════════════════════════════════════════════════════════════════
// Any protocol message (either direction)
// ═══════════════════════════════════════════════════════════════════════════
export const AnyMessageSchema = z.union([EngineEventSchema, TuiActionSchema]);
//# sourceMappingURL=messages.js.map