use a2a_rust::types::{Artifact, TaskArtifactUpdateEvent, TaskStatus, TaskStatusUpdateEvent};

use super::*;

fn part(text: &str) -> Part {
    Part {
        text: Some(text.into()),
        raw: None,
        url: None,
        data: None,
        metadata: None,
        filename: None,
        media_type: None,
    }
}

fn agent_message(parts: Vec<Part>) -> Message {
    Message {
        role: Role::Agent,
        parts,
        ..user_message(String::new(), Some("ctx".into()), None)
    }
}

fn status(state: WireState, message: Option<Message>) -> TaskStatus {
    TaskStatus {
        state,
        message,
        timestamp: None,
    }
}

fn artifact(name: Option<&str>, parts: Vec<Part>) -> Artifact {
    Artifact {
        artifact_id: "a".into(),
        name: name.map(str::to_owned),
        description: None,
        parts,
        metadata: None,
        extensions: Vec::new(),
    }
}

fn task(state: WireState, message: Option<Message>, artifacts: Vec<Artifact>) -> Task {
    Task {
        id: "t1".into(),
        context_id: Some("c1".into()),
        status: status(state, message),
        artifacts,
        history: Vec::new(),
        metadata: None,
    }
}

#[test]
fn state_names_are_short_snake_case() {
    assert_eq!(
        state_info(&WireState::Completed),
        TaskState::new("completed")
    );
    assert_eq!(
        state_info(&WireState::InputRequired),
        TaskState::new("input_required")
    );
    assert!(state_info(&WireState::Failed).is_failure());
    assert!(state_info(&WireState::Rejected).is_failure());
    assert!(!state_info(&WireState::Working).is_failure());
}

#[test]
fn text_of_joins_text_parts_and_skips_the_others() {
    let mut data_only = part("ignored");
    data_only.text = None;
    let parts = [part("a"), data_only, part("b")];
    assert_eq!(text_of(&parts), "a\nb");
    assert_eq!(text_of(&[]), "");
}

#[test]
fn user_message_is_a_user_text_message_with_a_fresh_id() {
    let a = user_message("hi".into(), Some("c".into()), Some("t".into()));
    let b = user_message("hi".into(), None, None);
    assert!(matches!(a.role, Role::User));
    assert_eq!(a.context_id.as_deref(), Some("c"));
    assert_eq!(a.task_id.as_deref(), Some("t"));
    assert_eq!(text_of(&a.parts), "hi");
    assert_ne!(a.message_id, b.message_id);
}

#[test]
fn message_info_keeps_context_text_and_raw() {
    let m = agent_message(vec![part("echo: hi")]);
    let info = message_info(&m).unwrap();
    assert_eq!(info.context_id.as_deref(), Some("ctx"));
    assert_eq!(info.text, "echo: hi");
    assert_eq!(info.raw["parts"][0]["text"], "echo: hi");
}

#[test]
fn task_info_maps_state_message_and_artifacts() {
    let t = task(
        WireState::InputRequired,
        Some(agent_message(vec![part("need more")])),
        vec![
            artifact(Some("result"), vec![part("done")]),
            artifact(None, vec![]),
        ],
    );
    let info = task_info(&t).unwrap();
    assert_eq!(info.id, "t1");
    assert_eq!(info.context_id.as_deref(), Some("c1"));
    assert_eq!(info.state, TaskState::new("input_required"));
    assert_eq!(info.message.as_deref(), Some("need more"));
    assert_eq!(info.artifacts.len(), 2);
    assert_eq!(info.artifacts[0].name.as_deref(), Some("result"));
    assert_eq!(info.artifacts[0].text, "done");
    assert_eq!(info.artifacts[1].name, None);
    assert_eq!(info.raw["id"], "t1");
}

#[test]
fn task_without_status_message_has_none() {
    let info = task_info(&task(WireState::Working, None, vec![])).unwrap();
    assert_eq!(info.message, None);
    assert!(info.artifacts.is_empty());
}

#[test]
fn event_info_maps_every_stream_response() {
    let ev = event_info(&StreamResponse::Task(task(
        WireState::Working,
        None,
        vec![],
    )))
    .unwrap();
    assert!(matches!(ev, TaskEvent::Task(t) if t.state == TaskState::new("working")));

    let ev = event_info(&StreamResponse::Message(agent_message(vec![part("m")]))).unwrap();
    assert!(matches!(ev, TaskEvent::Message(m) if m.text == "m"));

    let ev = event_info(&StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "t1".into(),
        context_id: "c1".into(),
        status: status(WireState::Completed, Some(agent_message(vec![part("ok")]))),
        metadata: None,
    }))
    .unwrap();
    match ev {
        TaskEvent::Status {
            task_id,
            state,
            message,
            ..
        } => {
            assert_eq!(task_id, "t1");
            assert_eq!(state, TaskState::new("completed"));
            assert_eq!(message.as_deref(), Some("ok"));
        }
        other => panic!("unexpected: {other:?}"),
    }

    let ev = event_info(&StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
        task_id: "t1".into(),
        context_id: "c1".into(),
        artifact: artifact(Some("r"), vec![part("chunk")]),
        append: true,
        last_chunk: true,
        metadata: None,
    }))
    .unwrap();
    match ev {
        TaskEvent::Artifact {
            name,
            text,
            append,
            last_chunk,
            ..
        } => {
            assert_eq!(name.as_deref(), Some("r"));
            assert_eq!(text, "chunk");
            assert!(append && last_chunk);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

/// 期待する短い名前と、失敗かどうか。網羅的な `match` なので、ライブラリに状態が増えるとコンパイルが通らなくなる
fn expected(wire: &WireState) -> (&'static str, bool) {
    match wire {
        WireState::Unspecified => ("unspecified", false),
        WireState::Submitted => ("submitted", false),
        WireState::Working => ("working", false),
        WireState::Completed => ("completed", false),
        WireState::Failed => ("failed", true),
        WireState::Canceled => ("canceled", false),
        WireState::InputRequired => ("input_required", false),
        WireState::Rejected => ("rejected", true),
        WireState::AuthRequired => ("auth_required", false),
    }
}

#[test]
fn every_wire_state_maps_to_its_short_name_and_only_failures_fail() {
    let all = [
        WireState::Unspecified,
        WireState::Submitted,
        WireState::Working,
        WireState::Completed,
        WireState::Failed,
        WireState::Canceled,
        WireState::InputRequired,
        WireState::Rejected,
        WireState::AuthRequired,
    ];
    for wire in all {
        let (name, failure) = expected(&wire);
        let state = state_info(&wire);
        assert_eq!(state.name(), name, "{wire:?}");
        assert_eq!(state.is_failure(), failure, "{wire:?}");
    }
}
