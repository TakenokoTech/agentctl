use super::*;
use crate::domain::a2a::{ArtifactInfo, SkillInfo};

fn task(state: &str) -> TaskInfo {
    TaskInfo {
        id: "t1".into(),
        context_id: Some("c1".into()),
        state: TaskState::new(state),
        message: None,
        artifacts: vec![ArtifactInfo {
            name: Some("result".into()),
            text: "done".into(),
        }],
        raw: json!({"raw": true}),
    }
}

fn card() -> AgentCard {
    AgentCard {
        name: "Test Agent".into(),
        description: "Echoes".into(),
        streaming: true,
        skills: vec![SkillInfo {
            id: "echo".into(),
            description: "Returns the input".into(),
            examples: vec!["hello".into()],
        }],
        raw: json!({"raw": "card"}),
    }
}

#[test]
fn card_summary_lists_skills() {
    assert_eq!(
        card_json(&card(), false),
        json!({
            "name": "Test Agent", "description": "Echoes", "streaming": true,
            "skills": [{"id": "echo", "description": "Returns the input", "examples": ["hello"]}],
        })
    );
}

#[test]
fn card_raw_returns_the_protocol_value() {
    assert_eq!(card_json(&card(), true), json!({"raw": "card"}));
}

#[test]
fn message_summary_and_raw() {
    let m = MessageInfo {
        context_id: Some("c".into()),
        text: "echo: hi".into(),
        raw: json!({"raw": "message"}),
    };
    assert_eq!(
        message_json(&m, false),
        json!({"type": "message", "contextId": "c", "text": "echo: hi"})
    );
    assert_eq!(message_json(&m, true), json!({"raw": "message"}));
}

#[test]
fn task_events_are_summarized_like_tasks_and_messages() {
    assert_eq!(
        event_json(&TaskEvent::Task(task("working")), false),
        task_json(&task("working"), false)
    );
    let m = MessageInfo {
        context_id: None,
        text: "t".into(),
        raw: Value::Null,
    };
    assert_eq!(
        event_json(&TaskEvent::Message(m.clone()), false),
        message_json(&m, false)
    );
}

#[test]
fn artifact_event_is_summarized_with_chunk_flags() {
    let ev = TaskEvent::Artifact {
        task_id: "t1".into(),
        name: Some("result".into()),
        text: "chunk".into(),
        append: true,
        last_chunk: false,
        raw: Value::Null,
    };
    assert_eq!(
        event_json(&ev, false),
        json!({"type": "artifact", "taskId": "t1", "name": "result",
               "text": "chunk", "append": true, "lastChunk": false})
    );
}

#[test]
fn raw_events_pass_the_protocol_value_through() {
    let ev = TaskEvent::Status {
        task_id: "t1".into(),
        state: TaskState::new("working"),
        message: None,
        raw: json!({"raw": "event"}),
    };
    assert_eq!(event_json(&ev, true), json!({"raw": "event"}));
}

#[test]
fn task_summary_is_compact() {
    assert_eq!(
        task_json(&task("working"), false),
        json!({
            "type": "task", "id": "t1", "contextId": "c1", "state": "working",
            "message": null, "artifacts": [{"name": "result", "text": "done"}],
        })
    );
}

#[test]
fn raw_returns_the_protocol_value() {
    assert_eq!(task_json(&task("working"), true), json!({"raw": true}));
}

#[test]
fn only_failed_and_rejected_map_to_exit_code_2() {
    assert_eq!(
        exit_code(&TaskState::new("failed")),
        ExitCode::from(EXIT_TOOL_FAILED)
    );
    assert_eq!(
        exit_code(&TaskState::new("rejected")),
        ExitCode::from(EXIT_TOOL_FAILED)
    );
    assert_eq!(exit_code(&TaskState::new("canceled")), ExitCode::SUCCESS);
    assert_eq!(exit_code(&TaskState::new("completed")), ExitCode::SUCCESS);
}

#[test]
fn status_event_is_summarized() {
    let ev = TaskEvent::Status {
        task_id: "t1".into(),
        state: TaskState::new("input_required"),
        message: Some("need more".into()),
        raw: Value::Null,
    };
    assert_eq!(
        event_json(&ev, false),
        json!({"type": "status", "taskId": "t1", "state": "input_required", "message": "need more"})
    );
}

#[test]
fn watch_ending_in_a_settled_state_exits_by_that_state() {
    let code = |s: &str| watch_exit(Some(&TaskState::new(s))).unwrap();
    assert_eq!(code("completed"), ExitCode::SUCCESS);
    assert_eq!(code("input_required"), ExitCode::SUCCESS);
    assert_eq!(code("failed"), ExitCode::from(EXIT_TOOL_FAILED));
}

#[test]
fn watch_ending_before_a_settled_state_is_an_error() {
    let err = watch_exit(None).unwrap_err().to_string();
    assert!(err.contains("before the task finished"), "{err}");
    let err = watch_exit(Some(&TaskState::new("working")))
        .unwrap_err()
        .to_string();
    assert!(err.contains("working"), "{err}");
}
