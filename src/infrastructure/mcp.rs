use std::{process::Stdio, time::Duration};

use anyhow::{Context, Result};
use rmcp::{
    RoleClient, ServiceExt,
    model::{CallToolRequestParams, CallToolResult, Tool},
    service::RunningService,
    transport::{
        StreamableHttpClientTransport, TokioChildProcess,
        streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use tokio::process::Command;

use crate::domain::config::{McpServer, McpTransport};
use crate::domain::mcp::{Content, JsonObject, McpConnector, McpSession, ToolInfo, ToolOutput};
use crate::infrastructure::config::{ensure_secure_transport, read_token};
use crate::infrastructure::timeout::within;
use stderr_tail::StderrTail;

pub struct RmcpConnector {
    timeout: Option<Duration>,
}

impl RmcpConnector {
    /// `timeout` は、接続・一覧・ツール呼び出しそれぞれの制限時間。`None` なら制限しない。
    pub fn new(timeout: Option<Duration>) -> Self {
        Self { timeout }
    }
}

pub struct RmcpSession {
    service: RunningService<RoleClient, ()>,
    timeout: Option<Duration>,
}

impl McpConnector for RmcpConnector {
    type Session = RmcpSession;

    async fn connect(&self, server: &McpServer) -> Result<RmcpSession> {
        let service = match &server.transport {
            McpTransport::Stdio { command, env } => {
                let (prog, args) = command.split_first().context("`command` is empty")?;
                let mut cmd = Command::new(prog);
                cmd.args(args).envs(env);
                // 通常はサーバのログを捨てて、エージェントのコンテキストに混ぜない。
                // 起動に失敗したときだけ、末尾をエラーに含める
                let (transport, stderr) = TokioChildProcess::builder(cmd)
                    .stderr(Stdio::piped())
                    .spawn()
                    .with_context(|| format!("failed to spawn {prog}"))?;
                let tail = stderr.map(StderrTail::start);
                let connected = within(self.timeout, "connecting to the MCP server", async {
                    Ok(().serve(transport).await?)
                })
                .await;
                match (connected, tail) {
                    (Ok(service), _) => service,
                    (Err(e), Some(tail)) => return Err(tail.explain(e).await),
                    (Err(e), None) => return Err(e),
                }
            }
            McpTransport::Http {
                url,
                token_env,
                allow_insecure,
            } => {
                let mut cfg = StreamableHttpClientTransportConfig::with_uri(url.as_str());
                if let Some(token) = read_token(token_env)? {
                    ensure_secure_transport(url, true, *allow_insecure)?;
                    cfg = cfg.auth_header(token);
                }
                within(self.timeout, "connecting to the MCP server", async {
                    Ok(
                        ().serve(StreamableHttpClientTransport::from_config(cfg))
                            .await?,
                    )
                })
                .await?
            }
        };
        Ok(RmcpSession {
            service,
            timeout: self.timeout,
        })
    }
}

impl McpSession for RmcpSession {
    async fn list_tools(&self) -> Result<Vec<ToolInfo>> {
        within(self.timeout, "listing the tools", async {
            Ok(self.service.list_all_tools().await?)
        })
        .await?
        .iter()
        .map(tool_info)
        .collect()
    }

    async fn call_tool(&self, name: &str, args: JsonObject) -> Result<ToolOutput> {
        let params = CallToolRequestParams::new(name.to_owned()).with_arguments(args);
        let res = within(self.timeout, "calling the tool", async {
            Ok(self.service.call_tool(params).await?)
        })
        .await?;
        to_output(&res)
    }

    async fn close(self) {
        // 応答しないサーバーで止まらないよう、切断にも上限を設ける
        let _ = tokio::time::timeout(CLOSE_LIMIT, self.service.cancel()).await;
    }
}

/// 切断を待つ上限。
const CLOSE_LIMIT: Duration = Duration::from_secs(5);

fn tool_info(t: &Tool) -> Result<ToolInfo> {
    Ok(ToolInfo {
        name: t.name.to_string(),
        description: t.description.as_deref().map(str::to_owned),
        input_schema: serde_json::to_value(&t.input_schema)?,
    })
}

fn to_output(res: &CallToolResult) -> Result<ToolOutput> {
    let contents = res
        .content
        .iter()
        .map(|c| match c.as_text() {
            Some(t) => Ok(Content::Text(t.text.clone())),
            None => Ok(Content::Other(serde_json::to_value(c)?)),
        })
        .collect::<Result<_>>()?;
    Ok(ToolOutput {
        is_error: res.is_error == Some(true),
        structured: res.structured_content.clone(),
        contents,
        raw: serde_json::to_value(res)?,
    })
}

mod stderr_tail;

#[cfg(test)]
mod tests;
