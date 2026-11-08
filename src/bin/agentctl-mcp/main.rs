//! `agentctl` を MCP ツールとして公開する MCP サーバー(stdio)。
//!
//! 各ツール呼び出しを `agentctl` のサブプロセス実行に変換するだけで、`agentctl` の内部には依存しない。
mod command;
mod runner;
mod server;
mod tools;

use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    server::Server::new(runner::Runner::from_env())
        .serve(stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
