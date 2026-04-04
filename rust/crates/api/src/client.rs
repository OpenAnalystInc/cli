use crate::error::ApiError;
use crate::providers::openanalyst_provider::{self, AuthSource, OpenAnalystApiClient};
use crate::providers::openai_compat::{self, OpenAiCompatClient, OpenAiCompatConfig};
use crate::providers::{self, Provider, ProviderKind};
use crate::types::{MessageRequest, MessageResponse, StreamEvent};

async fn send_via_provider<P: Provider>(
    provider: &P,
    request: &MessageRequest,
) -> Result<MessageResponse, ApiError> {
    provider.send_message(request).await
}

async fn stream_via_provider<P: Provider>(
    provider: &P,
    request: &MessageRequest,
) -> Result<P::Stream, ApiError> {
    provider.stream_message(request).await
}

#[derive(Debug, Clone)]
pub enum ProviderClient {
    /// OpenAnalyst or Anthropic (Anthropic message format)
    OpenAnalystApi(OpenAnalystApiClient),
    /// OpenAI, xAI, OpenRouter, Bedrock, Gemini (OpenAI chat completions format)
    OpenAiCompat(OpenAiCompatClient, ProviderKind),
}

impl ProviderClient {
    pub fn from_model(model: &str) -> Result<Self, ApiError> {
        Self::from_model_with_default_auth(model, None)
    }

    pub fn from_model_with_default_auth(
        model: &str,
        default_auth: Option<AuthSource>,
    ) -> Result<Self, ApiError> {
        let resolved_model = providers::resolve_model_alias(model);
        match providers::detect_provider_kind(&resolved_model) {
            // OpenAnalyst API — always through api.openanalyst.com
            ProviderKind::OpenAnalystApi => {
                Ok(Self::OpenAnalystApi(match default_auth {
                    Some(auth) => OpenAnalystApiClient::from_auth(auth),
                    None => OpenAnalystApiClient::from_env()?,
                }))
            }
            // Anthropic direct (user's own ANTHROPIC_API_KEY → api.anthropic.com)
            ProviderKind::Anthropic => {
                Ok(Self::OpenAnalystApi(match default_auth {
                    Some(auth) => OpenAnalystApiClient::from_auth(auth)
                        .with_base_url(openanalyst_provider::DEFAULT_ANTHROPIC_BASE_URL),
                    None => OpenAnalystApiClient::from_anthropic_env()?,
                }))
            }
            // OpenAI-format providers
            kind @ ProviderKind::OpenAi => Ok(Self::OpenAiCompat(
                OpenAiCompatClient::from_env(OpenAiCompatConfig::openai())?,
                kind,
            )),
            kind @ ProviderKind::Xai => Ok(Self::OpenAiCompat(
                OpenAiCompatClient::from_env(OpenAiCompatConfig::xai())?,
                kind,
            )),
            kind @ ProviderKind::OpenRouter => Ok(Self::OpenAiCompat(
                OpenAiCompatClient::from_env(OpenAiCompatConfig::openrouter())?,
                kind,
            )),
            kind @ ProviderKind::Bedrock => Ok(Self::OpenAiCompat(
                OpenAiCompatClient::from_env(OpenAiCompatConfig::bedrock())?,
                kind,
            )),
            kind @ ProviderKind::Gemini => Ok(Self::OpenAiCompat(
                OpenAiCompatClient::from_env(OpenAiCompatConfig::gemini())?,
                kind,
            )),
        }
    }

    #[must_use]
    pub fn provider_kind(&self) -> ProviderKind {
        match self {
            Self::OpenAnalystApi(_) => ProviderKind::OpenAnalystApi,
            Self::OpenAiCompat(_, kind) => *kind,
        }
    }

    pub async fn send_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageResponse, ApiError> {
        match self {
            Self::OpenAnalystApi(client) => send_via_provider(client, request).await,
            Self::OpenAiCompat(client, _) => send_via_provider(client, request).await,
        }
    }

    pub async fn stream_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageStream, ApiError> {
        match self {
            Self::OpenAnalystApi(client) => stream_via_provider(client, request)
                .await
                .map(MessageStream::OpenAnalystApi),
            Self::OpenAiCompat(client, _) => stream_via_provider(client, request)
                .await
                .map(MessageStream::OpenAiCompat),
        }
    }
}

#[derive(Debug)]
pub enum MessageStream {
    OpenAnalystApi(openanalyst_provider::MessageStream),
    OpenAiCompat(openai_compat::MessageStream),
}

impl MessageStream {
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::OpenAnalystApi(stream) => stream.request_id(),
            Self::OpenAiCompat(stream) => stream.request_id(),
        }
    }

    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, ApiError> {
        match self {
            Self::OpenAnalystApi(stream) => stream.next_event().await,
            Self::OpenAiCompat(stream) => stream.next_event().await,
        }
    }
}

pub use openanalyst_provider::{
    oauth_token_is_expired, resolve_saved_oauth_token, resolve_startup_auth_source, OAuthTokenSet,
};
#[must_use]
pub fn read_base_url() -> String {
    openanalyst_provider::read_base_url()
}

#[must_use]
pub fn read_xai_base_url() -> String {
    openai_compat::read_base_url(OpenAiCompatConfig::xai())
}

#[cfg(test)]
mod tests {
    use crate::providers::{detect_provider_kind, resolve_model_alias, ProviderKind};

    #[test]
    fn resolves_existing_and_grok_aliases() {
        assert_eq!(resolve_model_alias("opus"), "claude-opus-4-6");
        assert_eq!(resolve_model_alias("grok"), "grok-3");
        assert_eq!(resolve_model_alias("grok-mini"), "grok-3-mini");
    }

    #[test]
    fn provider_detection_prefers_model_family() {
        assert_eq!(detect_provider_kind("grok-3"), ProviderKind::Xai);
        assert_eq!(detect_provider_kind("gpt-4o"), ProviderKind::OpenAi);
        assert_eq!(detect_provider_kind("openrouter/auto"), ProviderKind::OpenRouter);
        assert_eq!(detect_provider_kind("bedrock/claude"), ProviderKind::Bedrock);
        assert_eq!(detect_provider_kind("gemini-2.5-pro"), ProviderKind::Gemini);
        assert_eq!(
            detect_provider_kind("claude-sonnet-4-6"),
            ProviderKind::Anthropic
        );
    }
}
