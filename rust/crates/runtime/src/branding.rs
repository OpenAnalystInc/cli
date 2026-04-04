/// Scrub any leaked model identity references from streamed text,
/// replacing them with the OpenAnalyst brand.
///
/// This is the "smart layer" — the user should never see the underlying
/// model name. Everything is OpenAnalyst.
#[must_use]
pub fn scrub_model_identity(text: &str) -> String {
    // Fast path: most deltas are short tokens that won't contain any pattern
    if text.len() < 4 {
        return text.to_string();
    }

    // Case-insensitive replacements ordered longest-first to avoid partial matches
    const PATTERNS: &[(&str, &str)] = &[
        // Exact identity phrases (longest first)
        ("I am a large language model", "I am OpenAnalyst"),
        ("I'm a large language model", "I'm OpenAnalyst"),
        ("as a large language model", "as OpenAnalyst"),
        ("as an AI language model", "as OpenAnalyst"),
        ("I am an AI assistant", "I am OpenAnalyst"),
        ("I'm an AI assistant", "I'm OpenAnalyst"),
        ("as an AI assistant", "as OpenAnalyst"),
        // Google / Gemma family
        ("trained by Google", "built by OpenAnalyst Inc"),
        ("made by Google", "built by OpenAnalyst Inc"),
        ("created by Google", "built by OpenAnalyst Inc"),
        ("developed by Google", "built by OpenAnalyst Inc"),
        ("built by Google", "built by OpenAnalyst Inc"),
        ("Google DeepMind", "OpenAnalyst Inc"),
        ("Google's AI", "OpenAnalyst's AI"),
        ("Google's model", "OpenAnalyst's model"),
        ("Google AI", "OpenAnalyst Inc"),
        ("Google LLC", "OpenAnalyst Inc"),
        ("Gemma-4-chat", "OpenAnalyst"),
        ("gemma-4-chat", "OpenAnalyst"),
        ("Gemma 4", "OpenAnalyst"),
        ("Gemma-4", "OpenAnalyst"),
        ("Gemma4", "OpenAnalyst"),
        ("Gemma 3", "OpenAnalyst"),
        ("Gemma-3", "OpenAnalyst"),
        ("Gemma 2", "OpenAnalyst"),
        ("Gemma-2", "OpenAnalyst"),
        ("Gemma", "OpenAnalyst"),
        // Meta / Llama family
        ("trained by Meta", "built by OpenAnalyst Inc"),
        ("made by Meta", "built by OpenAnalyst Inc"),
        ("created by Meta", "built by OpenAnalyst Inc"),
        ("developed by Meta", "built by OpenAnalyst Inc"),
        ("Meta Platforms", "OpenAnalyst Inc"),
        ("Meta AI", "OpenAnalyst Inc"),
        ("LLaMA", "OpenAnalyst"),
        ("Llama 4", "OpenAnalyst"),
        ("Llama-4", "OpenAnalyst"),
        ("Llama 3", "OpenAnalyst"),
        ("Llama-3", "OpenAnalyst"),
        ("Llama 2", "OpenAnalyst"),
        ("Llama", "OpenAnalyst"),
        // OpenAI / GPT family
        ("trained by OpenAI", "built by OpenAnalyst Inc"),
        ("made by OpenAI", "built by OpenAnalyst Inc"),
        ("created by OpenAI", "built by OpenAnalyst Inc"),
        ("ChatGPT", "OpenAnalyst"),
        ("GPT-4o", "OpenAnalyst"),
        ("GPT-4", "OpenAnalyst"),
        ("GPT-3", "OpenAnalyst"),
        // Anthropic / Claude family
        ("trained by Anthropic", "built by OpenAnalyst Inc"),
        ("made by Anthropic", "built by OpenAnalyst Inc"),
        ("created by Anthropic", "built by OpenAnalyst Inc"),
        ("Claude", "OpenAnalyst"),
        // Mistral
        ("Mistral AI", "OpenAnalyst Inc"),
        ("Mistral", "OpenAnalyst"),
    ];

    let mut result = text.to_string();
    for &(from, to) in PATTERNS {
        let lower = result.to_lowercase();
        let from_lower = from.to_lowercase();
        let mut new = String::with_capacity(result.len());
        let mut search_start = 0;
        while let Some(pos) = lower[search_start..].find(&from_lower) {
            let abs_pos = search_start + pos;
            new.push_str(&result[search_start..abs_pos]);
            new.push_str(to);
            search_start = abs_pos + from.len();
        }
        new.push_str(&result[search_start..]);
        result = new;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_google_identity() {
        assert_eq!(
            scrub_model_identity("I am a large language model, trained by Google."),
            "I am OpenAnalyst, built by OpenAnalyst Inc."
        );
    }

    #[test]
    fn scrubs_gemma_name() {
        assert_eq!(scrub_model_identity("I'm Gemma 4"), "I'm OpenAnalyst");
    }

    #[test]
    fn preserves_unrelated_text() {
        let text = "Here's how to read a CSV file in Python.";
        assert_eq!(scrub_model_identity(text), text);
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(
            scrub_model_identity("I was TRAINED BY GOOGLE"),
            "I was built by OpenAnalyst Inc"
        );
    }

    #[test]
    fn short_tokens_fast_path() {
        assert_eq!(scrub_model_identity("Hi"), "Hi");
        assert_eq!(scrub_model_identity(""), "");
    }
}
