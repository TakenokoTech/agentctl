use anyhow::anyhow;
use futures_util::stream;
use serde_json::Value;

use super::*;
use crate::domain::a2a::{EventStream, MessageInfo};

struct FakeConnector;
struct FakeSession;

fn status(state: &str) -> TaskEvent {
    TaskEvent::Status {
        task_id: "t".into(),
        state: TaskState::new(state),
        message: None,
        raw: Value::Null,
    }
}

fn task(id: &str, state: &str) -> TaskInfo {
    TaskInfo {
        id: id.into(),
        context_id: None,
        state: TaskState::new(state),
        message: None,
        artifacts: vec![],
        raw: Value::Null,
    }
}

impl A2aConnector for FakeConnector {
    type Session = FakeSession;

    fn connect(&self, agent: &A2aAgent) -> Result<FakeSession> {
        if agent.url == "unreachable" {
            return Err(anyhow!("cannot connect"));
        }
        Ok(FakeSession)
    }
}

impl A2aSession for FakeSession {
    async fn card(&self) -> Result<AgentCard> {
        Ok(AgentCard {
            name: "Fake".into(),
            description: String::new(),
            streaming: true,
            skills: vec![],
            raw: Value::Null,
        })
    }

    async fn send(&self, req: SendRequest) -> Result<Reply> {
        Ok(if req.no_wait {
            Reply::Task(task("new", "working"))
        } else {
            Reply::Message(MessageInfo {
                context_id: req.context_id,
                text: req.text,
                raw: Value::Null,
            })
        })
    }

    async fn get_task(&self, id: &str) -> Result<TaskInfo> {
        Ok(task(id, "working"))
    }

    async fn cancel_task(&self, id: &str) -> Result<TaskInfo> {
        Ok(task(id, "canceled"))
    }

    async fn watch(&self, id: &str) -> Result<EventStream> {
        let events = match id {
            "broken" => vec![Ok(status("working")), Err(anyhow!("stream broke"))],
            "silent" => vec![],
            _ => vec![
                Ok(status("working")),
                Ok(TaskEvent::Artifact {
                    task_id: "t".into(),
                    name: None,
                    text: "x".into(),
                    append: false,
                    last_chunk: true,
                    raw: Value::Null,
                }),
                Ok(status("completed")),
            ],
        };
        Ok(Box::pin(stream::iter(events)))
    }
}

fn agent(url: &str) -> A2aAgent {
    A2aAgent {
        url: url.into(),
        description: None,
        token_env: None,
    }
}

fn request(text: &str, no_wait: bool) -> SendRequest {
    SendRequest {
        text: text.into(),
        context_id: Some("ctx".into()),
        task_id: None,
        no_wait,
    }
}

#[tokio::test]
async fn card_is_fetched_through_the_session() {
    let card = card(&FakeConnector, &agent("ok")).await.unwrap();
    assert_eq!(card.name, "Fake");
}

#[tokio::test]
async fn send_returns_a_message_or_a_task_as_the_agent_replies() {
    let reply = send(&FakeConnector, &agent("ok"), request("hi", false))
        .await
        .unwrap();
    assert!(
        matches!(reply, Reply::Message(m) if m.text == "hi" && m.context_id.as_deref() == Some("ctx"))
    );

    let reply = send(&FakeConnector, &agent("ok"), request("hi", true))
        .await
        .unwrap();
    assert!(matches!(reply, Reply::Task(t) if t.id == "new"));
}

#[tokio::test]
async fn get_and_cancel_target_the_given_task() {
    let t = get_task(&FakeConnector, &agent("ok"), "abc").await.unwrap();
    assert_eq!((t.id.as_str(), t.state.name()), ("abc", "working"));
    let t = cancel_task(&FakeConnector, &agent("ok"), "abc")
        .await
        .unwrap();
    assert_eq!((t.id.as_str(), t.state.name()), ("abc", "canceled"));
}

#[tokio::test]
async fn connect_failure_is_propagated() {
    let err = card(&FakeConnector, &agent("unreachable"))
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "cannot connect");
    let err = watch(&FakeConnector, &agent("unreachable"), "t", |_| Ok(()))
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "cannot connect");
}

#[tokio::test]
async fn watch_reports_every_event_and_the_last_state() {
    let mut seen = 0;
    let last = watch(&FakeConnector, &agent("ok"), "t", |_| {
        seen += 1;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(seen, 3);
    assert_eq!(last, Some(TaskState::new("completed")));
}

#[tokio::test]
async fn watch_without_events_has_no_state() {
    let last = watch(&FakeConnector, &agent("ok"), "silent", |_| Ok(()))
        .await
        .unwrap();
    assert_eq!(last, None);
}

#[tokio::test]
async fn watch_stops_at_a_stream_error_after_delivering_earlier_events() {
    let mut seen = Vec::new();
    let err = watch(&FakeConnector, &agent("ok"), "broken", |ev| {
        seen.push(ev.state().map(|s| s.name().to_owned()));
        Ok(())
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "stream broke");
    assert_eq!(seen, vec![Some("working".to_owned())]);
}

#[tokio::test]
async fn watch_stops_when_the_callback_fails() {
    let mut calls = 0;
    let err = watch(&FakeConnector, &agent("ok"), "t", |_| {
        calls += 1;
        Err(anyhow!("stdout closed"))
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "stdout closed");
    assert_eq!(calls, 1);
}
