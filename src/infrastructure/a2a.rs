use a2a_rust::{
    A2AClient, A2AClientConfig,
    types::{
        AgentInterface, CancelTaskRequest, GetTaskRequest, Message, Part, Role,
        SendMessageConfiguration, SendMessageRequest, SendMessageResponse, StreamResponse,
        SubscribeToTaskRequest, Task, TaskState as WireState,
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
use crate::infrastructure::config::{ensure_secure_transport, read_token};

pub struct A2aRustConnector;

pub struct A2aRustSession {
    client: A2AClient,
    /// トークンを持つときだけ、送信先を縛る基準になる設定済みの URL。
    token_origin: Option<String>,
}

impl A2aConnector for A2aRustConnector {
    type Session = A2aRustSession;

    fn connect(&self, agent: &A2aAgent) -> Result<A2aRustSession> {
        let token = read_token(&agent.token_env)?;
        let has_token = token.is_some();
        ensure_secure_transport(&agent.url, has_token, agent.allow_insecure)?;
        let mut builder = reqwest::Client::builder().default_headers(auth_headers(token)?);
        if has_token {
            builder = builder.redirect(same_origin_redirects(&agent.url)?);
        }
        let http = builder.build()?;
        Ok(A2aRustSession {
            client: A2AClient::with_http_client(&agent.url, http, A2AClientConfig::default())?,
            token_origin: has_token.then(|| agent.url.clone()),
        })
    }
}

impl A2aRustSession {
    /// トークンを持つとき、カードが広告するエンドポイントが設定した URL と同じオリジンか確かめる。
    /// カードはクライアント内にキャッシュされ、続く送信先の選択も同じカードを使う。
    async fn guard_token_origin(&self) -> Result<()> {
        if let Some(base) = &self.token_origin {
            let card = self.client.discover_agent_card().await?;
            ensure_same_origin(base, &card.supported_interfaces)?;
        }
        Ok(())
    }
}

/// リダイレクト先が `base` と同じオリジン(スキーム、ホスト、ポート)か。
/// reqwest はスキームだけが変わるリダイレクトでは Authorization を外さないため、自前で判断する。
fn redirect_allowed(next: &reqwest::Url, base: &reqwest::Url) -> bool {
    next.origin() == base.origin()
}

/// トークンを持つときのリダイレクト方針。設定した URL と同じオリジンへのものだけ辿る。
fn same_origin_redirects(base: &str) -> Result<reqwest::redirect::Policy> {
    const MAX_REDIRECTS: usize = 10;
    let base = reqwest::Url::parse(base)?;
    Ok(reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            attempt.error("too many redirects")
        } else if redirect_allowed(attempt.url(), &base) {
            attempt.follow()
        } else {
            let msg = format!(
                "refusing to follow a redirect to a different origin ({}) with a token",
                attempt.url()
            );
            attempt.error(msg)
        }
    }))
}

/// クライアントが使うバインディングのエンドポイントが、すべて `base` と同じオリジンにあるか。
fn ensure_same_origin(base: &str, interfaces: &[AgentInterface]) -> Result<()> {
    let base = reqwest::Url::parse(base)?;
    for i in interfaces {
        let binding = i.protocol_binding.as_str();
        if !(binding.eq_ignore_ascii_case("JSONRPC") || binding.eq_ignore_ascii_case("HTTP+JSON")) {
            continue;
        }
        let url = reqwest::Url::parse(&i.url).or_else(|_| base.join(&i.url))?;
        if url.origin() != base.origin() {
            anyhow::bail!(
                "the agent card advertises an endpoint on a different origin ({url}); \
                 refusing to send the token there"
            );
        }
    }
    Ok(())
}

impl A2aSession for A2aRustSession {
    async fn card(&self) -> Result<AgentCard> {
        let card = self.client.discover_agent_card().await?;
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
        self.guard_token_origin().await?;
        let req = SendMessageRequest {
            message: user_message(req.text, req.context_id, req.task_id),
            configuration: Some(SendMessageConfiguration {
                return_immediately: req.no_wait,
                ..Default::default()
            }),
            metadata: None,
            tenant: None,
        };
        Ok(match self.client.send_message(req).await? {
            SendMessageResponse::Message(m) => Reply::Message(message_info(&m)?),
            SendMessageResponse::Task(t) => Reply::Task(task_info(&t)?),
        })
    }

    async fn get_task(&self, id: &str) -> Result<TaskInfo> {
        self.guard_token_origin().await?;
        let req = GetTaskRequest {
            id: id.to_owned(),
            history_length: None,
            tenant: None,
        };
        task_info(&self.client.get_task(req).await?)
    }

    async fn cancel_task(&self, id: &str) -> Result<TaskInfo> {
        self.guard_token_origin().await?;
        let req = CancelTaskRequest {
            id: id.to_owned(),
            tenant: None,
            metadata: None,
        };
        task_info(&self.client.cancel_task(req).await?)
    }

    async fn watch(&self, id: &str) -> Result<EventStream> {
        self.guard_token_origin().await?;
        let stream = self
            .client
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
        let mut value = HeaderValue::from_str(&format!("Bearer {token}"))?;
        value.set_sensitive(true);
        headers.insert(AUTHORIZATION, value);
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
