use cricket_protocol::{CoreEvent, FinishReason};
use std::{path::PathBuf, process::Command};

#[test]
fn mock_pipeline_prints_ordered_core_events() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden/openai/reasoning.sse");
    let output = Command::new(env!("CARGO_BIN_EXE_cricket-cli"))
        .args(["chat", "--provider", "mock", "--fixture"])
        .arg(fixture)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<CoreEvent>(line).unwrap())
        .collect::<Vec<_>>();
    assert!(matches!(
        events.first(),
        Some(CoreEvent::SessionStarted { .. })
    ));
    assert!(events.iter().any(|e|matches!(e,CoreEvent::ReasoningDelta {text,..} if text=="Compare evidence before answering.")));
    assert!(events.iter().any(|e| matches!(
        e,
        CoreEvent::MessageEnd {
            finish: FinishReason::Stop,
            ..
        }
    )));
    assert_eq!(events.last(), Some(&CoreEvent::StreamClosed));
}

#[test]
fn real_provider_requires_explicit_live() {
    let output = Command::new(env!("CARGO_BIN_EXE_cricket-cli"))
        .args(["chat", "--provider", "openai", "--model", "fixture"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires explicit --live"));
}

#[test]
fn interruption_emits_error_and_returns_failure() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden/anthropic/interrupted.sse");
    let output = Command::new(env!("CARGO_BIN_EXE_cricket-cli"))
        .args(["chat", "--provider", "mock", "--fixture"])
        .arg(fixture)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let events = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<CoreEvent>(line).unwrap())
        .collect::<Vec<_>>();
    assert!(events.iter().any(|e| matches!(
        e,
        CoreEvent::Error {
            retryable: true,
            ..
        }
    )));
    assert_eq!(events.last(), Some(&CoreEvent::StreamClosed));
}
