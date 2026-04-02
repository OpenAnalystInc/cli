//! Intent classification for /knowledge queries.
//!
//! Classifies user queries into categories to route to the right expert agent.
//! Uses keyword heuristics (fast, no API call) with optional LLM refinement.

use serde::{Deserialize, Serialize};

/// The classified intent of a knowledge query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Intent {
    /// "What is X?" — looking up a specific fact or definition.
    Factual,
    /// "Explain how X works" — understanding a concept deeply.
    Conceptual,
    /// "How do I do X?" — step-by-step instructions or process.
    Procedural,
    /// "What's the difference between X and Y?" — comparing options.
    Comparative,
    /// "What's the best approach for X?" — strategic/creative advice.
    Strategic,
    /// "Show me examples of X" — looking for concrete examples.
    ExampleSeeking,
    /// "What went wrong with X?" — debugging or troubleshooting.
    Diagnostic,
    /// Couldn't classify clearly — will use general expert.
    General,
}

impl Intent {
    /// Human-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Factual => "factual",
            Self::Conceptual => "conceptual",
            Self::Procedural => "procedural",
            Self::Comparative => "comparative",
            Self::Strategic => "strategic",
            Self::ExampleSeeking => "example",
            Self::Diagnostic => "diagnostic",
            Self::General => "general",
        }
    }

    /// Parse from stored string.
    #[must_use]
    pub fn from_label(s: &str) -> Self {
        match s {
            "factual" => Self::Factual,
            "conceptual" => Self::Conceptual,
            "procedural" => Self::Procedural,
            "comparative" => Self::Comparative,
            "strategic" => Self::Strategic,
            "example" => Self::ExampleSeeking,
            "diagnostic" => Self::Diagnostic,
            _ => Self::General,
        }
    }

    /// Description for the expert system prompt.
    #[must_use]
    pub const fn expert_instruction(self) -> &'static str {
        match self {
            Self::Factual => "Give a precise, factual answer with specific details. Cite sources if available.",
            Self::Conceptual => "Explain the concept thoroughly. Use analogies and build understanding from fundamentals.",
            Self::Procedural => "Provide clear step-by-step instructions. Number each step. Include prerequisites and common pitfalls.",
            Self::Comparative => "Compare the options systematically. Use a structured format (pros/cons, table). Give a recommendation.",
            Self::Strategic => "Analyze the situation and recommend the best approach. Consider trade-offs, constraints, and long-term implications.",
            Self::ExampleSeeking => "Provide multiple concrete examples. Show code, templates, or real-world cases. Annotate each example.",
            Self::Diagnostic => "Analyze what went wrong. Identify root causes. Suggest specific fixes ordered by likelihood.",
            Self::General => "Answer comprehensively. If the question is ambiguous, address the most likely interpretation.",
        }
    }
}

/// Classifies user queries into intents using keyword heuristics.
pub struct IntentClassifier;

impl IntentClassifier {
    /// Classify a query into an intent.
    #[must_use]
    pub fn classify(query: &str) -> Intent {
        let lower = query.to_ascii_lowercase();

        // Diagnostic — error/bug/issue/problem patterns
        if contains_any(&lower, &["what went wrong", "why does", "error", "bug", "issue", "problem", "not working", "fails", "crash", "debug"]) {
            return Intent::Diagnostic;
        }

        // Comparative — difference/compare/vs patterns
        if contains_any(&lower, &["difference between", "compare", " vs ", "versus", "which is better", "pros and cons", "trade-off"]) {
            return Intent::Comparative;
        }

        // Procedural — how to/steps/process patterns
        if lower.starts_with("how do") || lower.starts_with("how to") || lower.starts_with("how can")
            || contains_any(&lower, &["step by step", "steps to", "guide to", "tutorial", "walkthrough", "set up", "configure", "install"])
        {
            return Intent::Procedural;
        }

        // Example seeking
        if contains_any(&lower, &["example", "show me", "demonstrate", "sample", "template", "code for"]) {
            return Intent::ExampleSeeking;
        }

        // Strategic — best/strategy/approach/recommend patterns
        if contains_any(&lower, &["best approach", "best way", "strategy", "recommend", "should i", "optimal", "what approach", "best practice"]) {
            return Intent::Strategic;
        }

        // Factual — what is/who/when/where patterns
        if lower.starts_with("what is") || lower.starts_with("what are")
            || lower.starts_with("who ") || lower.starts_with("when ")
            || lower.starts_with("where ") || lower.starts_with("define ")
            || contains_any(&lower, &["meaning of", "definition of"])
        {
            return Intent::Factual;
        }

        // Conceptual — explain/understand/why patterns
        if lower.starts_with("explain") || lower.starts_with("why ")
            || contains_any(&lower, &["how does", "how works", "concept of", "understand", "theory behind", "principle"])
        {
            return Intent::Conceptual;
        }

        Intent::General
    }
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| text.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_factual() {
        assert_eq!(IntentClassifier::classify("what is a growth hook?"), Intent::Factual);
        assert_eq!(IntentClassifier::classify("define product-market fit"), Intent::Factual);
    }

    #[test]
    fn classifies_procedural() {
        assert_eq!(IntentClassifier::classify("how to set up CI/CD pipeline"), Intent::Procedural);
        assert_eq!(IntentClassifier::classify("steps to deploy to AWS"), Intent::Procedural);
    }

    #[test]
    fn classifies_comparative() {
        assert_eq!(IntentClassifier::classify("difference between REST and GraphQL"), Intent::Comparative);
        assert_eq!(IntentClassifier::classify("React vs Vue pros and cons"), Intent::Comparative);
    }

    #[test]
    fn classifies_strategic() {
        assert_eq!(IntentClassifier::classify("best approach for scaling microservices"), Intent::Strategic);
        assert_eq!(IntentClassifier::classify("should i use monorepo or polyrepo"), Intent::Strategic);
    }

    #[test]
    fn classifies_diagnostic() {
        assert_eq!(IntentClassifier::classify("why does my build fail on CI"), Intent::Diagnostic);
        assert_eq!(IntentClassifier::classify("debug the auth error"), Intent::Diagnostic);
    }

    #[test]
    fn classifies_example() {
        assert_eq!(IntentClassifier::classify("show me examples of async Rust"), Intent::ExampleSeeking);
    }

    #[test]
    fn classifies_conceptual() {
        assert_eq!(IntentClassifier::classify("explain how OAuth 2.0 works"), Intent::Conceptual);
    }
}
