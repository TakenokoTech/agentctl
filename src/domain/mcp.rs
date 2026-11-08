use anyhow::Result;
use serde_json::{Map, Value};

use crate::domain::config::McpServer;

pub type JsonObject = Map<String, Value>;

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    Text(String),
    /// text 以外(画像など)。JSON のまま保持する。
    Other(Value),
}

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub is_error: bool,
    pub structured: Option<Value>,
    pub contents: Vec<Content>,
    /// プロトコルの応答そのまま(`--raw` 用)。
    pub raw: Value,
}

/// 接続済みの MCP サーバとのセッション。
#[allow(async_fn_in_trait)]
pub trait McpSession {
    async fn list_tools(&self) -> Result<Vec<ToolInfo>>;
    async fn call_tool(&self, name: &str, args: JsonObject) -> Result<ToolOutput>;
    async fn close(self);
}

#[allow(async_fn_in_trait)]
pub trait McpConnector {
    type Session: McpSession;

    async fn connect(&self, server: &McpServer) -> Result<Self::Session>;
}
