//! Expert routing for /knowledge queries.
//!
//! Each intent maps to an expert configuration that controls:
//! - Which model tier to use
//! - How to frame the synthesis prompt
//! - What context to inject from learnings

use super::intent::Intent;

/// Expert configuration for a specific intent type.
#[derive(Debug, Clone)]
pub struct KnowledgeExpert {
    /// The intent this expert handles.
    pub intent: Intent,
    /// Model tier hint (fast/balanced/capable).
    pub model_tier: &'static str,
    /// Maximum KB results to request.
    pub max_results: usize,
    /// Whether to request synthesis from the KB backend.
    pub backend_synthesis: bool,
    /// System prompt template for the LLM synthesis.
    pub synthesis_prompt: String,
}

/// Routes intents to expert configurations.
pub struct ExpertRouter;

impl ExpertRouter {
    /// Get the expert for a given intent.
    #[must_use]
    pub fn expert_for(intent: Intent) -> KnowledgeExpert {
        match intent {
            Intent::Factual => KnowledgeExpert {
                intent,
                model_tier: "fast",
                max_results: 5,
                backend_synthesis: false,
                synthesis_prompt: format!(
                    "You are a knowledge expert. {}\n\
                     Answer concisely and precisely. If the KB results contain the answer, quote it directly.",
                    intent.expert_instruction()
                ),
            },
            Intent::Conceptual => KnowledgeExpert {
                intent,
                model_tier: "balanced",
                max_results: 10,
                backend_synthesis: false,
                synthesis_prompt: format!(
                    "You are a knowledge expert. {}\n\
                     Build understanding progressively. Start with the simple version, then add depth.",
                    intent.expert_instruction()
                ),
            },
            Intent::Procedural => KnowledgeExpert {
                intent,
                model_tier: "balanced",
                max_results: 8,
                backend_synthesis: false,
                synthesis_prompt: format!(
                    "You are a knowledge expert. {}\n\
                     Use numbered steps. Include prerequisites at the top. Add 'Common pitfalls' section at the end.",
                    intent.expert_instruction()
                ),
            },
            Intent::Comparative => KnowledgeExpert {
                intent,
                model_tier: "balanced",
                max_results: 10,
                backend_synthesis: false,
                synthesis_prompt: format!(
                    "You are a knowledge expert. {}\n\
                     Use a comparison table if appropriate. End with a clear recommendation.",
                    intent.expert_instruction()
                ),
            },
            Intent::Strategic => KnowledgeExpert {
                intent,
                model_tier: "capable",
                max_results: 10,
                backend_synthesis: false,
                synthesis_prompt: format!(
                    "You are a senior strategy advisor. {}\n\
                     Consider multiple angles. End with prioritized action items.",
                    intent.expert_instruction()
                ),
            },
            Intent::ExampleSeeking => KnowledgeExpert {
                intent,
                model_tier: "balanced",
                max_results: 8,
                backend_synthesis: false,
                synthesis_prompt: format!(
                    "You are a knowledge expert. {}\n\
                     Provide at least 3 examples. Annotate each with context and when to use it.",
                    intent.expert_instruction()
                ),
            },
            Intent::Diagnostic => KnowledgeExpert {
                intent,
                model_tier: "capable",
                max_results: 10,
                backend_synthesis: false,
                synthesis_prompt: format!(
                    "You are a senior diagnostics expert. {}\n\
                     Start with the most likely cause. Provide specific, actionable fixes.",
                    intent.expert_instruction()
                ),
            },
            Intent::General => KnowledgeExpert {
                intent,
                model_tier: "balanced",
                max_results: 10,
                backend_synthesis: false,
                synthesis_prompt: format!(
                    "You are a knowledge expert. {}\n\
                     Be comprehensive but concise.",
                    intent.expert_instruction()
                ),
            },
        }
    }

    /// Build the full prompt for LLM synthesis, including:
    /// - Expert system prompt
    /// - Past learnings from DB
    /// - KB results
    /// - User query
    #[must_use]
    pub fn build_synthesis_prompt(
        expert: &KnowledgeExpert,
        query: &str,
        kb_results: &str,
        learning_context: &str,
    ) -> String {
        let mut prompt = expert.synthesis_prompt.clone();

        // Inject learnings
        if !learning_context.is_empty() {
            prompt.push_str("\n\nYou have learned from past interactions:");
            prompt.push_str(learning_context);
            prompt.push_str("\nApply these learnings to improve your response.");
        }

        // Inject KB results
        if !kb_results.is_empty() {
            prompt.push_str("\n\n<knowledge-base-results>\n");
            prompt.push_str(kb_results);
            prompt.push_str("\n</knowledge-base-results>");
        } else {
            prompt.push_str("\n\nNo knowledge base results were found. Answer from your training knowledge.");
        }

        prompt.push_str(&format!("\n\n## User Query\n{query}"));

        // Add feedback request
        prompt.push_str(
            "\n\n---\n*After answering, I'll ask for your feedback to help me learn and improve.*"
        );

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expert_for_each_intent() {
        for intent in [
            Intent::Factual, Intent::Conceptual, Intent::Procedural,
            Intent::Comparative, Intent::Strategic, Intent::ExampleSeeking,
            Intent::Diagnostic, Intent::General,
        ] {
            let expert = ExpertRouter::expert_for(intent);
            assert_eq!(expert.intent, intent);
            assert!(!expert.synthesis_prompt.is_empty());
        }
    }

    #[test]
    fn synthesis_prompt_includes_all_sections() {
        let expert = ExpertRouter::expert_for(Intent::Factual);
        let prompt = ExpertRouter::build_synthesis_prompt(
            &expert,
            "what is a growth hook?",
            "Result 1: A growth hook is...",
            "\n<past-learnings>\n- Users prefer concise answers\n</past-learnings>\n",
        );
        assert!(prompt.contains("what is a growth hook?"));
        assert!(prompt.contains("knowledge-base-results"));
        assert!(prompt.contains("past-learnings"));
        assert!(prompt.contains("feedback"));
    }

    #[test]
    fn synthesis_prompt_handles_empty_kb() {
        let expert = ExpertRouter::expert_for(Intent::General);
        let prompt = ExpertRouter::build_synthesis_prompt(&expert, "test", "", "");
        assert!(prompt.contains("No knowledge base results"));
    }
}
