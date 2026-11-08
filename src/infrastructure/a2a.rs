use std::time::Duration;

use a2a_rust::{
    A2AClient, A2AClientConfig,
    types::{
        AgentInterface, CancelTaskRequest, GetTaskRequest, SendMessageConfiguration,
        SendMessageRequest, SendMessageResponse, SubscribeToTaskRequest,
    },
};

use anyhow::Result;
use futures_util::StreamExt;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use crate::domain::a2a::{
    A2aConnector, A2aSession, AgentCard, EventStream, Reply, SendRequest, SkillInfo, TaskInfo,
};
use crate::domain::config::A2aAgent;
use crate::infrastructure::config::{ensure_secure_transport, read_token};
use convert::{event_info, message_info, task_info, user_message};

pub struct A2aRustConnector {
    timeout: Option<Duration>,
}

impl A2aRustConnector {
    /// `timeout` は接続とリクエスト全体の制限時間。`None` なら制限しない。
    pub fn new(timeout: Option<Duration>) -> Self {
        Self { timeout }
    }
}

pub struct A2aRustSession {
    /// 通常のリクエスト用。全体の制限時間を持つ。
    request: A2AClient,
    /// `watch` 用。タスクが終わるまで続くので、接続の制限時間だけを持つ。
    stream: A2AClient,
    timeout: Option<Duration>,
    /// トークンを持つときだけ、送信先を縛る基準になる設定済みの URL。
    token_origin: Option<String>,
}

fn client(
    agent: &A2aAgent,
    headers: HeaderMap,
    timeout: Option<Duration>,
    whole_request: bool,
) -> Result<A2AClient> {
    let mut http = reqwest::Client::builder().default_headers(headers);
    if let Some(limit) = timeout {
        http = http.connect_timeout(limit);
        if whole_request {
            http = http.timeout(limit);
        }
    }
    Ok(A2AClient::with_http_client(
        &agent.url,
        http.build()?,
        A2AClientConfig::default(),
    )?)
}

impl A2aConnector for A2aRustConnector {
    type Session = A2aRustSession;

    fn connect(&self, agent: &A2aAgent) -> Result<A2aRustSession> {
        let token = read_token(&agent.token_env)?;
        let has_token = token.is_some();
        ensure_secure_transport(&agent.url, has_token, agent.allow_insecure)?;
        let headers = auth_headers(token)?;
        Ok(A2aRustSession {
            request: client(agent, headers.clone(), self.timeout, true)?,
            stream: client(agent, headers, self.timeout, false)?,
            timeout: self.timeout,
            token_origin: has_token.then(|| agent.url.clone()),
        })
    }
}

/// 制限時間の超過なら、制限時間と変更方法を示すエラーにする。
fn explain_timeout(err: impl Into<anyhow::Error>, timeout: Option<Duration>) -> anyhow::Error {
    let err = err.into();
    let timed_out = err.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(reqwest::Error::is_timeout)
    });
    match (timed_out, timeout) {
        (true, Some(limit)) => anyhow::anyhow!(
            "timed out after {} (change it with --timeout)",
            crate::infrastructure::timeout::describe(limit)
        ),
        _ => err,
    }
}

impl A2aRustSession {
    /// トークンを持つとき、カードが広告するエンドポイントが設定した URL と同じオリジンか確かめる。
    /// カードはクライアント内にキャッシュされ、続く送信先の選択も同じカードを使う。
    /// そのため、これから使うクライアントで確かめる。
    async fn guard_token_origin(&self, client: &A2AClient) -> Result<()> {
        if let Some(base) = &self.token_origin {
            let card = client
                .discover_agent_card()
                .await
                .map_err(|e| explain_timeout(e, self.timeout))?;
            ensure_same_origin(base, &card.supported_interfaces)?;
        }
        Ok(())
    }
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
        let card = self
            .request
            .discover_agent_card()
            .await
            .map_err(|e| explain_timeout(e, self.timeout))?;
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
        self.guard_token_origin(&self.request).await?;
        let req = SendMessageRequest {
            message: user_message(req.text, req.context_id, req.task_id),
            configuration: Some(SendMessageConfiguration {
                return_immediately: req.no_wait,
                ..Default::default()
            }),
            metadata: None,
            tenant: None,
        };
        Ok(
            match self
                .request
                .send_message(req)
                .await
                .map_err(|e| explain_timeout(e, self.timeout))?
            {
                SendMessageResponse::Message(m) => Reply::Message(message_info(&m)?),
                SendMessageResponse::Task(t) => Reply::Task(task_info(&t)?),
            },
        )
    }

    async fn get_task(&self, id: &str) -> Result<TaskInfo> {
        self.guard_token_origin(&self.request).await?;
        let req = GetTaskRequest {
            id: id.to_owned(),
            history_length: None,
            tenant: None,
        };
        task_info(
            &self
                .request
                .get_task(req)
                .await
                .map_err(|e| explain_timeout(e, self.timeout))?,
        )
    }

    async fn cancel_task(&self, id: &str) -> Result<TaskInfo> {
        self.guard_token_origin(&self.request).await?;
        let req = CancelTaskRequest {
            id: id.to_owned(),
            tenant: None,
            metadata: None,
        };
        task_info(
            &self
                .request
                .cancel_task(req)
                .await
                .map_err(|e| explain_timeout(e, self.timeout))?,
        )
    }

    async fn watch(&self, id: &str) -> Result<EventStream> {
        self.guard_token_origin(&self.stream).await?;
        let stream = self
            .stream
            .subscribe_to_task(SubscribeToTaskRequest {
                id: id.to_owned(),
                tenant: None,
            })
            .await
            .map_err(|e| explain_timeout(e, self.timeout))?;
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

mod convert;

#[cfg(test)]
mod tests;
