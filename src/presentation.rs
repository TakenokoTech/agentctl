pub mod a2a;
pub mod cli;
pub mod mcp;
pub mod output;

use std::process::ExitCode;

use anyhow::Result;

use crate::domain::a2a::A2aConnector;
use crate::domain::config::{Config, Entry};
use crate::domain::mcp::McpConnector;
use cli::{Cli, Cmd};

/// tool がエラーを返した / task が failed・rejected になった場合の終了コード。
pub const EXIT_TOOL_FAILED: u8 = 2;

pub async fn run<M: McpConnector, A: A2aConnector>(
    cli: Cli,
    cfg: &Config,
    mcp: &M,
    a2a: &A,
) -> Result<ExitCode> {
    match cli.cmd {
        Cmd::List => {
            for e in cfg.entries() {
                output::line(list_line(&e))?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Mcp { server, cmd } => mcp::run(mcp, cfg.mcp(&server)?, cmd, cli.raw).await,
        Cmd::A2a { agent, cmd } => a2a::run(a2a, cfg.a2a(&agent)?, cmd, cli.raw).await,
    }
}

fn list_line(e: &Entry) -> String {
    format!(
        "{}\t{}\t{}",
        e.kind.as_str(),
        output::tsv_field(e.name),
        output::tsv_field(e.description)
    )
}

#[cfg(test)]
mod tests;
