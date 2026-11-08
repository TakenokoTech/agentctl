//! MCP サーバー本体。ツール呼び出しを `agentctl` の実行に変換する。
use std::sync::Arc;

use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, model::*, service::RequestContext};

use crate::{command::to_invocation, runner::Runner, tools::tools};

pub struct Server {
    runner: Arc<Runner>,
}

impl Server {
    pub fn new(runner: Runner) -> Self {
        Self {
            runner: Arc::new(runner),
        }
    }
}

impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let args = request.arguments.unwrap_or_default();
        let inv =
            to_invocation(&request.name, args).map_err(|e| McpError::invalid_params(e, None))?;
        let outcome = self.runner.run(&inv).await;
        let content = vec![ContentBlock::text(outcome.text)];
        Ok(if outcome.is_error {
            CallToolResult::error(content)
        } else {
            CallToolResult::success(content)
        }
        .into())
    }
}
