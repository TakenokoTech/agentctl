use a2a_rust::{
    A2AClient, A2AClientConfig,
    types::{
        CancelTaskRequest, GetTaskRequest, Message, Part, Role, SendMessageConfiguration,
        SendMessageRequest, SendMessageResponse, StreamResponse, SubscribeToTaskRequest, Task,
        TaskState as WireState,
    },
};
use anyhow::Result;
use futures_util::StreamExt;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use crate::domain::a2a::{
    A2aConnector, A2aSession, AgentCard, ArtifactInfo, EventStream, MessageInfo, Reply,
    SendRequest, SkillInfo, TaskEvent, TaskInfo, TaskState,
};
use crate::domain::config::A2aAgent;
use crate::infrastructure::config::read_token;

pub struct A2aRustConnector;

pub struct A2aRustSession(A2AClient);

impl A2aConnector for A2aRustConnector {
    type Session = A2aRustSession;

    fn connect(&self, agent: &A2aAgent) -> Result<A2aRustSession> {
        let http = reqwest::Client::builder()
            .default_headers(auth_headers(read_token(&agent.token_env)?)?)
            .build()?;
        Ok(A2aRustSession(A2AClient::with_http_client(
            &agent.url,
            http,
            A2AClientConfig::default(),
        )?))
    }
}

impl A2aSession for A2aRustSession {
    async fn card(&self) -> Result<AgentCard> {
        let card = self.0.discover_agent_card().await?;
        Ok(AgentCard {
            name: card.name.clone(),
            description: card.description.clone(),
            streaming: card.capabilities.streaming.unwrap_or(false),
            skills: card
                .skills
                .iter()
                .map(|s| SkillInfo {
                    id: s.id.clone(),
                    description: s.description.clone(),
                    examples: s.examples.clone(),
                })
                .collect(),
            raw: serde_json::to_value(&card)?,
        })
    }

    async fn send(&self, req: SendRequest) -> Result<Reply> {
        let req = SendMessageRequest {
            message: user_message(req.text, req.context_id, req.task_id),
            configuration: Some(SendMessageConfiguration {
                return_immediately: req.no_wait,
                ..Default::default()
            }),
            metadata: None,
            tenant: None,
        };
        Ok(match self.0.send_message(req).await? {
            SendMessageResponse::Message(m) => Reply::Message(message_info(&m)?),
            SendMessageResponse::Task(t) => Reply::Task(task_info(&t)?),
        })
    }

    async fn get_task(&self, id: &str) -> Result<TaskInfo> {
        let req = GetTaskRequest {
            id: id.to_owned(),
            history_length: None,
            tenant: None,
        };
        task_info(&self.0.get_task(req).await?)
    }

    async fn cancel_task(&self, id: &str) -> Result<TaskInfo> {
        let req = CancelTaskRequest {
            id: id.to_owned(),
            tenant: None,
            metadata: None,
        };
        task_info(&self.0.cancel_task(req).await?)
    }

    async fn watch(&self, id: &str) -> Result<EventStream> {
        let stream = self
            .0
            .subscribe_to_task(SubscribeToTaskRequest {
                id: id.to_owned(),
                tenant: None,
            })
            .await?;
        Ok(Box::pin(stream.map(|ev| event_info(&ev?))))
    }
}

fn auth_headers(token: Option<String>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    if let Some(token) = token {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))?,
        );
    }
    Ok(headers)
}

fn user_message(text: String, context_id: Option<String>, task_id: Option<String>) -> Message {
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
fn state_info(s: &WireState) -> TaskState {
    let name = serde_json::to_value(s)
        .ok()
        .and_then(|v| {
            v.as_str()
                .map(|s| s.trim_start_matches("TASK_STATE_").to_ascii_lowercase())
        })
        .unwrap_or_default();
    TaskState::new(name)
}

fn text_of(parts: &[Part]) -> String {
    parts
        .iter()
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
}

fn message_info(m: &Message) -> Result<MessageInfo> {
    Ok(MessageInfo {
        context_id: m.context_id.clone(),
        text: text_of(&m.parts),
        raw: serde_json::to_value(m)?,
    })
}

fn task_info(t: &Task) -> Result<TaskInfo> {
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

fn event_info(ev: &StreamResponse) -> Result<TaskEvent> {
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
