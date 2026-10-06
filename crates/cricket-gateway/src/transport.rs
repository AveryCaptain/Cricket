use crate::{
    adapter_for, model_chain, provider_name, Capability, GatewayEvent, GatewayStream,
    NormalizeState, NormalizedRequest, RawSseFrame,
};
use cricket_protocol::{CricketError, ModelRef, ProviderId};
use eventsource_stream::Eventsource;
use futures::{StreamExt, TryStreamExt};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

type Frames = futures::stream::BoxStream<'static, Result<eventsource_stream::Event, CricketError>>;

fn normalized_stream(provider: ProviderId, frames: Frames) -> GatewayStream {
    let state = (frames, NormalizeState::default(), VecDeque::new(), false);
    futures::stream::unfold(
        state,
        move |(mut frames, mut normalize, mut queued, mut ended)| async move {
            loop {
                if let Some(item) = queued.pop_front() {
                    return Some((item, (frames, normalize, queued, ended)));
                }
                if ended {
                    return None;
                }
                match frames.next().await {
                    Some(Ok(frame)) => match adapter_for(provider).and_then(|a| {
                        a.normalize_delta(
                            &mut normalize,
                            RawSseFrame {
                                event: frame.event,
                                data: frame.data,
                            },
                        )
                    }) {
                        Ok(events) => queued.extend(events.into_iter().map(Ok)),
                        Err(e) => {
                            queued.push_back(Err(e));
                            ended = true;
                        }
                    },
                    Some(Err(e)) => {
                        queued.push_back(Err(e));
                        ended = true;
                    }
                    None => {
                        match normalize.eof(provider) {
                            Ok(events) => queued.extend(events.into_iter().map(Ok)),
                            Err(e) => queued.push_back(Err(e)),
                        };
                        ended = true;
                    }
                }
            }
        },
    )
    .boxed()
}

pub fn replay(provider: ProviderId, fixture: String) -> Result<GatewayStream, CricketError> {
    adapter_for(provider)?;
    // Exercise the same eventsource parser as HTTP, including arbitrary UTF-8 chunk boundaries.
    let chunks = fixture
        .into_bytes()
        .chunks(7)
        .map(|c| Ok::<_, std::io::Error>(c.to_vec()))
        .collect::<Vec<_>>();
    let frames = futures::stream::iter(chunks)
        .eventsource()
        .map_err(|e| CricketError::Protocol(e.to_string()))
        .boxed();
    Ok(normalized_stream(provider, frames))
}

#[derive(Debug, Clone)]
pub struct MockProvider {
    pub provider: ProviderId,
    pub fixture: String,
}

impl MockProvider {
    pub fn stream(&self) -> Result<GatewayStream, CricketError> {
        replay(self.provider, self.fixture.clone())
    }
}

pub(crate) fn request_body(
    req: &NormalizedRequest,
    model: &ModelRef,
) -> Result<Value, CricketError> {
    let adapter = adapter_for(model.provider)?;
    let tools = if adapter.supports(&Capability::ToolCalls)? {
        req.tools.clone()
    } else {
        Vec::new()
    };
    let reasoning = if adapter.supports(&Capability::Reasoning)? {
        req.models.reasoning_effort.as_deref()
    } else {
        None
    };
    let cache = req.models.use_cache && adapter.supports(&Capability::CachePrompt)?;
    let mut body = match model.provider {
        ProviderId::Openai | ProviderId::OpenaiCompatible => {
            let mut messages = Vec::new();
            if let Some(system) = &req.system {
                messages.push(json!({"role":"system","content":system}));
            }
            let mut content = vec![json!({"type":"text","text":req.text})];
            if adapter.supports(&Capability::Vision)? {
                content.extend(
                    req.images
                        .iter()
                        .map(|url| json!({"type":"image_url","image_url":{"url":url}})),
                );
            }
            messages.push(json!({"role":"user","content":content}));
            json!({"model":model.model,"messages":messages,"stream":true,"stream_options":{"include_usage":true}})
        }
        ProviderId::Anthropic => {
            let mut content = vec![json!({"type":"text","text":req.text})];
            if cache {
                content[0]["cache_control"] = json!({"type":"ephemeral"});
            }
            if adapter.supports(&Capability::Vision)? {
                content.extend(
                    req.images
                        .iter()
                        .map(|url| json!({"type":"image","source":{"type":"url","url":url}})),
                );
            }
            let mut v = json!({"model":model.model,"messages":[{"role":"user","content":content}],"stream":true,"max_tokens":req.max_output_tokens});
            if let Some(system) = &req.system {
                v["system"] = json!(system);
            }
            v
        }
        ProviderId::Gemini => {
            let mut parts = vec![json!({"text":req.text})];
            if adapter.supports(&Capability::Vision)? {
                parts.extend(
                    req.images
                        .iter()
                        .map(|url| json!({"fileData":{"mimeType":"image/png","fileUri":url}})),
                );
            }
            let mut v = json!({"contents":[{"role":"user","parts":parts}],"generationConfig":{"maxOutputTokens":req.max_output_tokens}});
            if let Some(system) = &req.system {
                v["systemInstruction"] = json!({"parts":[{"text":system}]});
            }
            v
        }
        ProviderId::Mock => return Err(CricketError::Protocol("mock requires fixture".into())),
    };
    if !tools.is_empty() {
        body["tools"]=match model.provider {
            ProviderId::Openai | ProviderId::OpenaiCompatible=>json!(tools.iter().map(|t|json!({"type":"function","function":t})).collect::<Vec<_>>()),
            ProviderId::Anthropic=>json!(tools.iter().map(|t|json!({"name":t["name"],"description":t["description"],"input_schema":t["parameters"]})).collect::<Vec<_>>()),
            _=>json!([{"functionDeclarations":tools}]),
        };
    }
    if let Some(effort) = reasoning {
        match model.provider {
            ProviderId::Openai | ProviderId::OpenaiCompatible => {
                body["reasoning_effort"] = json!(effort)
            }
            ProviderId::Anthropic => {
                body["thinking"] = json!({"type":"enabled","budget_tokens":1024})
            }
            ProviderId::Gemini => {
                body["generationConfig"]["thinkingConfig"] = json!({"includeThoughts":true})
            }
            _ => {}
        }
    }
    if matches!(
        model.provider,
        ProviderId::Openai | ProviderId::OpenaiCompatible
    ) && req.max_output_tokens > 0
    {
        body["max_completion_tokens"] = json!(req.max_output_tokens);
    }
    Ok(body)
}

fn endpoint(model: &ModelRef) -> Result<(String, String), CricketError> {
    let (key_var, url_var, default_url) = match model.provider {
        ProviderId::Openai => (
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "https://api.openai.com/v1",
        ),
        ProviderId::OpenaiCompatible => (
            "OPENAI_COMPATIBLE_API_KEY",
            "OPENAI_COMPATIBLE_BASE_URL",
            "",
        ),
        ProviderId::Anthropic => (
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_BASE_URL",
            "https://api.anthropic.com/v1",
        ),
        ProviderId::Gemini => (
            "GEMINI_API_KEY",
            "GEMINI_BASE_URL",
            "https://generativelanguage.googleapis.com/v1beta",
        ),
        ProviderId::Mock => {
            return Err(CricketError::Protocol("mock is not a live provider".into()))
        }
    };
    let key =
        std::env::var(key_var).map_err(|_| CricketError::Auth(format!("missing {key_var}")))?;
    let base = std::env::var(url_var).unwrap_or_else(|_| default_url.into());
    let url = match model.provider {
        ProviderId::Anthropic => format!("{}/messages", base.trim_end_matches('/')),
        ProviderId::Gemini => format!(
            "{}/models/{}:streamGenerateContent?alt=sse",
            base.trim_end_matches('/'),
            model.model
        ),
        _ => format!("{}/chat/completions", base.trim_end_matches('/')),
    };
    Ok((url, key))
}

fn delay(attempt: u32) -> Duration {
    let jitter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64 % 101);
    Duration::from_millis((250_u64 << attempt) + jitter)
}

pub async fn stream_request(req: NormalizedRequest) -> Result<GatewayStream, CricketError> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| CricketError::Network("HTTP client initialization failed".into()))?;
    let mut last = CricketError::Network("no available model".into());
    for model in model_chain(&req.models) {
        let (url, key) = match endpoint(&model) {
            Ok(x) => x,
            Err(e) => {
                last = e;
                continue;
            }
        };
        let body = request_body(&req, &model)?;
        for attempt in 0..=3 {
            let mut request = client
                .post(&url)
                .header("content-type", "application/json")
                .body(body.to_string());
            request = match model.provider {
                ProviderId::Anthropic => request
                    .header("x-api-key", &key)
                    .header("anthropic-version", "2023-06-01"),
                ProviderId::Gemini => request.header("x-goog-api-key", &key),
                _ => request.bearer_auth(&key),
            };
            let retry = match request.send().await {
                Ok(response) if response.status().is_success() => {
                    let frames = response
                        .bytes_stream()
                        .eventsource()
                        .map_err(|_| CricketError::Network("upstream stream interrupted".into()))
                        .boxed();
                    let selected = GatewayEvent::ProviderSelected {
                        provider: model.provider,
                        model: model.model,
                    };
                    return Ok(futures::stream::once(async move { Ok(selected) })
                        .chain(normalized_stream(model.provider, frames))
                        .boxed());
                }
                Ok(response) => {
                    let status = response.status();
                    last = if status.as_u16() == 401 || status.as_u16() == 403 {
                        CricketError::Auth(format!(
                            "{} HTTP {}",
                            provider_name(model.provider),
                            status.as_u16()
                        ))
                    } else {
                        CricketError::Network(format!(
                            "{} HTTP {}",
                            provider_name(model.provider),
                            status.as_u16()
                        ))
                    };
                    status.as_u16() == 429 || status.is_server_error()
                }
                Err(_) => {
                    last = CricketError::Network(format!(
                        "{} connection failed",
                        provider_name(model.provider)
                    ));
                    true
                }
            };
            if !retry || attempt == 3 {
                break;
            }
            tokio::time::sleep(delay(attempt)).await;
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GatewayEvent, UsageLedger};
    use cricket_protocol::{FinishReason, ModelPref, Usage};
    #[test]
    fn retry_delay_is_bounded() {
        for attempt in 0..3 {
            let ms = delay(attempt).as_millis();
            assert!((250_u128 << attempt) <= ms && ms <= (250_u128 << attempt) + 100);
        }
    }
    #[test]
    fn fallback_order() {
        let p = ModelPref {
            primary: ModelRef {
                provider: ProviderId::Openai,
                model: "a".into(),
            },
            fallbacks: vec![ModelRef {
                provider: ProviderId::Anthropic,
                model: "b".into(),
            }],
            ..ModelPref::default()
        };
        assert_eq!(
            model_chain(&p)
                .iter()
                .map(|m| m.model.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }
    #[test]
    fn ledger_counts_final_usage_once_not_snapshots() {
        let mut l = UsageLedger::default();
        let u = Usage {
            input_tokens: 2,
            output_tokens: 3,
            cached_tokens: 1,
            reasoning_tokens: 1,
        };
        l.record(&GatewayEvent::Usage { usage: u.clone() }).unwrap();
        l.record(&GatewayEvent::Finish {
            message_id: "m".into(),
            finish: FinishReason::Stop,
            usage: u.clone(),
        })
        .unwrap();
        assert_eq!(l.totals, u);
        assert_eq!(l.completed_requests, 1);
    }
    #[test]
    fn disabled_cache_omits_anthropic_markers() {
        let req = NormalizedRequest {
            models: ModelPref {
                use_cache: false,
                ..ModelPref::default()
            },
            text: "x".into(),
            system: None,
            max_output_tokens: 2048,
            tools: vec![],
            images: vec![],
        };
        let model = ModelRef {
            provider: ProviderId::Anthropic,
            model: "claude".into(),
        };
        assert!(
            request_body(&req, &model).unwrap()["messages"][0]["content"][0]
                .get("cache_control")
                .is_none()
        );
    }
}
