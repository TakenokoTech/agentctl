use std::process::ExitCode;

use anyhow::{Result, bail};
use serde::Serialize;
use serde_json::{Value, json};

use crate::application::a2a as app;
use crate::domain::a2a::{
    A2aConnector, AgentCard, MessageInfo, Reply, SendRequest, TaskEvent, TaskInfo, TaskState,
};
use crate::domain::config::A2aAgent;
use crate::presentation::EXIT_TOOL_FAILED;
use crate::presentation::cli::{A2aCmd, TaskCmd};

pub async fn run<C: A2aConnector>(
    connector: &C,
    agent: &A2aAgent,
    cmd: A2aCmd,
    raw: bool,
) -> Result<ExitCode> {
    match cmd {
        A2aCmd::Card => {
            let card = app::card(connector, agent).await?;
            emit(&card_json(&card, raw))
        }
        A2aCmd::Send {
            text,
            context_id,
            task_id,
            no_wait,
        } => {
            let req = SendRequest {
                text,
                context_id,
                task_id,
                no_wait,
            };
            match app::send(connector, agent, req).await? {
                Reply::Message(m) => emit(&message_json(&m, raw)),
                Reply::Task(t) => emit_task(&t, raw),
            }
        }
        A2aCmd::Task {
            cmd: TaskCmd::Get { id },
        } => emit_task(&app::get_task(connector, agent, &id).await?, raw),
        A2aCmd::Task {
            cmd: TaskCmd::Cancel { id },
        } => emit_task(&app::cancel_task(connector, agent, &id).await?, raw),
        A2aCmd::Task {
            cmd: TaskCmd::Watch { id },
        } => {
            let last = app::watch(connector, agent, &id, |ev| {
                emit(&event_json(ev, raw)).map(drop)
            })
            .await?;
            watch_exit(last.as_ref())
        }
    }
}

/// ストリームが閉じたときの終了コード。最後の状態がまだ進行中なら、完了を確認できていない。
fn watch_exit(last: Option<&TaskState>) -> Result<ExitCode> {
    match last {
        Some(s) if s.is_settled() => Ok(exit_code(s)),
        Some(s) => bail!(
            "the stream closed before the task finished (last state: {})",
            s.name()
        ),
        None => bail!("the stream closed before the task finished (no state received)"),
    }
}

fn emit(v: &impl Serialize) -> Result<ExitCode> {
    println!("{}", serde_json::to_string(v)?);
    Ok(ExitCode::SUCCESS)
}

fn emit_task(t: &TaskInfo, raw: bool) -> Result<ExitCode> {
    emit(&task_json(t, raw))?;
    Ok(exit_code(&t.state))
}

/// 呼び方は正しいが処理が失敗した場合を、終了コード 2 で区別する。
fn exit_code(s: &TaskState) -> ExitCode {
    if s.is_failure() {
        ExitCode::from(EXIT_TOOL_FAILED)
    } else {
        ExitCode::SUCCESS
    }
}

fn card_json(card: &AgentCard, raw: bool) -> Value {
    if raw {
        return card.raw.clone();
    }
    let skills: Vec<Value> = card
        .skills
        .iter()
        .map(|s| json!({"id": s.id, "description": s.description, "examples": s.examples}))
        .collect();
    json!({
        "name": card.name,
        "description": card.description,
        "streaming": card.streaming,
        "skills": skills,
    })
}

fn message_json(m: &MessageInfo, raw: bool) -> Value {
    if raw {
        m.raw.clone()
    } else {
        json!({"type": "message", "contextId": m.context_id, "text": m.text})
    }
}

fn task_json(t: &TaskInfo, raw: bool) -> Value {
    if raw {
        return t.raw.clone();
    }
    let artifacts: Vec<Value> = t
        .artifacts
        .iter()
        .map(|a| json!({"name": a.name, "text": a.text}))
        .collect();
    json!({
        "type": "task",
        "id": t.id,
        "contextId": t.context_id,
        "state": t.state.name(),
        "message": t.message,
        "artifacts": artifacts,
    })
}

fn event_json(ev: &TaskEvent, raw: bool) -> Value {
    if raw {
        return ev.raw().clone();
    }
    match ev {
        TaskEvent::Task(t) => task_json(t, false),
        TaskEvent::Message(m) => message_json(m, false),
        TaskEvent::Status {
            task_id,
            state,
            message,
            ..
        } => json!({
            "type": "status",
            "taskId": task_id,
            "state": state.name(),
            "message": message,
        }),
        TaskEvent::Artifact {
            task_id,
            name,
            text,
            append,
            last_chunk,
            ..
        } => json!({
            "type": "artifact",
            "taskId": task_id,
            "name": name,
            "text": text,
            "append": append,
            "lastChunk": last_chunk,
        }),
    }
}

#[cfg(test)]
mod tests;
