//! E2E テスト用の MCP サーバ(stdio)。ツール定義は `mcp_handler.rs` と共有する。
//!
//! cargo run --example test_mcp_server
#[path = "mcp_handler.rs"]
mod handler;

use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    handler::TestServer.serve(stdio()).await?.waiting().await?;
    Ok(())
}
