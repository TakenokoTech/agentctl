use serde_json::json;

use super::*;

#[test]
fn only_failed_and_rejected_are_failures() {
    for name in ["failed", "rejected"] {
        assert!(TaskState::new(name).is_failure(), "{name}");
    }
    for name in [
        "submitted",
        "working",
        "completed",
        "canceled",
        "input_required",
        "",
    ] {
        assert!(!TaskState::new(name).is_failure(), "{name}");
    }
}

#[test]
fn watching_may_stop_once_the_task_is_final_or_waiting_for_the_user() {
    for name in [
        "completed",
        "failed",
        "rejected",
        "canceled",
        "input_required",
        "auth_required",
    ] {
        assert!(TaskState::new(name).is_settled(), "{name}");
    }
    for name in ["submitted", "working", ""] {
        assert!(!TaskState::new(name).is_settled(), "{name}");
    }
}

#[test]
fn events_expose_state_only_when_they_carry_one() {
    let task = TaskInfo {
        id: "t".into(),
        context_id: None,
        state: TaskState::new("working"),
        message: None,
        artifacts: vec![],
        raw: json!("task"),
    };
    let message = MessageInfo {
        context_id: None,
        text: "m".into(),
        raw: json!("message"),
    };
    let status = TaskEvent::Status {
        task_id: "t".into(),
        state: TaskState::new("completed"),
        message: None,
        raw: json!("status"),
    };
    let artifact = TaskEvent::Artifact {
        task_id: "t".into(),
        name: None,
        text: String::new(),
        append: false,
        last_chunk: false,
        raw: json!("artifact"),
    };

    assert_eq!(
        TaskEvent::Task(task.clone()).state(),
        Some(&TaskState::new("working"))
    );
    assert_eq!(status.state(), Some(&TaskState::new("completed")));
    assert_eq!(TaskEvent::Message(message.clone()).state(), None);
    assert_eq!(artifact.state(), None);

    assert_eq!(TaskEvent::Task(task).raw(), &json!("task"));
    assert_eq!(TaskEvent::Message(message).raw(), &json!("message"));
    assert_eq!(status.raw(), &json!("status"));
    assert_eq!(artifact.raw(), &json!("artifact"));
}

#[test]
fn only_the_four_final_states_are_terminal() {
    for name in ["completed", "failed", "canceled", "rejected"] {
        assert!(TaskState::new(name).is_terminal(), "{name}");
    }
    // input_required / auth_required は、ユーザーの入力を待つ途中の状態
    for name in [
        "submitted",
        "working",
        "input_required",
        "auth_required",
        "unspecified",
        "",
    ] {
        assert!(!TaskState::new(name).is_terminal(), "{name}");
    }
}
