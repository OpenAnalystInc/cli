//! Loop detection service — prevents agents from getting stuck in infinite loops.
//!
//! Three independent detection mechanisms:
//! 1. **Tool Call Loop** — detects N identical consecutive tool calls (hash-based)
//! 2. **Content Chanting** — detects repetitive text patterns in streaming content
//! 3. **Turn Budget** — caps total turns to prevent runaway agents

use std::collections::HashMap;

// ── Constants ────────────────────────────────────────────────────────────────

/// Consecutive identical tool calls to trigger detection.
const TOOL_CALL_LOOP_THRESHOLD: usize = 5;

/// Identical content chunks needed to trigger chanting detection.
const CONTENT_LOOP_THRESHOLD: usize = 10;

/// Character window for content chunk hashing.
const CONTENT_CHUNK_SIZE: usize = 50;

/// Maximum content history to track (prevents unbounded memory).
const MAX_HISTORY_LENGTH: usize = 5000;

/// Default max turns before forced stop (autonomous safety net).
const DEFAULT_MAX_TURNS: u32 = 200;

// ── Types ────────────────────────────────────────────────────────────────────

/// The type of loop detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopType {
    /// Same tool+args called N times in a row.
    ConsecutiveIdenticalToolCalls,
    /// Repetitive text pattern in streaming output.
    ContentChanting,
    /// Exceeded maximum turn budget.
    TurnBudgetExceeded,
}

/// Result of a loop detection check.
#[derive(Debug, Clone)]
pub struct LoopDetectionResult {
    /// Number of times a loop has been detected in this prompt.
    pub count: u32,
    /// Type of loop (if detected).
    pub loop_type: Option<LoopType>,
    /// Human-readable detail.
    pub detail: Option<String>,
}

impl LoopDetectionResult {
    /// No loop detected.
    pub fn none() -> Self {
        Self { count: 0, loop_type: None, detail: None }
    }

    /// A loop was detected.
    pub fn detected(count: u32, loop_type: LoopType, detail: String) -> Self {
        Self { count, loop_type: Some(loop_type), detail: Some(detail) }
    }

    /// Was a loop detected?
    pub fn is_loop(&self) -> bool {
        self.count > 0
    }
}

// ── Service ──────────────────────────────────────────────────────────────────

/// Loop detection service for agent orchestration.
///
/// Call `check_tool_call()` on each tool invocation and `check_content()` on
/// each streaming delta. Call `turn_started()` at the beginning of each LLM turn.
pub struct LoopDetectionService {
    // Tool call tracking
    last_tool_call_key: Option<u64>,
    tool_call_repetition_count: usize,

    // Content chanting tracking
    content_history: String,
    /// Map of chunk hash → list of positions where that chunk appeared.
    content_stats: HashMap<u64, Vec<usize>>,
    last_content_index: usize,
    in_code_block: bool,

    // State
    loop_detected: bool,
    detected_count: u32,
    last_detail: Option<String>,
    last_loop_type: Option<LoopType>,

    // Turn tracking
    turns_in_prompt: u32,
    max_turns: u32,

    // Session-level disable
    disabled: bool,
}

impl Default for LoopDetectionService {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_TURNS)
    }
}

impl LoopDetectionService {
    /// Create with a custom max turn budget.
    pub fn new(max_turns: u32) -> Self {
        Self {
            last_tool_call_key: None,
            tool_call_repetition_count: 0,
            content_history: String::new(),
            content_stats: HashMap::new(),
            last_content_index: 0,
            in_code_block: false,
            loop_detected: false,
            detected_count: 0,
            last_detail: None,
            last_loop_type: None,
            turns_in_prompt: 0,
            max_turns,
            disabled: false,
        }
    }

    /// Reset all state for a new user prompt.
    pub fn reset(&mut self) {
        self.last_tool_call_key = None;
        self.tool_call_repetition_count = 0;
        self.content_history.clear();
        self.content_stats.clear();
        self.last_content_index = 0;
        self.in_code_block = false;
        self.loop_detected = false;
        self.detected_count = 0;
        self.last_detail = None;
        self.last_loop_type = None;
        self.turns_in_prompt = 0;
    }

    /// Allow one recovery turn after a loop was detected.
    pub fn clear_detection(&mut self) {
        self.loop_detected = false;
    }

    /// Disable loop detection for the rest of the session.
    pub fn disable(&mut self) {
        self.disabled = true;
    }

    /// Check if a loop was already detected (cached result).
    pub fn current_result(&self) -> LoopDetectionResult {
        if self.loop_detected {
            LoopDetectionResult {
                count: self.detected_count,
                loop_type: self.last_loop_type,
                detail: self.last_detail.clone(),
            }
        } else {
            LoopDetectionResult::none()
        }
    }

    // ── Tool Call Loop Detection ─────────────────────────────────────────

    /// Check a tool call for loops. Call this on every `ToolCallStart` event.
    ///
    /// `tool_name` is the tool name, `args_json` is the JSON-serialized arguments.
    pub fn check_tool_call(&mut self, tool_name: &str, args_json: &str) -> LoopDetectionResult {
        if self.disabled { return LoopDetectionResult::none(); }
        if self.loop_detected { return self.current_result(); }

        // Reset content tracking when a tool call happens
        self.reset_content_tracking();

        let key = hash_tool_call(tool_name, args_json);

        if self.last_tool_call_key == Some(key) {
            self.tool_call_repetition_count += 1;
        } else {
            self.last_tool_call_key = Some(key);
            self.tool_call_repetition_count = 1;
        }

        if self.tool_call_repetition_count >= TOOL_CALL_LOOP_THRESHOLD {
            self.loop_detected = true;
            self.detected_count += 1;
            self.last_loop_type = Some(LoopType::ConsecutiveIdenticalToolCalls);
            self.last_detail = Some(format!(
                "Repeated tool call: {tool_name} ({} identical calls in a row)",
                self.tool_call_repetition_count
            ));
            return self.current_result();
        }

        LoopDetectionResult::none()
    }

    // ── Content Chanting Loop Detection ──────────────────────────────────

    /// Check streaming content for chanting loops. Call on each `StreamDelta`.
    pub fn check_content(&mut self, content: &str) -> LoopDetectionResult {
        if self.disabled { return LoopDetectionResult::none(); }
        if self.loop_detected { return self.current_result(); }

        // Detect markdown structures to avoid false positives
        let has_code_fence = content.contains("```");
        let has_table = content.contains('|') && content.contains("---");
        let has_list = content.starts_with("- ") || content.starts_with("* ") || content.starts_with("1. ");
        let has_heading = content.starts_with('#');

        if has_code_fence || has_table || has_list || has_heading {
            self.reset_content_tracking();
        }

        // Track code block state
        let was_in_code_block = self.in_code_block;
        if has_code_fence {
            self.in_code_block = !self.in_code_block;
        }

        // Skip analysis inside code blocks
        if was_in_code_block || self.in_code_block {
            return LoopDetectionResult::none();
        }

        self.content_history.push_str(content);
        self.truncate_history();

        if self.analyze_content_chunks() {
            self.loop_detected = true;
            self.detected_count += 1;
            self.last_loop_type = Some(LoopType::ContentChanting);
            let start = self.last_content_index.saturating_sub(20);
            let end = (self.last_content_index + CONTENT_CHUNK_SIZE).min(self.content_history.len());
            let snippet: String = self.content_history.chars().skip(start).take(end - start).collect();
            self.last_detail = Some(format!("Repeating content detected: \"{snippet}...\""));
            return self.current_result();
        }

        LoopDetectionResult::none()
    }

    // ── Turn Budget Detection ────────────────────────────────────────────

    /// Call at the start of each LLM turn.
    pub fn turn_started(&mut self) -> LoopDetectionResult {
        if self.disabled { return LoopDetectionResult::none(); }
        if self.loop_detected { return self.current_result(); }

        self.turns_in_prompt += 1;

        if self.turns_in_prompt >= self.max_turns {
            self.loop_detected = true;
            self.detected_count += 1;
            self.last_loop_type = Some(LoopType::TurnBudgetExceeded);
            self.last_detail = Some(format!(
                "Turn budget exceeded: {} turns (max {})",
                self.turns_in_prompt, self.max_turns
            ));
            return self.current_result();
        }

        LoopDetectionResult::none()
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    fn reset_content_tracking(&mut self) {
        self.content_history.clear();
        self.content_stats.clear();
        self.last_content_index = 0;
    }

    fn truncate_history(&mut self) {
        if self.content_history.len() <= MAX_HISTORY_LENGTH {
            return;
        }
        let truncation = self.content_history.len() - MAX_HISTORY_LENGTH;
        // Safe truncation at char boundary
        let boundary = self.content_history.char_indices()
            .find(|(i, _)| *i >= truncation)
            .map(|(i, _)| i)
            .unwrap_or(truncation);

        self.content_history = self.content_history[boundary..].to_string();
        self.last_content_index = self.last_content_index.saturating_sub(boundary);

        // Adjust all tracked indices
        let mut to_remove = Vec::new();
        for (hash, indices) in self.content_stats.iter_mut() {
            indices.retain(|idx| *idx >= boundary);
            for idx in indices.iter_mut() {
                *idx -= boundary;
            }
            if indices.is_empty() {
                to_remove.push(*hash);
            }
        }
        for hash in to_remove {
            self.content_stats.remove(&hash);
        }
    }

    /// Sliding window analysis: hash 50-char chunks, detect repetition.
    fn analyze_content_chunks(&mut self) -> bool {
        while self.last_content_index + CONTENT_CHUNK_SIZE <= self.content_history.len() {
            // Copy the chunk to avoid borrowing content_history across mut self
            let chunk = self.content_history[self.last_content_index..self.last_content_index + CONTENT_CHUNK_SIZE].to_string();
            let hash = fast_hash(&chunk);

            // Inline the loop detection logic to avoid borrow issues
            let is_loop = {
                let indices = self.content_stats.entry(hash).or_default();

                if indices.is_empty() {
                    indices.push(self.last_content_index);
                    false
                } else {
                    // Verify actual content match
                    let first_idx = indices[0];
                    let matches = if first_idx + CONTENT_CHUNK_SIZE <= self.content_history.len() {
                        &self.content_history[first_idx..first_idx + CONTENT_CHUNK_SIZE] == chunk.as_str()
                    } else {
                        false
                    };

                    if !matches {
                        false
                    } else {
                        indices.push(self.last_content_index);

                        if indices.len() < CONTENT_LOOP_THRESHOLD {
                            false
                        } else {
                            let recent = &indices[indices.len() - CONTENT_LOOP_THRESHOLD..];
                            let total_distance = recent[recent.len() - 1] - recent[0];
                            let avg_distance = total_distance / (CONTENT_LOOP_THRESHOLD - 1);

                            if avg_distance > CONTENT_CHUNK_SIZE * 5 {
                                false // Too far apart
                            } else {
                                // Count unique periods
                                let mut unique_periods = std::collections::HashSet::new();
                                for i in 0..recent.len() - 1 {
                                    let end = recent[i + 1].min(self.content_history.len());
                                    let period = &self.content_history[recent[i]..end];
                                    unique_periods.insert(period.to_string());
                                }
                                unique_periods.len() <= CONTENT_LOOP_THRESHOLD / 2
                            }
                        }
                    }
                }
            };

            if is_loop {
                return true;
            }

            self.last_content_index += 1;
        }
        false
    }
}

// ── Hashing helpers ──────────────────────────────────────────────────────────

/// Fast non-cryptographic hash for tool call deduplication.
/// Uses FNV-1a for speed (no need for SHA256 in Rust — we verify content on match).
fn hash_tool_call(name: &str, args: &str) -> u64 {
    fast_hash(&format!("{name}:{args}"))
}

/// FNV-1a hash — fast, good distribution for short strings.
fn fast_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_loop_on_different_tool_calls() {
        let mut svc = LoopDetectionService::default();
        for i in 0..10 {
            let result = svc.check_tool_call("read_file", &format!(r#"{{"path":"file{i}.rs"}}"#));
            assert!(!result.is_loop(), "should not detect loop for different args");
        }
    }

    #[test]
    fn detect_identical_tool_call_loop() {
        let mut svc = LoopDetectionService::default();
        for i in 0..5 {
            let result = svc.check_tool_call("read_file", r#"{"path":"same.rs"}"#);
            if i < 4 {
                assert!(!result.is_loop());
            } else {
                assert!(result.is_loop());
                assert_eq!(result.loop_type, Some(LoopType::ConsecutiveIdenticalToolCalls));
            }
        }
    }

    #[test]
    fn detect_content_chanting() {
        let mut svc = LoopDetectionService::default();
        let repeated = "Hello world, this is a test of the loop detection. ";
        for _ in 0..15 {
            svc.check_content(repeated);
        }
        let result = svc.check_content(repeated);
        assert!(result.is_loop() || svc.loop_detected, "should detect chanting");
    }

    #[test]
    fn turn_budget_exceeded() {
        let mut svc = LoopDetectionService::new(5);
        for i in 0..5 {
            let result = svc.turn_started();
            if i < 4 {
                assert!(!result.is_loop());
            } else {
                assert!(result.is_loop());
                assert_eq!(result.loop_type, Some(LoopType::TurnBudgetExceeded));
            }
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut svc = LoopDetectionService::default();
        for _ in 0..5 {
            svc.check_tool_call("read_file", r#"{"path":"same.rs"}"#);
        }
        assert!(svc.loop_detected);
        svc.reset();
        assert!(!svc.loop_detected);
        assert_eq!(svc.detected_count, 0);
    }

    #[test]
    fn code_blocks_ignored() {
        let mut svc = LoopDetectionService::default();
        svc.check_content("```\n");
        for _ in 0..20 {
            svc.check_content("repeated line repeated line repeated line repeated ");
        }
        svc.check_content("\n```\n");
        // Should not detect loop inside code blocks
        assert!(!svc.loop_detected);
    }

    #[test]
    fn disabled_service_never_detects() {
        let mut svc = LoopDetectionService::default();
        svc.disable();
        for _ in 0..10 {
            let result = svc.check_tool_call("read_file", r#"{"path":"same.rs"}"#);
            assert!(!result.is_loop());
        }
    }
}
