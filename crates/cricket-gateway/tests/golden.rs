use cricket_gateway::{replay, GatewayEvent};
use cricket_protocol::ProviderId;
use futures::TryStreamExt;
use std::path::PathBuf;

fn verify(provider: ProviderId, directory: &str, name: &str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden")
        .join(directory);
    let input = std::fs::read_to_string(root.join(format!("{name}.sse"))).unwrap();
    let expected: Vec<GatewayEvent> = serde_json::from_str(
        &std::fs::read_to_string(root.join(format!("{name}.expected.json"))).unwrap(),
    )
    .unwrap();
    let actual =
        futures::executor::block_on(replay(provider, input).unwrap().try_collect::<Vec<_>>())
            .unwrap();
    assert_eq!(actual, expected, "{directory}/{name}");
}

#[test]
fn openai_text() {
    verify(ProviderId::Openai, "openai", "text");
}

#[test]
fn openai_compatible_uses_same_wire_contract() {
    verify(ProviderId::OpenaiCompatible, "openai", "reasoning");
}

#[test]
fn openai_unicode() {
    verify(ProviderId::Openai, "openai", "unicode");
}

#[test]
fn openai_reasoning() {
    verify(ProviderId::Openai, "openai", "reasoning");
}

#[test]
fn openai_tool() {
    verify(ProviderId::Openai, "openai", "tool");
}

#[test]
fn openai_parallel_tools() {
    verify(ProviderId::Openai, "openai", "parallel_tools");
}

#[test]
fn openai_interrupted() {
    verify(ProviderId::Openai, "openai", "interrupted");
}

#[test]
fn openai_length() {
    verify(ProviderId::Openai, "openai", "length");
}

#[test]
fn openai_filter() {
    verify(ProviderId::Openai, "openai", "filter");
}

#[test]
fn openai_long() {
    verify(ProviderId::Openai, "openai", "long");
}

#[test]
fn openai_usage() {
    verify(ProviderId::Openai, "openai", "usage");
}

#[test]
fn anthropic_text() {
    verify(ProviderId::Anthropic, "anthropic", "text");
}

#[test]
fn anthropic_unicode() {
    verify(ProviderId::Anthropic, "anthropic", "unicode");
}

#[test]
fn anthropic_reasoning() {
    verify(ProviderId::Anthropic, "anthropic", "reasoning");
}

#[test]
fn anthropic_tool() {
    verify(ProviderId::Anthropic, "anthropic", "tool");
}

#[test]
fn anthropic_parallel_tools() {
    verify(ProviderId::Anthropic, "anthropic", "parallel_tools");
}

#[test]
fn anthropic_interrupted() {
    verify(ProviderId::Anthropic, "anthropic", "interrupted");
}

#[test]
fn anthropic_length() {
    verify(ProviderId::Anthropic, "anthropic", "length");
}

#[test]
fn anthropic_filter() {
    verify(ProviderId::Anthropic, "anthropic", "filter");
}

#[test]
fn anthropic_long() {
    verify(ProviderId::Anthropic, "anthropic", "long");
}

#[test]
fn anthropic_usage() {
    verify(ProviderId::Anthropic, "anthropic", "usage");
}

#[test]
fn gemini_text() {
    verify(ProviderId::Gemini, "gemini", "text");
}

#[test]
fn gemini_unicode() {
    verify(ProviderId::Gemini, "gemini", "unicode");
}

#[test]
fn gemini_reasoning() {
    verify(ProviderId::Gemini, "gemini", "reasoning");
}

#[test]
fn gemini_tool() {
    verify(ProviderId::Gemini, "gemini", "tool");
}

#[test]
fn gemini_parallel_tools() {
    verify(ProviderId::Gemini, "gemini", "parallel_tools");
}

#[test]
fn gemini_interrupted() {
    verify(ProviderId::Gemini, "gemini", "interrupted");
}

#[test]
fn gemini_length() {
    verify(ProviderId::Gemini, "gemini", "length");
}

#[test]
fn gemini_filter() {
    verify(ProviderId::Gemini, "gemini", "filter");
}

#[test]
fn gemini_long() {
    verify(ProviderId::Gemini, "gemini", "long");
}

#[test]
fn gemini_usage() {
    verify(ProviderId::Gemini, "gemini", "usage");
}
