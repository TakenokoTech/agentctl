//! `a2a-rust` の型と、ドメインの型の間の変換。
use a2a_rust::types::{Message, Part, Role, StreamResponse, Task, TaskState as WireState};
use anyhow::Result;

use crate::domain::a2a::{ArtifactInfo, MessageInfo, TaskEvent, TaskInfo, TaskState};

pub(super) fn user_message(
    text: String,
    context_id: Option<String>,
    task_id: Option<String>,
) -> Message {
    Message {
        message_id: uuid::Uuid::new_v4().to_string(),
        context_id,
        task_id,
        role: Role::User,
        parts: vec![Part {
            text: Some(text),
            raw: None,
            url: None,
            data: None,
            metadata: None,
            filename: None,
            media_type: None,
        }],
        metadata: None,
        extensions: Vec::new(),
        reference_task_ids: Vec::new(),
    }
}

/// "TASK_STATE_INPUT_REQUIRED" -> "input_required"
pub(super) fn state_info(s: &WireState) -> TaskState {
    let name = serde_json::to_value(s)
        .ok()
        .and_then(|v| {
            v.as_str()
                .map(|s| s.trim_start_matches("TASK_STATE_").to_ascii_lowercase())
        })
        .unwrap_or_else(|| "unknown".to_owned());
    TaskState::new(name)
}

pub(super) fn text_of(parts: &[Part]) -> String {
    parts
        .iter()
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn message_info(m: &Message) -> Result<MessageInfo> {
    Ok(MessageInfo {
        context_id: m.context_id.clone(),
        text: text_of(&m.parts),
        raw: serde_json::to_value(m)?,
    })
}

pub(super) fn task_info(t: &Task) -> Result<TaskInfo> {
    Ok(TaskInfo {
        id: t.id.clone(),
        context_id: t.context_id.clone(),
        state: state_info(&t.status.state),
        message: t.status.message.as_ref().map(|m| text_of(&m.parts)),
        artifacts: t
            .artifacts
            .iter()
            .map(|a| ArtifactInfo {
                name: a.name.clone(),
                text: text_of(&a.parts),
            })
            .collect(),
        raw: serde_json::to_value(t)?,
    })
}

pub(super) fn event_info(ev: &StreamResponse) -> Result<TaskEvent> {
    Ok(match ev {
        StreamResponse::Task(t) => TaskEvent::Task(task_info(t)?),
        StreamResponse::Message(m) => TaskEvent::Message(message_info(m)?),
        StreamResponse::StatusUpdate(e) => TaskEvent::Status {
            task_id: e.task_id.clone(),
            state: state_info(&e.status.state),
            message: e.status.message.as_ref().map(|m| text_of(&m.parts)),
            raw: serde_json::to_value(ev)?,
        },
        StreamResponse::ArtifactUpdate(e) => TaskEvent::Artifact {
            task_id: e.task_id.clone(),
            name: e.artifact.name.clone(),
            text: text_of(&e.artifact.parts),
            append: e.append,
            last_chunk: e.last_chunk,
            raw: serde_json::to_value(ev)?,
        },
    })
}

#[cfg(test)]
mod tests;
