use anyhow::Result;
use futures_util::StreamExt;

use crate::domain::a2a::{
    A2aConnector, A2aSession, AgentCard, Reply, SendRequest, TaskEvent, TaskInfo, WatchEnd,
};
use crate::domain::config::A2aAgent;

pub async fn card<C: A2aConnector>(connector: &C, agent: &A2aAgent) -> Result<AgentCard> {
    connector.connect(agent)?.card().await
}

pub async fn send<C: A2aConnector>(
    connector: &C,
    agent: &A2aAgent,
    req: SendRequest,
) -> Result<Reply> {
    connector.connect(agent)?.send(req).await
}

pub async fn get_task<C: A2aConnector>(
    connector: &C,
    agent: &A2aAgent,
    id: &str,
) -> Result<TaskInfo> {
    connector.connect(agent)?.get_task(id).await
}

pub async fn cancel_task<C: A2aConnector>(
    connector: &C,
    agent: &A2aAgent,
    id: &str,
) -> Result<TaskInfo> {
    connector.connect(agent)?.cancel_task(id).await
}

/// 終端状態に達するまで、イベントを `on_event` に渡す。
///
/// 終端状態を受けたら、サーバーがストリームを閉じるのを待たずに終わる。終端状態に達する前に
/// ストリームが閉じたら、最後に観測した状態を `Interrupted` で返す。
pub async fn watch<C: A2aConnector>(
    connector: &C,
    agent: &A2aAgent,
    id: &str,
    mut on_event: impl FnMut(&TaskEvent) -> Result<()>,
) -> Result<WatchEnd> {
    let session = connector.connect(agent)?;
    let mut stream = session.watch(id).await?;
    let mut last = None;
    while let Some(ev) = stream.next().await {
        let ev = ev?;
        on_event(&ev)?;
        if let Some(state) = ev.state() {
            if state.is_terminal() {
                return Ok(WatchEnd::Finished(state.clone()));
            }
            last = Some(state.clone());
        }
    }
    Ok(WatchEnd::Interrupted(last))
}

#[cfg(test)]
mod tests;
