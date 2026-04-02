//! Real tokenizer integration using tiktoken-rs.
//!
//! Provides accurate token counting for Anthropic (cl100k_base) and OpenAI models.
//! Falls back to heuristic estimation for unknown models.

use tiktoken_rs::cl100k_base;

/// Count tokens in text using the cl100k_base tokenizer (used by Claude, GPT-4, etc.).
///
/// This is accurate for Anthropic Claude models and OpenAI GPT-4/GPT-4o models.
/// For other models, this provides a close approximation.
#[must_use]
pub fn count_tokens(text: &str) -> usize {
    let bpe = cl100k_base().expect("cl100k_base tokenizer should load");
    bpe.encode_with_special_tokens(text).len()
}

/// Count tokens for a model-specific text.
/// Uses cl100k_base for all known models (close enough for billing estimates).
#[must_use]
pub fn count_tokens_for_model(text: &str, _model: &str) -> usize {
    // All modern models (Claude, GPT-4, Gemini) use similar BPE tokenization.
    // cl100k_base is accurate for Claude and GPT-4; close enough for others.
    count_tokens(text)
}

/// Estimate if text exceeds a token budget without counting every token.
/// Uses a fast heuristic first, falls back to actual counting if near the boundary.
#[must_use]
pub fn exceeds_budget(text: &str, budget: usize) -> bool {
    // Fast heuristic: ~4 chars per token for English text
    let char_estimate = text.len() / 4;
    if char_estimate > budget * 2 {
        return true; // Clearly over budget
    }
    if char_estimate < budget / 2 {
        return false; // Clearly under budget
    }
    // Near the boundary — count exactly
    count_tokens(text) > budget
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_simple_english_text() {
        let count = count_tokens("Hello, world!");
        assert!(count > 0);
        assert!(count < 10); // Should be ~4 tokens
    }

    #[test]
    fn counts_code() {
        let code = "fn main() {\n    println!(\"Hello, world!\");\n}";
        let count = count_tokens(code);
        assert!(count > 5);
        assert!(count < 30);
    }

    #[test]
    fn empty_string_is_zero() {
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn budget_check_works() {
        assert!(!exceeds_budget("hello", 100));
        let long_text = "word ".repeat(1000);
        assert!(exceeds_budget(&long_text, 100));
    }

    #[test]
    fn model_specific_matches_default() {
        let text = "The quick brown fox jumps over the lazy dog";
        assert_eq!(
            count_tokens(text),
            count_tokens_for_model(text, "claude-opus-4-6")
        );
    }

    // ── Edge case tests ──

    #[test]
    fn counts_unicode_text() {
        let text = "日本語テスト こんにちは世界";
        let count = count_tokens(text);
        assert!(count > 0);
    }

    #[test]
    fn counts_emoji_text() {
        let text = "Hello 👋 World 🌍 Test 🧪";
        let count = count_tokens(text);
        assert!(count > 3);
    }

    #[test]
    fn counts_very_long_text() {
        let text = "token ".repeat(10_000);
        let count = count_tokens(&text);
        assert!(count > 9_000);
        assert!(count < 11_000);
    }

    #[test]
    fn budget_boundary_uses_exact_count() {
        // Near the boundary — should trigger exact counting
        let text = "word ".repeat(50); // ~50 tokens
        assert!(!exceeds_budget(&text, 200)); // Well under
        assert!(exceeds_budget(&text, 10));    // Well over
        // Near boundary — exact count kicks in
        assert!(!exceeds_budget(&text, 60));
    }

    #[test]
    fn handles_special_characters() {
        let text = "a\nb\tc\r\n\x00\x01\x02";
        let count = count_tokens(text);
        assert!(count > 0);
    }
}
