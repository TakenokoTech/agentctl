//! E2E テスト用の A2A エージェント。
//! - 通常のテキスト: Message で即返す
//! - "slow ..." : Task を作り、2秒後に completed + artifact
//! - "fail ..." : failed 状態の Task をすぐ返す

use a2a_rust::{
    A2AError,
    server::{A2AHandler, A2AStream},
    types::*,
};
use futures_util::{StreamExt, stream};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
#[derive(Clone, Default)]
pub struct TestAgent {
    tasks: Arc<Mutex<HashMap<String, Task>>>,
}
fn text_part(s: &str) -> Part {
    Part {
        text: Some(s.to_owned()),
        raw: None,
        url: None,
        data: None,
        metadata: None,
        filename: None,
        media_type: None,
    }
}
fn status(state: TaskState) -> TaskStatus {
    TaskStatus {
        state,
        message: None,
        timestamp: None,
    }
}
fn is_terminal(s: &TaskState) -> bool {
    matches!(
        s,
        TaskState::Completed | TaskState::Failed | TaskState::Canceled | TaskState::Rejected
    )
}
impl TestAgent {
    fn snapshot(&self, id: &str) -> Result<Task, A2AError> {
        self.tasks
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| A2AError::TaskNotFound(id.to_owned()))
    }
}
#[async_trait::async_trait]
impl A2AHandler for TestAgent {
    async fn get_agent_card(&self) -> Result<AgentCard, A2AError> {
        let iface = |url: &str, binding: &str| AgentInterface {
            url: url.to_owned(),
            protocol_binding: binding.to_owned(),
            tenant: None,
            protocol_version: "1.0".to_owned(),
        };
        Ok(AgentCard {
            name: "Test Agent".to_owned(),
            description: "Echoes text. Prefix with 'slow' to get a long-running task.".to_owned(),
            supported_interfaces: vec![iface("/", "HTTP+JSON"), iface("/rpc", "JSONRPC")],
            provider: None,
            version: "0.1.0".to_owned(),
            documentation_url: None,
            capabilities: AgentCapabilities {
                streaming: Some(true),
                push_notifications: Some(false),
                extensions: Vec::new(),
                extended_agent_card: Some(false),
            },
            security_schemes: Default::default(),
            security_requirements: Vec::new(),
            default_input_modes: vec!["text/plain".to_owned()],
            default_output_modes: vec!["text/plain".to_owned()],
            skills: vec![AgentSkill {
                id: "echo".to_owned(),
                name: "Echo".to_owned(),
                description: "Returns the input text".to_owned(),
                tags: vec!["test".to_owned()],
                examples: vec!["hello".to_owned(), "slow report".to_owned()],
                input_modes: Vec::new(),
                output_modes: Vec::new(),
                security_requirements: Vec::new(),
            }],
            signatures: Vec::new(),
            icon_url: None,
        })
    }
    async fn send_message(&self, req: SendMessageRequest) -> Result<SendMessageResponse, A2AError> {
        let text = req
            .message
            .parts
            .iter()
            .find_map(|p| p.text.clone())
            .unwrap_or_default();
        let context_id = req
            .message
            .context_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if text.starts_with("fail") {
            let id = uuid::Uuid::new_v4().to_string();
            let task = Task {
                id: id.clone(),
                context_id: Some(context_id),
                status: status(TaskState::Failed),
                artifacts: Vec::new(),
                history: vec![req.message],
                metadata: None,
            };
            self.tasks.lock().unwrap().insert(id, task.clone());
            return Ok(SendMessageResponse::Task(task));
        }
        // "drop ...": 終端状態に達しないまま、watch のストリームだけ閉じる
        // "linger ...": すぐ完了するが、終端状態のあとも watch のストリームを閉じない
        if text.starts_with("drop") || text.starts_with("linger") {
            let mode = if text.starts_with("drop") {
                "drop"
            } else {
                "linger"
            };
            let id = uuid::Uuid::new_v4().to_string();
            let task = Task {
                id: id.clone(),
                context_id: Some(context_id),
                status: status(TaskState::Working),
                artifacts: Vec::new(),
                history: vec![req.message],
                metadata: serde_json::json!({ "mode": mode }).as_object().cloned(),
            };
            self.tasks.lock().unwrap().insert(id.clone(), task.clone());
            if mode == "linger" {
                let tasks = self.tasks.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    if let Some(t) = tasks.lock().unwrap().get_mut(&id) {
                        t.status = status(TaskState::Completed);
                    }
                });
            }
            return Ok(SendMessageResponse::Task(task));
        }
        if !text.starts_with("slow") {
            return Ok(SendMessageResponse::Message(Message {
                message_id: uuid::Uuid::new_v4().to_string(),
                context_id: Some(context_id),
                task_id: None,
                role: Role::Agent,
                parts: vec![text_part(&format!("echo: {text}"))],
                metadata: None,
                extensions: Vec::new(),
                reference_task_ids: Vec::new(),
            }));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let task = Task {
            id: id.clone(),
            context_id: Some(context_id),
            status: status(TaskState::Working),
            artifacts: Vec::new(),
            history: vec![req.message],
            metadata: None,
        };
        self.tasks.lock().unwrap().insert(id.clone(), task.clone());
        let tasks = self.tasks.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if let Some(t) = tasks.lock().unwrap().get_mut(&id) {
                if is_terminal(&t.status.state) {
                    return;
                }
                t.status = status(TaskState::Completed);
                t.artifacts.push(Artifact {
                    artifact_id: "result".to_owned(),
                    name: Some("result".to_owned()),
                    description: None,
                    parts: vec![text_part(&format!("done: {text}"))],
                    metadata: None,
                    extensions: Vec::new(),
                });
            }
        });
        Ok(SendMessageResponse::Task(task))
    }
    async fn get_task(&self, req: GetTaskRequest) -> Result<Task, A2AError> {
        self.snapshot(&req.id)
    }
    async fn cancel_task(&self, req: CancelTaskRequest) -> Result<Task, A2AError> {
        let mut tasks = self.tasks.lock().unwrap();
        let t = tasks
            .get_mut(&req.id)
            .ok_or_else(|| A2AError::TaskNotFound(req.id.clone()))?;
        if !is_terminal(&t.status.state) {
            t.status = status(TaskState::Canceled);
        }
        Ok(t.clone())
    }
    /// 現在の Task を1回流し、その後は状態が変わるたびに StatusUpdate を流す。終端で閉じる
    async fn subscribe_to_task(&self, req: SubscribeToTaskRequest) -> Result<A2AStream, A2AError> {
        let first = self.snapshot(&req.id)?;
        let mode = first
            .metadata
            .as_ref()
            .and_then(|m| m.get("mode"))
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        if mode.as_deref() == Some("drop") {
            let only_snapshot = stream::iter(vec![StreamResponse::Task(first)]);
            return Ok(Box::pin(only_snapshot));
        }
        let agent = self.clone();
        let init = (Some(first.clone()), first.status.state, false);
        let s = stream::unfold(
            (agent, req.id, init),
            |(agent, id, (first, last, done))| async move {
                if done {
                    return None;
                }
                if let Some(t) = first {
                    let fin = is_terminal(&t.status.state);
                    return Some((StreamResponse::Task(t), (agent, id, (None, last, fin))));
                }
                loop {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let t = agent.snapshot(&id).ok()?;
                    if t.status.state != last {
                        let fin = is_terminal(&t.status.state);
                        let ev = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
                            task_id: t.id.clone(),
                            context_id: t.context_id.clone().unwrap_or_default(),
                            status: t.status.clone(),
                            metadata: None,
                        });
                        return Some((ev, (agent, id, (None, t.status.state, fin))));
                    }
                }
            },
        );
        if mode.as_deref() == Some("linger") {
            return Ok(Box::pin(s.chain(stream::pending())));
        }
        Ok(Box::pin(s))
    }
}
