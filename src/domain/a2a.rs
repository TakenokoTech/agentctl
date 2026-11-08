use std::pin::Pin;

use anyhow::Result;
use futures_util::Stream;
use serde_json::Value;

use crate::domain::config::A2aAgent;

/// 小文字のスネークケースに正規化した task 状態(例: `input_required`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskState(String);

impl TaskState {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    /// これ以上変わらない最終の状態か。`input_required` などは、入力を待つ途中の状態で含まない。
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.0.as_str(),
            "completed" | "failed" | "canceled" | "rejected"
        )
    }

    /// 処理が失敗した状態か。
    pub fn is_failure(&self) -> bool {
        matches!(self.0.as_str(), "failed" | "rejected")
    }
}

#[derive(Debug, Clone)]
pub struct SendRequest {
    pub text: String,
    pub context_id: Option<String>,
    pub task_id: Option<String>,
    /// 受付だけで返す。
    pub no_wait: bool,
}

#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub id: String,
    pub description: String,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub streaming: bool,
    pub skills: Vec<SkillInfo>,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub context_id: Option<String>,
    pub text: String,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct ArtifactInfo {
    pub name: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: String,
    pub context_id: Option<String>,
    pub state: TaskState,
    pub message: Option<String>,
    pub artifacts: Vec<ArtifactInfo>,
    pub raw: Value,
}

/// `send` の返事。message か task のどちらか。
#[derive(Debug, Clone)]
pub enum Reply {
    Message(MessageInfo),
    Task(TaskInfo),
}

/// `task watch` で流れてくるイベント。
#[derive(Debug, Clone)]
pub enum TaskEvent {
    Task(TaskInfo),
    Message(MessageInfo),
    Status {
        task_id: String,
        state: TaskState,
        message: Option<String>,
        raw: Value,
    },
    Artifact {
        task_id: String,
        name: Option<String>,
        text: String,
        append: bool,
        last_chunk: bool,
        raw: Value,
    },
}

impl TaskEvent {
    pub fn state(&self) -> Option<&TaskState> {
        match self {
            TaskEvent::Task(t) => Some(&t.state),
            TaskEvent::Status { state, .. } => Some(state),
            TaskEvent::Message(_) | TaskEvent::Artifact { .. } => None,
        }
    }

    pub fn raw(&self) -> &Value {
        match self {
            TaskEvent::Task(t) => &t.raw,
            TaskEvent::Message(m) => &m.raw,
            TaskEvent::Status { raw, .. } | TaskEvent::Artifact { raw, .. } => raw,
        }
    }
}

/// `task watch` の終わり方。
#[derive(Debug, PartialEq)]
pub enum WatchEnd {
    /// 終端状態に達した。
    Finished(TaskState),
    /// 終端状態に達する前に、ストリームが閉じた。最後に観測した状態を持つ。
    Interrupted(Option<TaskState>),
}

pub type EventStream = Pin<Box<dyn Stream<Item = Result<TaskEvent>>>>;

#[allow(async_fn_in_trait)]
pub trait A2aSession {
    async fn card(&self) -> Result<AgentCard>;
    async fn send(&self, req: SendRequest) -> Result<Reply>;
    async fn get_task(&self, id: &str) -> Result<TaskInfo>;
    async fn cancel_task(&self, id: &str) -> Result<TaskInfo>;
    async fn watch(&self, id: &str) -> Result<EventStream>;
}

pub trait A2aConnector {
    type Session: A2aSession;

    fn connect(&self, agent: &A2aAgent) -> Result<Self::Session>;
}

#[cfg(test)]
mod tests;
