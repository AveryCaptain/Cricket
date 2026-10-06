//! 上游协议差异→归一化 `GatewayEvent`

mod normalize;
mod transport;

use cricket_protocol::{
    CoreEvent, CricketError, FinishReason, ModelPref, ModelRef, ProviderId, Role, Usage,
};
use futures::{future::BoxFuture, stream::BoxStream};
pub use normalize::{NormalizeState, RawSseFrame};
use serde::{Deserialize, Serialize};
pub use transport::{replay, stream_request, MockProvider};

pub type GatewayStream = BoxStream<'static, Result<GatewayEvent, CricketError>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    Reasoning,
    CachePrompt,
    ToolCalls,
    Vision,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewayEvent {
    ProviderSelected {
        provider: ProviderId,
        model: String,
    },
    MessageStart {
        message_id: String,
    },
    TextDelta {
        message_id: String,
        text: String,
    },
    ReasoningDelta {
        message_id: String,
        text: String,
    },
    ToolCallStart {
        message_id: String,
        call_id: String,
        name: String,
    },
    ToolCallArgs {
        call_id: String,
        args_json_delta: String,
    },
    ToolCallEnd {
        call_id: String,
        args_json: String,
    },
    Usage {
        usage: Usage,
    },
    Finish {
        message_id: String,
        finish: FinishReason,
        usage: Usage,
    },
    Error {
        message: String,
        retryable: bool,
        provider: Option<String>,
    },
}

impl GatewayEvent {
    /// Provider selection and usage updates remain available to the gateway ledger;
    /// CoreEvent carries the final usage and only permits provider metadata on errors.
    pub fn into_core(self) -> Result<Option<CoreEvent>, CricketError> {
        Ok(match self {
            Self::ProviderSelected { .. } | Self::Usage { .. } => None,
            Self::MessageStart { message_id } => Some(CoreEvent::MessageStart {
                message_id,
                role: Role::Assistant,
                parent_event_id: None,
            }),
            Self::TextDelta { message_id, text } => Some(CoreEvent::TextDelta { message_id, text }),
            Self::ReasoningDelta { message_id, text } => {
                Some(CoreEvent::ReasoningDelta { message_id, text })
            }
            Self::ToolCallStart {
                message_id,
                call_id,
                name,
            } => Some(CoreEvent::ToolCallStart {
                message_id,
                call_id,
                name,
            }),
            Self::ToolCallArgs {
                call_id,
                args_json_delta,
            } => Some(CoreEvent::ToolCallArgs {
                call_id,
                args_json_delta,
            }),
            Self::ToolCallEnd { call_id, args_json } => {
                Some(CoreEvent::ToolCallEnd { call_id, args_json })
            }
            Self::Finish {
                message_id,
                finish,
                usage,
            } => Some(CoreEvent::MessageEnd {
                message_id,
                finish,
                usage,
            }),
            Self::Error {
                message,
                retryable,
                provider,
            } => Some(CoreEvent::Error {
                message,
                retryable,
                provider,
            }),
        })
    }
}

#[derive(Debug, Clone)]
pub struct NormalizedRequest {
    pub models: ModelPref,
    pub text: String,
    pub system: Option<String>,
    pub max_output_tokens: u32,
    pub tools: Vec<serde_json::Value>,
    pub images: Vec<String>,
}

pub trait ProviderAdapter: Send + Sync {
    fn id(&self) -> Result<ProviderId, CricketError>;
    fn supports(&self, cap: &Capability) -> Result<bool, CricketError>;
    fn normalize_delta(
        &self,
        state: &mut NormalizeState,
        raw: RawSseFrame,
    ) -> Result<Vec<GatewayEvent>, CricketError>;
    fn stream(
        &self,
        req: NormalizedRequest,
    ) -> BoxFuture<'static, Result<GatewayStream, CricketError>>;
}

macro_rules! adapter {
    ($name:ident, $provider:expr) => {
        #[derive(Debug, Default, Clone, Copy)]
        pub struct $name;
        impl ProviderAdapter for $name {
            fn id(&self) -> Result<ProviderId, CricketError> {
                Ok($provider)
            }
            fn supports(&self, cap: &Capability) -> Result<bool, CricketError> {
                Ok(!matches!(cap, Capability::Other(_))
                    && !($provider == ProviderId::Gemini && *cap == Capability::CachePrompt))
            }
            fn normalize_delta(
                &self,
                state: &mut NormalizeState,
                raw: RawSseFrame,
            ) -> Result<Vec<GatewayEvent>, CricketError> {
                state.normalize($provider, raw)
            }
            fn stream(
                &self,
                req: NormalizedRequest,
            ) -> BoxFuture<'static, Result<GatewayStream, CricketError>> {
                Box::pin(stream_request(req))
            }
        }
    };
}
adapter!(OpenAiAdapter, ProviderId::Openai);
adapter!(AnthropicAdapter, ProviderId::Anthropic);
adapter!(GeminiAdapter, ProviderId::Gemini);
adapter!(OpenAiCompatibleAdapter, ProviderId::OpenaiCompatible);

pub(crate) fn adapter_for(provider: ProviderId) -> Result<Box<dyn ProviderAdapter>, CricketError> {
    Ok(match provider {
        ProviderId::Openai => Box::new(OpenAiAdapter),
        ProviderId::Anthropic => Box::new(AnthropicAdapter),
        ProviderId::Gemini => Box::new(GeminiAdapter),
        ProviderId::OpenaiCompatible => Box::new(OpenAiCompatibleAdapter),
        ProviderId::Mock => return Err(CricketError::Protocol("mock requires a fixture".into())),
    })
}

pub(crate) fn provider_name(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Openai => "openai",
        ProviderId::Anthropic => "anthropic",
        ProviderId::Gemini => "gemini",
        ProviderId::OpenaiCompatible => "openai_compatible",
        ProviderId::Mock => "mock",
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UsageLedger {
    pub totals: Usage,
    pub completed_requests: u64,
}

impl UsageLedger {
    pub fn record(&mut self, event: &GatewayEvent) -> Result<(), CricketError> {
        if let GatewayEvent::Finish { usage, .. } = event {
            self.totals.input_tokens = self.totals.input_tokens.saturating_add(usage.input_tokens);
            self.totals.output_tokens = self
                .totals
                .output_tokens
                .saturating_add(usage.output_tokens);
            self.totals.cached_tokens = self
                .totals
                .cached_tokens
                .saturating_add(usage.cached_tokens);
            self.totals.reasoning_tokens = self
                .totals
                .reasoning_tokens
                .saturating_add(usage.reasoning_tokens);
            self.completed_requests = self.completed_requests.saturating_add(1);
        }
        Ok(())
    }
}

pub(crate) fn model_chain(pref: &ModelPref) -> Vec<ModelRef> {
    std::iter::once(pref.primary.clone())
        .chain(pref.fallbacks.clone())
        .collect()
}
