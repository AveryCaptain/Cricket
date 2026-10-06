use crate::{provider_name, GatewayEvent};
use cricket_protocol::{CricketError, FinishReason, ProviderId, Usage};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct RawSseFrame {
    pub event: String,
    pub data: String,
}

#[derive(Debug, Default)]
struct Call {
    id: String,
    name: String,
    args: String,
}

#[derive(Debug, Default)]
pub struct NormalizeState {
    message_id: String,
    started: bool,
    terminal: bool,
    finish: Option<FinishReason>,
    usage: Usage,
    calls: BTreeMap<u64, Call>,
}

fn protocol(message: impl Into<String>) -> CricketError {
    CricketError::Protocol(message.into())
}
fn str_at<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}
fn count(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or_default()
}
fn finish_reason(value: &str) -> Result<FinishReason, CricketError> {
    Ok(match value {
        "stop" | "end_turn" | "stop_sequence" | "STOP" => FinishReason::Stop,
        "length" | "max_tokens" | "MAX_TOKENS" => FinishReason::Length,
        "tool_calls" | "tool_use" => FinishReason::ToolCalls,
        "content_filter" | "refusal" | "SAFETY" | "RECITATION" | "BLOCKLIST"
        | "PROHIBITED_CONTENT" => FinishReason::ContentFilter,
        "error" | "ERROR" | "MALFORMED_FUNCTION_CALL" => FinishReason::Error,
        other => return Err(protocol(format!("unknown finish reason: {other}"))),
    })
}

impl NormalizeState {
    pub fn normalize(
        &mut self,
        provider: ProviderId,
        raw: RawSseFrame,
    ) -> Result<Vec<GatewayEvent>, CricketError> {
        if self.terminal {
            return Err(protocol("received data after terminal frame"));
        }
        let mut out = Vec::new();
        if raw.data == "[DONE]" {
            if self.finish.is_none() {
                return Err(protocol("DONE without finish reason"));
            }
            self.end(&mut out)?;
            return Ok(out);
        }
        let v: Value = serde_json::from_str(&raw.data).map_err(|e| protocol(e.to_string()))?;
        if v.get("error").is_some() || raw.event == "error" || str_at(&v, "type") == "error" {
            self.terminal = true;
            let message = v
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("provider stream error");
            out.push(GatewayEvent::Error {
                message: message.into(),
                retryable: true,
                provider: Some(provider_name(provider).into()),
            });
            return Ok(out);
        }
        match provider {
            ProviderId::Openai | ProviderId::OpenaiCompatible => self.openai(&v, &mut out)?,
            ProviderId::Anthropic => self.anthropic(&v, &raw.event, &mut out)?,
            ProviderId::Gemini => self.gemini(&v, &mut out)?,
            ProviderId::Mock => {
                return Err(protocol("mock normalization requires a native provider"))
            }
        }
        Ok(out)
    }

    pub fn eof(&mut self, provider: ProviderId) -> Result<Vec<GatewayEvent>, CricketError> {
        let mut out = Vec::new();
        if !self.terminal {
            if self.finish.is_some() {
                self.end(&mut out)?;
            } else {
                self.terminal = true;
                out.push(GatewayEvent::Error {
                    message: "upstream interrupted before finish".into(),
                    retryable: true,
                    provider: Some(provider_name(provider).into()),
                });
            }
        }
        Ok(out)
    }

    fn start(&mut self, id: &str, out: &mut Vec<GatewayEvent>) {
        if !self.started {
            self.message_id = if id.is_empty() {
                "upstream-message".into()
            } else {
                id.into()
            };
            self.started = true;
            out.push(GatewayEvent::MessageStart {
                message_id: self.message_id.clone(),
            });
        }
    }

    fn delta(&self, text: &str, reasoning: bool, out: &mut Vec<GatewayEvent>) {
        if text.is_empty() {
            return;
        }
        out.push(if reasoning {
            GatewayEvent::ReasoningDelta {
                message_id: self.message_id.clone(),
                text: text.into(),
            }
        } else {
            GatewayEvent::TextDelta {
                message_id: self.message_id.clone(),
                text: text.into(),
            }
        });
    }

    fn call(
        &mut self,
        index: u64,
        id: &str,
        name: &str,
        args: &str,
        out: &mut Vec<GatewayEvent>,
    ) -> Result<(), CricketError> {
        if !self.calls.contains_key(&index) {
            if id.is_empty() || name.is_empty() {
                return Err(protocol("tool first frame requires id/name"));
            }
            self.calls.insert(
                index,
                Call {
                    id: id.into(),
                    name: name.into(),
                    args: String::new(),
                },
            );
            out.push(GatewayEvent::ToolCallStart {
                message_id: self.message_id.clone(),
                call_id: id.into(),
                name: name.into(),
            });
        }
        let call = self
            .calls
            .get_mut(&index)
            .ok_or_else(|| protocol("unknown tool index"))?;
        if (!id.is_empty() && id != call.id) || (!name.is_empty() && name != call.name) {
            return Err(protocol("tool identity changed"));
        }
        if !args.is_empty() {
            call.args.push_str(args);
            out.push(GatewayEvent::ToolCallArgs {
                call_id: call.id.clone(),
                args_json_delta: args.into(),
            });
        }
        Ok(())
    }

    fn end_call(&mut self, index: u64, out: &mut Vec<GatewayEvent>) -> Result<(), CricketError> {
        if let Some(call) = self.calls.remove(&index) {
            let args = if call.args.is_empty() {
                "{}".to_owned()
            } else {
                call.args
            };
            let value: Value = serde_json::from_str(&args)
                .map_err(|e| protocol(format!("invalid completed tool arguments: {e}")))?;
            if !value.is_object() {
                return Err(protocol("tool arguments must be an object"));
            }
            out.push(GatewayEvent::ToolCallEnd {
                call_id: call.id,
                args_json: args,
            });
        }
        Ok(())
    }

    fn end(&mut self, out: &mut Vec<GatewayEvent>) -> Result<(), CricketError> {
        let finish = self.finish.ok_or_else(|| protocol("missing finish"))?;
        for index in self.calls.keys().copied().collect::<Vec<_>>() {
            self.end_call(index, out)?;
        }
        self.terminal = true;
        out.push(GatewayEvent::Finish {
            message_id: self.message_id.clone(),
            finish,
            usage: self.usage.clone(),
        });
        Ok(())
    }

    fn openai(&mut self, v: &Value, out: &mut Vec<GatewayEvent>) -> Result<(), CricketError> {
        let choices = v
            .get("choices")
            .and_then(Value::as_array)
            .ok_or_else(|| protocol("missing OpenAI choices"))?;
        if let Some(choice) = choices.first() {
            self.start(str_at(v, "id"), out);
            let delta = &choice["delta"];
            self.delta(str_at(delta, "reasoning_content"), true, out);
            self.delta(str_at(delta, "content"), false, out);
            if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
                for call in calls {
                    let index = call
                        .get("index")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| protocol("tool requires index"))?;
                    self.call(
                        index,
                        str_at(call, "id"),
                        str_at(&call["function"], "name"),
                        str_at(&call["function"], "arguments"),
                        out,
                    )?;
                }
            }
            if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                self.finish = Some(finish_reason(reason)?);
            }
        }
        if let Some(u) = v.get("usage").filter(|u| u.is_object()) {
            self.usage = Usage {
                input_tokens: count(u, "prompt_tokens"),
                output_tokens: count(u, "completion_tokens"),
                cached_tokens: count(&u["prompt_tokens_details"], "cached_tokens"),
                reasoning_tokens: count(&u["completion_tokens_details"], "reasoning_tokens"),
            };
            out.push(GatewayEvent::Usage {
                usage: self.usage.clone(),
            });
        }
        Ok(())
    }

    fn anthropic(
        &mut self,
        v: &Value,
        event: &str,
        out: &mut Vec<GatewayEvent>,
    ) -> Result<(), CricketError> {
        let ty = str_at(v, "type");
        if !event.is_empty() && event != "message" && event != ty {
            return Err(protocol("Anthropic event/data type mismatch"));
        }
        match ty {
            "ping" => {}
            "message_start" => {
                self.start(str_at(&v["message"], "id"), out);
                let u = &v["message"]["usage"];
                self.usage.input_tokens = count(u, "input_tokens");
                self.usage.cached_tokens = count(u, "cache_read_input_tokens");
                out.push(GatewayEvent::Usage {
                    usage: self.usage.clone(),
                });
            }
            "content_block_start" => {
                if !self.started {
                    return Err(protocol("block before message_start"));
                }
                let block = &v["content_block"];
                match str_at(block, "type") {
                    "tool_use" => {
                        let input = block
                            .get("input")
                            .filter(|v| v.as_object().is_some_and(|o| !o.is_empty()))
                            .map(Value::to_string)
                            .unwrap_or_default();
                        self.call(
                            count(v, "index"),
                            str_at(block, "id"),
                            str_at(block, "name"),
                            &input,
                            out,
                        )?;
                    }
                    "text" => self.delta(str_at(block, "text"), false, out),
                    "thinking" => self.delta(str_at(block, "thinking"), true, out),
                    "redacted_thinking" => {}
                    _ => return Err(protocol("unknown Anthropic content block")),
                }
            }
            "content_block_delta" => {
                if !self.started {
                    return Err(protocol("delta before message_start"));
                }
                let delta = &v["delta"];
                match str_at(delta, "type") {
                    "text_delta" => self.delta(str_at(delta, "text"), false, out),
                    "thinking_delta" => self.delta(str_at(delta, "thinking"), true, out),
                    "signature_delta" => {}
                    "input_json_delta" => self.call(
                        count(v, "index"),
                        "",
                        "",
                        str_at(delta, "partial_json"),
                        out,
                    )?,
                    _ => return Err(protocol("unknown Anthropic delta")),
                }
            }
            "content_block_stop" => self.end_call(count(v, "index"), out)?,
            "message_delta" => {
                if let Some(reason) = v["delta"].get("stop_reason").and_then(Value::as_str) {
                    self.finish = Some(finish_reason(reason)?);
                }
                self.usage.output_tokens = count(&v["usage"], "output_tokens");
                out.push(GatewayEvent::Usage {
                    usage: self.usage.clone(),
                });
            }
            "message_stop" => self.end(out)?,
            _ => return Err(protocol("unknown Anthropic frame")),
        }
        Ok(())
    }

    fn gemini(&mut self, v: &Value, out: &mut Vec<GatewayEvent>) -> Result<(), CricketError> {
        if let Some(candidates) = v.get("candidates").and_then(Value::as_array) {
            if let Some(candidate) = candidates.first() {
                self.start(str_at(v, "responseId"), out);
                if let Some(parts) = candidate["content"].get("parts").and_then(Value::as_array) {
                    for part in parts {
                        self.delta(
                            str_at(part, "text"),
                            part["thought"].as_bool() == Some(true),
                            out,
                        );
                        if let Some(call) = part.get("functionCall") {
                            let index = self.calls.keys().next_back().map_or(0, |i| i + 1);
                            let id = call
                                .get("id")
                                .and_then(Value::as_str)
                                .map(str::to_owned)
                                .unwrap_or_else(|| format!("{}-call-{index}", self.message_id));
                            let args = call
                                .get("args")
                                .ok_or_else(|| protocol("Gemini functionCall requires args"))?
                                .to_string();
                            self.call(index, &id, str_at(call, "name"), &args, out)?;
                        }
                    }
                }
                if let Some(reason) = candidate.get("finishReason").and_then(Value::as_str) {
                    self.finish = Some(if reason == "STOP" && !self.calls.is_empty() {
                        FinishReason::ToolCalls
                    } else {
                        finish_reason(reason)?
                    });
                }
            }
        } else if v.get("usageMetadata").is_none() {
            return Err(protocol("missing Gemini candidates"));
        }
        if let Some(u) = v.get("usageMetadata") {
            self.usage = Usage {
                input_tokens: count(u, "promptTokenCount"),
                output_tokens: count(u, "candidatesTokenCount"),
                cached_tokens: count(u, "cachedContentTokenCount"),
                reasoning_tokens: count(u, "thoughtsTokenCount"),
            };
            out.push(GatewayEvent::Usage {
                usage: self.usage.clone(),
            });
        }
        if self.finish.is_some() {
            self.end(out)?;
        }
        Ok(())
    }
}
