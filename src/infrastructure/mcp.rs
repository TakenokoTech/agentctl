use std::process::Stdio;

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

pub struct RmcpConnector;

pub struct RmcpSession(RunningService<RoleClient, ()>);

impl McpConnector for RmcpConnector {
    type Session = RmcpSession;

    async fn connect(&self, server: &McpServer) -> Result<RmcpSession> {
        let service = match &server.transport {
            McpTransport::Stdio { command, env } => {
                let (prog, args) = command.split_first().context("`command` is empty")?;
                let mut cmd = Command::new(prog);
                cmd.args(args).envs(env);
                // サーバのログがエージェントのコンテキストに混ざらないよう捨てる
                let (transport, _) = TokioChildProcess::builder(cmd)
                    .stderr(Stdio::null())
                    .spawn()
                    .with_context(|| format!("failed to spawn {prog}"))?;
                ().serve(transport).await?
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
                ().serve(StreamableHttpClientTransport::from_config(cfg))
                    .await?
            }
        };
        Ok(RmcpSession(service))
    }
}

impl McpSession for RmcpSession {
    async fn list_tools(&self) -> Result<Vec<ToolInfo>> {
        self.0
            .list_all_tools()
            .await?
            .iter()
            .map(tool_info)
            .collect()
    }

    async fn call_tool(&self, name: &str, args: JsonObject) -> Result<ToolOutput> {
        let params = CallToolRequestParams::new(name.to_owned()).with_arguments(args);
        let res = self.0.call_tool(params).await?;
        to_output(&res)
    }

    async fn close(self) {
        let _ = self.0.cancel().await;
    }
}

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

#[cfg(test)]
mod tests;
