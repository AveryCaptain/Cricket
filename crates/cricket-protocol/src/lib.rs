//! 事件/类型/错误契约，三端唯一真源

use serde::{Deserialize, Serialize};

#[cfg(feature = "ffi")]
uniffi::setup_scaffolding!();

macro_rules! unit_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}

unit_enum!(Role {
    System,
    User,
    Assistant,
    Tool
});
unit_enum!(FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error,
    Aborted
});
unit_enum!(TaskState {
    Pending,
    Ready,
    Running,
    Verifying,
    Repair,
    Blocked,
    Failed,
    Done,
    Skipped
});
unit_enum!(AgentMode {
    General,
    Novel,
    Engineering
});
unit_enum!(RouteMode {
    Relay,
    Direct,
    Mock
});
unit_enum!(ProviderId {
    Openai,
    Anthropic,
    Gemini,
    OpenaiCompatible,
    Mock
});
unit_enum!(AttachmentKind {
    Text,
    Code,
    Image,
    File
});

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    #[default]
    Idle,
    Running,
    Aborted,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Stage {
    Connecting,
    Planning,
    RetrievingKnowledge,
    Reasoning,
    CallingTool { name: String },
    Executing,
    Verifying,
    Streaming,
    Done,
    Aborted,
    Failed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreEvent {
    SessionStarted {
        session_id: String,
    },
    StageChanged {
        stage: Stage,
    },
    MessageStart {
        message_id: String,
        role: Role,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_event_id: Option<String>,
    },
    ReasoningDelta {
        message_id: String,
        text: String,
    },
    TextDelta {
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
    ToolResult {
        call_id: String,
        ok: bool,
        duration_ms: u64,
        preview: String,
    },
    MessageEnd {
        message_id: String,
        finish: FinishReason,
        usage: Usage,
    },
    TaskUpdated {
        task: TaskInfo,
    },
    KbChanged {
        scope: KbScope,
    },
    Error {
        message: String,
        retryable: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider: Option<String>,
    },
    StreamClosed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum KbScope {
    Global,
    Agent { agent_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct TaskInfo {
    pub goal_id: String,
    pub task_id: String,
    pub title: String,
    pub state: TaskState,
    pub attempt: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Error))]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum CricketError {
    #[error("网络错误: {0}")]
    Network(String),
    #[error("协议错误: {0}")]
    Protocol(String),
    #[error("鉴权失败: {0}")]
    Auth(String),
    #[error("会话不存在: {0}")]
    SessionNotFound(String),
    #[error("工具执行失败: {0}")]
    Tool(String),
    #[error("已取消: {0}")]
    Cancelled(String),
    #[error("内部错误: {0}")]
    Internal(String),
}

fn default_cache() -> bool {
    true
}
fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u32 {
    4096
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct ModelRef {
    pub provider: ProviderId,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct ModelPref {
    pub primary: ModelRef,
    #[serde(default)]
    pub fallbacks: Vec<ModelRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(default = "default_cache")]
    pub use_cache: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct AgentSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub model_pref: ModelPref,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub tool_allowlist: Vec<String>,
    #[serde(default)]
    pub skill_allowlist: Vec<String>,
    #[serde(default)]
    pub kb_scopes: Vec<KbScope>,
    pub mode: AgentMode,
    #[serde(default = "default_max_tokens")]
    pub max_output_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_extra: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct AgentBrief {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mode: AgentMode,
    pub primary_model: String,
    #[serde(default)]
    pub session_count: u32,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct SessionSnapshot {
    pub session_id: String,
    pub agent_id: String,
    pub title: String,
    #[serde(default)]
    pub state: SessionState,
    #[serde(default)]
    pub last_event_id: i64,
    #[serde(default)]
    pub message_count: u32,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct SessionBrief {
    pub session_id: String,
    pub agent_id: String,
    pub title: String,
    #[serde(default)]
    pub state: SessionState,
    pub preview: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct DocumentInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub scope: KbScope,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub content_md: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct KbHit {
    pub document_id: String,
    pub chunk_seq: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading_path: Option<String>,
    pub snippet: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(rename_all = "snake_case")]
pub struct Attachment {
    pub id: String,
    pub kind: AttachmentKind,
    pub name: String,
    pub mime: String,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

impl Default for ModelPref {
    fn default() -> Self {
        Self {
            primary: ModelRef {
                provider: ProviderId::Mock,
                model: String::new(),
            },
            fallbacks: Vec::new(),
            reasoning_effort: None,
            use_cache: true,
        }
    }
}
