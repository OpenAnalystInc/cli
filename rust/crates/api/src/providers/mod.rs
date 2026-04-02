use std::future::Future;
use std::pin::Pin;

use crate::error::ApiError;
use crate::types::{MessageRequest, MessageResponse};

pub mod openanalyst_provider;
pub mod openai_compat;

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;

pub trait Provider {
    type Stream;

    fn send_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, MessageResponse>;

    fn stream_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, Self::Stream>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// OpenAnalyst API (Anthropic-compatible format)
    OpenAnalystApi,
    /// Anthropic Claude API (native format)
    Anthropic,
    /// OpenAI / GPT / Codex
    OpenAi,
    /// xAI Grok
    Xai,
    /// OpenRouter (OpenAI-compatible, multi-model gateway)
    OpenRouter,
    /// Amazon Bedrock (OpenAI-compatible gateway)
    Bedrock,
    /// Google Gemini (OpenAI-compatible gateway)
    Gemini,
}

impl ProviderKind {
    /// Human-readable provider name for the banner
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::OpenAnalystApi => "OpenAnalyst Inc",
            Self::Anthropic => "Anthropic",
            Self::OpenAi => "OpenAI",
            Self::Xai => "xAI",
            Self::OpenRouter => "OpenRouter",
            Self::Bedrock => "Amazon Bedrock",
            Self::Gemini => "Google Gemini",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub provider: ProviderKind,
    pub auth_env: &'static str,
    pub base_url_env: &'static str,
    pub default_base_url: &'static str,
}

// ── Model Registry: maps model names/aliases → provider + auth config ──

const MODEL_REGISTRY: &[(&str, ProviderMetadata)] = &[
    // ── OpenAnalyst ──
    ("openanalyst-beta", ProviderMetadata {
        provider: ProviderKind::OpenAnalystApi,
        auth_env: "OPENANALYST_API_KEY",
        base_url_env: "OPENANALYST_BASE_URL",
        default_base_url: openanalyst_provider::DEFAULT_BASE_URL,
    }),
    ("openanalyst", ProviderMetadata {
        provider: ProviderKind::OpenAnalystApi,
        auth_env: "OPENANALYST_API_KEY",
        base_url_env: "OPENANALYST_BASE_URL",
        default_base_url: openanalyst_provider::DEFAULT_BASE_URL,
    }),

    // ── Anthropic / Claude (also routed via OpenAnalyst when OA auth is set) ──
    ("opus", ProviderMetadata {
        provider: ProviderKind::OpenAnalystApi,
        auth_env: "OPENANALYST_API_KEY",
        base_url_env: "OPENANALYST_BASE_URL",
        default_base_url: openanalyst_provider::DEFAULT_BASE_URL,
    }),
    ("sonnet", ProviderMetadata {
        provider: ProviderKind::OpenAnalystApi,
        auth_env: "OPENANALYST_API_KEY",
        base_url_env: "OPENANALYST_BASE_URL",
        default_base_url: openanalyst_provider::DEFAULT_BASE_URL,
    }),
    ("haiku", ProviderMetadata {
        provider: ProviderKind::OpenAnalystApi,
        auth_env: "OPENANALYST_API_KEY",
        base_url_env: "OPENANALYST_BASE_URL",
        default_base_url: openanalyst_provider::DEFAULT_BASE_URL,
    }),
    ("claude-opus-4-6", ProviderMetadata {
        provider: ProviderKind::OpenAnalystApi,
        auth_env: "OPENANALYST_API_KEY",
        base_url_env: "OPENANALYST_BASE_URL",
        default_base_url: openanalyst_provider::DEFAULT_BASE_URL,
    }),
    ("claude-sonnet-4-6", ProviderMetadata {
        provider: ProviderKind::OpenAnalystApi,
        auth_env: "OPENANALYST_API_KEY",
        base_url_env: "OPENANALYST_BASE_URL",
        default_base_url: openanalyst_provider::DEFAULT_BASE_URL,
    }),
    ("claude-haiku-4-5-20251213", ProviderMetadata {
        provider: ProviderKind::OpenAnalystApi,
        auth_env: "OPENANALYST_API_KEY",
        base_url_env: "OPENANALYST_BASE_URL",
        default_base_url: openanalyst_provider::DEFAULT_BASE_URL,
    }),

    // ── OpenAI / GPT / Codex ──
    ("gpt-4o", ProviderMetadata {
        provider: ProviderKind::OpenAi,
        auth_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
    }),
    ("gpt-4o-mini", ProviderMetadata {
        provider: ProviderKind::OpenAi,
        auth_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
    }),
    ("gpt-4.1", ProviderMetadata {
        provider: ProviderKind::OpenAi,
        auth_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
    }),
    ("gpt-4.1-mini", ProviderMetadata {
        provider: ProviderKind::OpenAi,
        auth_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
    }),
    ("gpt-4.1-nano", ProviderMetadata {
        provider: ProviderKind::OpenAi,
        auth_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
    }),
    ("o3", ProviderMetadata {
        provider: ProviderKind::OpenAi,
        auth_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
    }),
    ("o3-mini", ProviderMetadata {
        provider: ProviderKind::OpenAi,
        auth_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
    }),
    ("o4-mini", ProviderMetadata {
        provider: ProviderKind::OpenAi,
        auth_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
    }),
    ("codex-mini", ProviderMetadata {
        provider: ProviderKind::OpenAi,
        auth_env: "OPENAI_API_KEY",
        base_url_env: "OPENAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
    }),

    // ── xAI / Grok ──
    ("grok", ProviderMetadata {
        provider: ProviderKind::Xai,
        auth_env: "XAI_API_KEY",
        base_url_env: "XAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
    }),
    ("grok-3", ProviderMetadata {
        provider: ProviderKind::Xai,
        auth_env: "XAI_API_KEY",
        base_url_env: "XAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
    }),
    ("grok-mini", ProviderMetadata {
        provider: ProviderKind::Xai,
        auth_env: "XAI_API_KEY",
        base_url_env: "XAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
    }),
    ("grok-3-mini", ProviderMetadata {
        provider: ProviderKind::Xai,
        auth_env: "XAI_API_KEY",
        base_url_env: "XAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
    }),
    ("grok-2", ProviderMetadata {
        provider: ProviderKind::Xai,
        auth_env: "XAI_API_KEY",
        base_url_env: "XAI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
    }),

    // ── OpenRouter (any model via OpenAI-compat gateway) ──
    ("openrouter/auto", ProviderMetadata {
        provider: ProviderKind::OpenRouter,
        auth_env: "OPENROUTER_API_KEY",
        base_url_env: "OPENROUTER_BASE_URL",
        default_base_url: openai_compat::DEFAULT_OPENROUTER_BASE_URL,
    }),

    // ── Amazon Bedrock (via OpenAI-compat gateway) ──
    ("bedrock/claude", ProviderMetadata {
        provider: ProviderKind::Bedrock,
        auth_env: "BEDROCK_API_KEY",
        base_url_env: "BEDROCK_BASE_URL",
        default_base_url: openai_compat::DEFAULT_BEDROCK_BASE_URL,
    }),

    // ── Google Gemini (via OpenAI-compat gateway) ──
    ("gemini-2.5-pro", ProviderMetadata {
        provider: ProviderKind::Gemini,
        auth_env: "GEMINI_API_KEY",
        base_url_env: "GEMINI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_GEMINI_BASE_URL,
    }),
    ("gemini-2.5-flash", ProviderMetadata {
        provider: ProviderKind::Gemini,
        auth_env: "GEMINI_API_KEY",
        base_url_env: "GEMINI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_GEMINI_BASE_URL,
    }),
    ("gemini-2.0-flash", ProviderMetadata {
        provider: ProviderKind::Gemini,
        auth_env: "GEMINI_API_KEY",
        base_url_env: "GEMINI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_GEMINI_BASE_URL,
    }),
    ("gemini-2.0-flash-lite", ProviderMetadata {
        provider: ProviderKind::Gemini,
        auth_env: "GEMINI_API_KEY",
        base_url_env: "GEMINI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_GEMINI_BASE_URL,
    }),
    ("gemini-1.5-pro", ProviderMetadata {
        provider: ProviderKind::Gemini,
        auth_env: "GEMINI_API_KEY",
        base_url_env: "GEMINI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_GEMINI_BASE_URL,
    }),
    ("gemini-1.5-flash", ProviderMetadata {
        provider: ProviderKind::Gemini,
        auth_env: "GEMINI_API_KEY",
        base_url_env: "GEMINI_BASE_URL",
        default_base_url: openai_compat::DEFAULT_GEMINI_BASE_URL,
    }),
];

#[must_use]
pub fn resolve_model_alias(model: &str) -> String {
    let trimmed = model.trim();
    let lower = trimmed.to_ascii_lowercase();
    MODEL_REGISTRY
        .iter()
        .find_map(|(alias, metadata)| {
            (*alias == lower).then_some(match metadata.provider {
                ProviderKind::OpenAnalystApi | ProviderKind::Anthropic => match *alias {
                    "openanalyst" | "openanalyst-beta" => "openanalyst-beta",
                    "opus" => "claude-opus-4-6",
                    "sonnet" => "claude-sonnet-4-6",
                    "haiku" => "claude-haiku-4-5-20251213",
                    _ => trimmed,
                },
                ProviderKind::Xai => match *alias {
                    "grok" | "grok-3" => "grok-3",
                    "grok-mini" | "grok-3-mini" => "grok-3-mini",
                    "grok-2" => "grok-2",
                    _ => trimmed,
                },
                ProviderKind::OpenAi
                | ProviderKind::OpenRouter
                | ProviderKind::Bedrock
                | ProviderKind::Gemini => trimmed,
            })
        })
        .map_or_else(|| trimmed.to_string(), ToOwned::to_owned)
}

#[must_use]
pub fn metadata_for_model(model: &str) -> Option<ProviderMetadata> {
    let canonical = resolve_model_alias(model);
    let lower = canonical.to_ascii_lowercase();

    // Exact match in registry
    if let Some((_, metadata)) = MODEL_REGISTRY.iter().find(|(alias, _)| *alias == lower) {
        return Some(*metadata);
    }

    // Prefix-based detection for models not in registry
    if lower.starts_with("grok") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        });
    }
    if lower.starts_with("gpt-") || lower.starts_with("o1") || lower.starts_with("o3")
        || lower.starts_with("o4") || lower.starts_with("codex")
    {
        return Some(ProviderMetadata {
            provider: ProviderKind::OpenAi,
            auth_env: "OPENAI_API_KEY",
            base_url_env: "OPENAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
        });
    }
    if lower.starts_with("openrouter/") {
        return Some(ProviderMetadata {
            provider: ProviderKind::OpenRouter,
            auth_env: "OPENROUTER_API_KEY",
            base_url_env: "OPENROUTER_BASE_URL",
            default_base_url: openai_compat::DEFAULT_OPENROUTER_BASE_URL,
        });
    }
    if lower.starts_with("bedrock/") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Bedrock,
            auth_env: "BEDROCK_API_KEY",
            base_url_env: "BEDROCK_BASE_URL",
            default_base_url: openai_compat::DEFAULT_BEDROCK_BASE_URL,
        });
    }
    if lower.starts_with("gemini") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Gemini,
            auth_env: "GEMINI_API_KEY",
            base_url_env: "GEMINI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_GEMINI_BASE_URL,
        });
    }
    None
}

/// Detect the provider from the model name, then fall back to checking env vars
#[must_use]
pub fn detect_provider_kind(model: &str) -> ProviderKind {
    // 1. Model name → provider
    if let Some(metadata) = metadata_for_model(model) {
        return metadata.provider;
    }
    // 2. Check which auth env vars are set
    if openanalyst_provider::has_auth_from_env_or_saved().unwrap_or(false) {
        return ProviderKind::OpenAnalystApi;
    }
    if openai_compat::has_api_key("OPENAI_API_KEY") {
        return ProviderKind::OpenAi;
    }
    if openai_compat::has_api_key("XAI_API_KEY") {
        return ProviderKind::Xai;
    }
    if openai_compat::has_api_key("OPENROUTER_API_KEY") {
        return ProviderKind::OpenRouter;
    }
    if openai_compat::has_api_key("BEDROCK_API_KEY") {
        return ProviderKind::Bedrock;
    }
    if openai_compat::has_api_key("GEMINI_API_KEY") {
        return ProviderKind::Gemini;
    }
    if openai_compat::has_api_key("ANTHROPIC_API_KEY") {
        return ProviderKind::Anthropic;
    }
    // Default
    ProviderKind::OpenAnalystApi
}

#[must_use]
pub fn max_tokens_for_model(model: &str) -> u32 {
    let canonical = resolve_model_alias(model);
    if canonical.contains("opus") {
        32_000
    } else if canonical.starts_with("gemini-2.5") {
        // Gemini 2.5 Pro/Flash support up to 65k output tokens via OpenAI-compat
        65_536
    } else if canonical.starts_with("gemini-1.5") {
        // Gemini 1.5 Pro supports 8k output tokens
        8_192
    } else if canonical.starts_with("gemini") {
        // Gemini 2.0 Flash models
        8_192
    } else {
        64_000
    }
}

#[cfg(test)]
mod tests {
    use super::{detect_provider_kind, max_tokens_for_model, resolve_model_alias, ProviderKind};

    #[test]
    fn resolves_grok_aliases() {
        assert_eq!(resolve_model_alias("grok"), "grok-3");
        assert_eq!(resolve_model_alias("grok-mini"), "grok-3-mini");
        assert_eq!(resolve_model_alias("grok-2"), "grok-2");
    }

    #[test]
    fn resolves_openai_models() {
        assert_eq!(detect_provider_kind("gpt-4o"), ProviderKind::OpenAi);
        assert_eq!(detect_provider_kind("o3-mini"), ProviderKind::OpenAi);
        assert_eq!(detect_provider_kind("codex-mini"), ProviderKind::OpenAi);
    }

    #[test]
    fn resolves_openrouter_models() {
        assert_eq!(detect_provider_kind("openrouter/auto"), ProviderKind::OpenRouter);
        assert_eq!(detect_provider_kind("openrouter/anthropic/claude-3.5-sonnet"), ProviderKind::OpenRouter);
    }

    #[test]
    fn resolves_bedrock_models() {
        assert_eq!(detect_provider_kind("bedrock/claude"), ProviderKind::Bedrock);
        assert_eq!(detect_provider_kind("bedrock/anthropic.claude-v2"), ProviderKind::Bedrock);
    }

    #[test]
    fn detects_provider_from_model_name_first() {
        assert_eq!(detect_provider_kind("grok"), ProviderKind::Xai);
        assert_eq!(detect_provider_kind("openanalyst-beta"), ProviderKind::OpenAnalystApi);
        assert_eq!(detect_provider_kind("claude-sonnet-4-6"), ProviderKind::OpenAnalystApi);
    }

    #[test]
    fn resolves_gemini_models() {
        assert_eq!(detect_provider_kind("gemini-2.5-pro"), ProviderKind::Gemini);
        assert_eq!(detect_provider_kind("gemini-2.5-flash"), ProviderKind::Gemini);
        assert_eq!(detect_provider_kind("gemini-2.0-flash"), ProviderKind::Gemini);
        assert_eq!(detect_provider_kind("gemini-1.5-pro"), ProviderKind::Gemini);
    }

    #[test]
    fn keeps_existing_max_token_heuristic() {
        assert_eq!(max_tokens_for_model("opus"), 32_000);
        assert_eq!(max_tokens_for_model("grok-3"), 64_000);
    }
}
