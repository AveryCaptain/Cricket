use clap::{Parser, Subcommand, ValueEnum};
use cricket_agent::{Emitter, EventSink};
use cricket_gateway::{
    MockProvider, NormalizedRequest, OpenAiAdapter, ProviderAdapter, UsageLedger,
};
use cricket_protocol::{CoreEvent, CricketError, ModelPref, ModelRef, ProviderId, Stage};
use futures::StreamExt;
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};

#[derive(Parser)]
#[command(name = "cricket-cli")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Chat {
        #[arg(long, value_enum, default_value = "mock")]
        provider: Provider,
        #[arg(long, required_if_eq("provider", "mock"), conflicts_with = "live")]
        fixture: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        live: bool,
        #[arg(long)]
        model: Option<String>,
        #[arg(long, default_value = "Hello Cricket")]
        text: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
enum Provider {
    Mock,
    Openai,
    Anthropic,
    Gemini,
    OpenaiCompatible,
}

impl Provider {
    fn id(self) -> ProviderId {
        match self {
            Self::Mock => ProviderId::Mock,
            Self::Openai => ProviderId::Openai,
            Self::Anthropic => ProviderId::Anthropic,
            Self::Gemini => ProviderId::Gemini,
            Self::OpenaiCompatible => ProviderId::OpenaiCompatible,
        }
    }
}

struct Stdout;
impl EventSink for Stdout {
    fn emit(&self, event: CoreEvent) -> Result<(), CricketError> {
        let mut out = io::stdout().lock();
        serde_json::to_writer(&mut out, &event)
            .map_err(|e| CricketError::Internal(e.to_string()))?;
        writeln!(out).map_err(|e| CricketError::Internal(e.to_string()))?;
        out.flush()
            .map_err(|e| CricketError::Internal(e.to_string()))
    }
}

fn fixture_provider(path: &std::path::Path) -> Result<ProviderId, CricketError> {
    match path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
    {
        Some("openai") => Ok(ProviderId::Openai),
        Some("anthropic") => Ok(ProviderId::Anthropic),
        Some("gemini") => Ok(ProviderId::Gemini),
        _ => Err(CricketError::Protocol(
            "fixture must be in an openai, anthropic, or gemini directory".into(),
        )),
    }
}

async fn run(cli: Cli) -> Result<(), CricketError> {
    let Command::Chat {
        provider,
        fixture,
        live,
        model,
        text,
    } = cli.command;
    let mut stream = if provider == Provider::Mock {
        let path =
            fixture.ok_or_else(|| CricketError::Protocol("mock requires --fixture".into()))?;
        if live {
            return Err(CricketError::Protocol("mock cannot be live".into()));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| CricketError::Protocol(format!("cannot read fixture: {e}")))?;
        MockProvider {
            provider: fixture_provider(&path)?,
            fixture: content,
        }
        .stream()?
    } else {
        if !live {
            return Err(CricketError::Auth(
                "network access requires explicit --live".into(),
            ));
        }
        let model =
            model.ok_or_else(|| CricketError::Protocol("live requires explicit --model".into()))?;
        let req = NormalizedRequest {
            models: ModelPref {
                primary: ModelRef {
                    provider: provider.id(),
                    model,
                },
                ..ModelPref::default()
            },
            text,
            system: None,
            max_output_tokens: 4096,
            tools: vec![],
            images: vec![],
        };
        OpenAiAdapter.stream(req).await?
    };
    let emitter = Emitter::new(vec![Arc::new(Stdout)])?;
    emitter
        .emit(CoreEvent::SessionStarted {
            session_id: "cli-session".into(),
        })
        .await?;
    emitter
        .emit(CoreEvent::StageChanged {
            stage: Stage::Connecting,
        })
        .await?;
    let mut ledger = UsageLedger::default();
    let mut failure = None;
    while let Some(event) = stream.next().await {
        match event {
            Ok(event) => {
                if let cricket_gateway::GatewayEvent::Error { message, .. } = &event {
                    failure = Some(CricketError::Network(message.clone()));
                }
                ledger.record(&event)?;
                if let Some(event) = event.into_core()? {
                    emitter.emit(event).await?;
                }
            }
            Err(e) => {
                emitter
                    .emit(CoreEvent::Error {
                        message: e.to_string(),
                        retryable: true,
                        provider: Some(format!("{:?}", provider.id()).to_lowercase()),
                    })
                    .await?;
                failure = Some(e);
                break;
            }
        }
    }
    emitter.emit(CoreEvent::StreamClosed).await?;
    emitter.close().await?;
    failure.map_or(Ok(()), Err)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}
