use cricket_protocol::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: Value) {
    let first: T = serde_json::from_value(value.clone()).unwrap();
    let serialized = serde_json::to_value(&first).unwrap();
    assert_eq!(serialized, value);
    let second: T = serde_json::from_value(serialized).unwrap();
    assert_eq!(first, second);
}

macro_rules! contract {
    ($name:ident, $ty:ty, $value:expr) => {
        #[test]
        fn $name() {
            round_trip::<$ty>($value);
        }
    };
}

contract!(
    stage,
    Stage,
    json!({"stage":"calling_tool","name":"kb_search"})
);
contract!(role, Role, json!("assistant"));
contract!(finish_reason, FinishReason, json!("tool_calls"));
contract!(task_state, TaskState, json!("repair"));
contract!(agent_mode, AgentMode, json!("engineering"));
contract!(route_mode, RouteMode, json!("mock"));
contract!(provider_id, ProviderId, json!("openai_compatible"));
contract!(attachment_kind, AttachmentKind, json!("image"));
contract!(session_state, SessionState, json!("idle"));
contract!(
    usage,
    Usage,
    json!({"input_tokens":1,"output_tokens":2,"cached_tokens":3,"reasoning_tokens":4})
);
contract!(scope, KbScope, json!({"scope":"agent","agent_id":"a"}));
contract!(
    task,
    TaskInfo,
    json!({"goal_id":"g","task_id":"t","title":"test","state":"ready","attempt":2,"detail":"d"})
);
contract!(
    error_network,
    CricketError,
    json!({"kind":"network","message":"reset"})
);
contract!(
    error_protocol,
    CricketError,
    json!({"kind":"protocol","message":"bad frame"})
);
contract!(
    error_auth,
    CricketError,
    json!({"kind":"auth","message":"denied"})
);
contract!(
    error_session,
    CricketError,
    json!({"kind":"session_not_found","message":"s"})
);
contract!(
    error_tool,
    CricketError,
    json!({"kind":"tool","message":"failed"})
);
contract!(
    error_internal,
    CricketError,
    json!({"kind":"internal","message":"panic"})
);
contract!(error_cancelled, CricketError, json!({"kind":"cancelled"}));
contract!(
    model_ref,
    ModelRef,
    json!({"provider":"openai","model":"gpt-5"})
);
contract!(
    model_pref,
    ModelPref,
    json!({"primary":{"provider":"mock","model":"fixture"},"fallbacks":[],"reasoning_effort":"high","use_cache":false})
);
contract!(
    agent_spec,
    AgentSpec,
    json!({"id":"a","name":"test","description":"d","system_prompt":"s","model_pref":{"primary":{"provider":"mock","model":"fixture"},"fallbacks":[],"use_cache":true},"temperature":0.5,"tool_allowlist":["kb_search"],"skill_allowlist":[],"kb_scopes":[{"scope":"global"}],"mode":"general","max_output_tokens":4096,"config_extra":"{}"})
);
contract!(
    agent_brief,
    AgentBrief,
    json!({"id":"a","name":"test","description":"d","mode":"general","primary_model":"gpt-5","session_count":3,"updated_at_ms":123})
);
contract!(
    session_snapshot,
    SessionSnapshot,
    json!({"session_id":"s","agent_id":"a","title":"t","state":"running","last_event_id":5,"message_count":2,"created_at_ms":10,"updated_at_ms":20,"summary":"sum"})
);
contract!(
    session_brief,
    SessionBrief,
    json!({"session_id":"s","agent_id":"a","title":"t","state":"error","preview":"p","updated_at_ms":20})
);
contract!(
    document_input,
    DocumentInput,
    json!({"id":"d","scope":{"scope":"global"},"title":"t","source":"url","content_md":"# x","tags":["tag"],"kind_hint":"text"})
);
contract!(
    kb_hit,
    KbHit,
    json!({"document_id":"d","chunk_seq":2,"title":"t","heading_path":"a/b","snippet":"<b>x</b>","source":"url","score":0.5})
);
contract!(
    attachment,
    Attachment,
    json!({"id":"att","kind":"code","name":"main.rs","mime":"text/plain","size_bytes":10,"content_text":"fn main() {}","content_url":"url","language":"rust"})
);
contract!(
    core_start,
    CoreEvent,
    json!({"type":"message_start","message_id":"m","role":"assistant","parent_event_id":"1"})
);
contract!(
    core_reasoning,
    CoreEvent,
    json!({"type":"reasoning_delta","message_id":"m","text":"think"})
);
contract!(
    core_text,
    CoreEvent,
    json!({"type":"text_delta","message_id":"m","text":"hello"})
);
contract!(
    core_stage,
    CoreEvent,
    json!({"type":"stage_changed","stage":{"stage":"streaming"}})
);
contract!(
    core_end,
    CoreEvent,
    json!({"type":"message_end","message_id":"m","finish":"stop","usage":{"input_tokens":1,"output_tokens":2,"cached_tokens":0,"reasoning_tokens":0}})
);

#[test]
fn defaults_and_absent_options() {
    let pref: ModelPref =
        serde_json::from_value(json!({"primary":{"provider":"mock","model":"x"}})).unwrap();
    assert_eq!(
        pref,
        ModelPref {
            primary: ModelRef {
                provider: ProviderId::Mock,
                model: "x".into()
            },
            ..ModelPref::default()
        }
    );
    assert_eq!(
        serde_json::to_value(pref).unwrap(),
        json!({"primary":{"provider":"mock","model":"x"},"fallbacks":[],"use_cache":true})
    );
    let session: SessionSnapshot = serde_json::from_value(
        json!({"session_id":"s","agent_id":"a","title":"t","created_at_ms":1,"updated_at_ms":2}),
    )
    .unwrap();
    assert_eq!(
        session,
        SessionSnapshot {
            session_id: "s".into(),
            agent_id: "a".into(),
            title: "t".into(),
            state: SessionState::Idle,
            last_event_id: 0,
            message_count: 0,
            created_at_ms: 1,
            updated_at_ms: 2,
            summary: None
        }
    );
    let start: CoreEvent =
        serde_json::from_value(json!({"type":"message_start","message_id":"m","role":"assistant"}))
            .unwrap();
    assert_eq!(
        start,
        CoreEvent::MessageStart {
            message_id: "m".into(),
            role: Role::Assistant,
            parent_event_id: None
        }
    );
    assert!(serde_json::to_value(start)
        .unwrap()
        .get("parent_event_id")
        .is_none());
}
