use cricket_protocol::{CoreEvent, CricketError};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, oneshot},
    time::{sleep_until, Instant},
};

const FRAME_WINDOW: Duration = Duration::from_millis(16);

pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: CoreEvent) -> Result<(), CricketError>;
}

enum Command {
    Emit(CoreEvent, oneshot::Sender<Result<(), CricketError>>),
    Flush(oneshot::Sender<Result<(), CricketError>>),
    Close(oneshot::Sender<Result<(), CricketError>>),
}

#[derive(Clone)]
pub struct Emitter {
    tx: mpsc::Sender<Command>,
}

fn closed() -> CricketError {
    CricketError::Internal("emitter closed".into())
}

impl Emitter {
    pub fn new(sinks: Vec<Arc<dyn EventSink>>) -> Result<Self, CricketError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| CricketError::Internal("Emitter requires a Tokio runtime".into()))?;
        let (tx, rx) = mpsc::channel(128);
        runtime.spawn(run(rx, sinks));
        Ok(Self { tx })
    }

    pub async fn emit(&self, event: CoreEvent) -> Result<(), CricketError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Emit(event, tx))
            .await
            .map_err(|_| closed())?;
        rx.await.map_err(|_| closed())?
    }

    pub async fn flush(&self) -> Result<(), CricketError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Flush(tx))
            .await
            .map_err(|_| closed())?;
        rx.await.map_err(|_| closed())?
    }

    pub async fn close(&self) -> Result<(), CricketError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Close(tx))
            .await
            .map_err(|_| closed())?;
        rx.await.map_err(|_| closed())?
    }
}

fn fanout(event: CoreEvent, sinks: &[Arc<dyn EventSink>]) -> Result<(), CricketError> {
    let mut failure = None;
    for sink in sinks {
        // External sink implementations must not unwind through the emission task.
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sink.emit(event.clone())))
                .unwrap_or_else(|_| Err(CricketError::Internal("event sink panicked".into())));
        if let Err(e) = result {
            if failure.is_none() {
                failure = Some(e);
            }
        }
    }
    failure.map_or(Ok(()), Err)
}

fn flush_pending(
    pending: &mut Vec<CoreEvent>,
    sinks: &[Arc<dyn EventSink>],
) -> Result<(), CricketError> {
    for event in pending.drain(..) {
        fanout(event, sinks)?;
    }
    Ok(())
}

fn append(pending: &mut Vec<CoreEvent>, event: CoreEvent) {
    match (pending.last_mut(), &event) {
        (
            Some(CoreEvent::TextDelta { message_id, text }),
            CoreEvent::TextDelta {
                message_id: next,
                text: delta,
            },
        )
        | (
            Some(CoreEvent::ReasoningDelta { message_id, text }),
            CoreEvent::ReasoningDelta {
                message_id: next,
                text: delta,
            },
        ) if message_id == next => {
            text.push_str(delta);
            return;
        }
        _ => {}
    }
    pending.push(event);
}

async fn run(mut rx: mpsc::Receiver<Command>, sinks: Vec<Arc<dyn EventSink>>) {
    let mut pending = Vec::new();
    let mut deadline = None;
    let mut failure: Option<CricketError> = None;
    loop {
        let command = if let Some(time) = deadline {
            tokio::select! {
                biased;
                _=sleep_until(time)=>{
                    if let Err(e)=flush_pending(&mut pending,&sinks) {failure=Some(e);}
                    deadline=None;
                    continue;
                },
                command=rx.recv()=>command,
            }
        } else {
            rx.recv().await
        };
        let Some(command) = command else {
            let _ = flush_pending(&mut pending, &sinks);
            break;
        };
        match command {
            Command::Emit(event, reply) => {
                let result = if let Some(e) = &failure {
                    Err(e.clone())
                } else if matches!(
                    event,
                    CoreEvent::TextDelta { .. } | CoreEvent::ReasoningDelta { .. }
                ) {
                    append(&mut pending, event);
                    if deadline.is_none() {
                        deadline = Some(Instant::now() + FRAME_WINDOW);
                    }
                    Ok(())
                } else {
                    deadline = None;
                    flush_pending(&mut pending, &sinks).and_then(|()| fanout(event, &sinks))
                };
                if let Err(e) = &result {
                    failure = Some(e.clone());
                }
                let _ = reply.send(result);
            }
            Command::Flush(reply) => {
                deadline = None;
                let result = failure
                    .clone()
                    .map_or_else(|| flush_pending(&mut pending, &sinks), Err);
                if let Err(e) = &result {
                    failure = Some(e.clone());
                }
                let _ = reply.send(result);
            }
            Command::Close(reply) => {
                let result = failure.map_or_else(|| flush_pending(&mut pending, &sinks), Err);
                let _ = reply.send(result);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cricket_protocol::{Role, Stage};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Capture(Mutex<Vec<CoreEvent>>);
    impl EventSink for Capture {
        fn emit(&self, event: CoreEvent) -> Result<(), CricketError> {
            self.0.lock().map_err(|_| closed())?.push(event);
            Ok(())
        }
    }
    fn text(s: &str) -> CoreEvent {
        CoreEvent::TextDelta {
            message_id: "m".into(),
            text: s.into(),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn sixty_deltas_coalesce_into_at_most_five_frames() {
        let capture = Arc::new(Capture::default());
        let emitter = Emitter::new(vec![capture.clone()]).unwrap();
        for _ in 0..60 {
            emitter.emit(text("x")).await.unwrap();
            tokio::time::advance(Duration::from_millis(1)).await;
            tokio::task::yield_now().await;
        }
        emitter.close().await.unwrap();
        let output = capture.0.lock().unwrap();
        assert!(output.len() <= 5, "{} frames", output.len());
        assert!(
            output.len() >= 3,
            "timer must actually flush during the stream"
        );
        let joined = output
            .iter()
            .map(|e| match e {
                CoreEvent::TextDelta { text, .. } => text.as_str(),
                _ => panic!("wrong event"),
            })
            .collect::<String>();
        assert_eq!(joined, "x".repeat(60));
    }

    #[tokio::test(start_paused = true)]
    async fn sparse_delta_flushes_without_another_event() {
        let capture = Arc::new(Capture::default());
        let emitter = Emitter::new(vec![capture.clone()]).unwrap();
        emitter.emit(text("one")).await.unwrap();
        tokio::time::advance(FRAME_WINDOW).await;
        tokio::task::yield_now().await;
        assert_eq!(*capture.0.lock().unwrap(), vec![text("one")]);
        emitter.close().await.unwrap();
    }

    #[tokio::test]
    async fn controls_preserve_order_and_reasoning_is_separate() {
        let a = Arc::new(Capture::default());
        let b = Arc::new(Capture::default());
        let emitter = Emitter::new(vec![a.clone(), b.clone()]).unwrap();
        let start = CoreEvent::MessageStart {
            message_id: "m".into(),
            role: Role::Assistant,
            parent_event_id: None,
        };
        let thought = CoreEvent::ReasoningDelta {
            message_id: "m".into(),
            text: "why".into(),
        };
        let stage = CoreEvent::StageChanged { stage: Stage::Done };
        for e in [
            start.clone(),
            text("a"),
            text("b"),
            thought.clone(),
            stage.clone(),
            CoreEvent::StreamClosed,
        ] {
            emitter.emit(e).await.unwrap();
        }
        emitter.close().await.unwrap();
        let expected = vec![start, text("ab"), thought, stage, CoreEvent::StreamClosed];
        assert_eq!(*a.0.lock().unwrap(), expected);
        assert_eq!(*b.0.lock().unwrap(), expected);
    }

    struct Panicking;
    impl EventSink for Panicking {
        fn emit(&self, _: CoreEvent) -> Result<(), CricketError> {
            panic!("sink failure")
        }
    }
    #[tokio::test]
    async fn sink_panic_becomes_error_and_other_sinks_receive_event() {
        let capture = Arc::new(Capture::default());
        let emitter = Emitter::new(vec![Arc::new(Panicking), capture.clone()]).unwrap();
        assert!(matches!(
            emitter.emit(CoreEvent::StreamClosed).await,
            Err(CricketError::Internal(_))
        ));
        assert_eq!(*capture.0.lock().unwrap(), vec![CoreEvent::StreamClosed]);
        assert!(emitter.close().await.is_err());
    }
}
