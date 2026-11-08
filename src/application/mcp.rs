use anyhow::{Context, Result};

use crate::domain::config::McpServer;
use crate::domain::mcp::{JsonObject, McpConnector, McpSession, ToolInfo, ToolOutput};

/// 接続し、処理を実行し、結果にかかわらず必ず切断する。
async fn with_session<C: McpConnector, T>(
    connector: &C,
    server: &McpServer,
    f: impl AsyncFnOnce(&C::Session) -> Result<T>,
) -> Result<T> {
    let session = connector.connect(server).await?;
    let result = f(&session).await;
    session.close().await;
    result
}

pub async fn list_tools<C: McpConnector>(
    connector: &C,
    server: &McpServer,
) -> Result<Vec<ToolInfo>> {
    with_session(connector, server, async |s| s.list_tools().await).await
}

pub async fn tool_schema<C: McpConnector>(
    connector: &C,
    server: &McpServer,
    tool: &str,
) -> Result<ToolInfo> {
    with_session(connector, server, async |s| {
        s.list_tools()
            .await?
            .into_iter()
            .find(|t| t.name == tool)
            .with_context(|| format!("unknown tool '{tool}' (see `tools`)"))
    })
    .await
}

pub async fn call_tool<C: McpConnector>(
    connector: &C,
    server: &McpServer,
    tool: &str,
    args: JsonObject,
) -> Result<ToolOutput> {
    with_session(connector, server, async |s| s.call_tool(tool, args).await).await
}

#[cfg(test)]
mod tests;
