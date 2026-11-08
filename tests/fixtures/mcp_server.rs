//! E2E テスト用の MCP サーバ(stdio)。
//! - echo: text を返す
//! - structured: structuredContent を返す
//! - fail: isError の結果を返す
//!
//! cargo run --example test_mcp_server
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt, model::*,
    service::RequestContext, transport::stdio,
};
use serde_json::{Value, json};

struct TestServer;

fn schema(v: Value) -> Arc<JsonObject> {
    Arc::new(v.as_object().cloned().unwrap_or_default())
}

impl ServerHandler for TestServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let message = json!({
            "type": "object",
            "properties": {"message": {"type": "string"}},
            "required": ["message"],
        });
        Ok(ListToolsResult::with_all_items(vec![
            Tool::new("echo", "Echo the message\nSecond line", schema(message)),
            Tool::new(
                "structured",
                "Return structured content",
                schema(json!({"type": "object"})),
            ),
            Tool::new(
                "fail",
                "Always return an error result",
                schema(json!({"type": "object"})),
            ),
        ]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let args = request.arguments.unwrap_or_default();
        let result = match &*request.name {
            "echo" => match args.get("message").and_then(Value::as_str) {
                Some(m) => CallToolResult::success(vec![ContentBlock::text(format!("echo: {m}"))]),
                None => CallToolResult::error(vec![ContentBlock::text("message must be a string")]),
            },
            "structured" => CallToolResult::structured(json!({"args": args})),
            "fail" => CallToolResult::error(vec![ContentBlock::text("boom")]),
            other => {
                return Err(McpError::invalid_params(
                    format!("unknown tool {other}"),
                    None,
                ));
            }
        };
        Ok(result.into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    TestServer.serve(stdio()).await?.waiting().await?;
    Ok(())
}
