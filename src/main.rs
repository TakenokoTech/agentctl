mod application;
mod domain;
mod infrastructure;
mod presentation;

use std::process::ExitCode;

use clap::Parser;

use infrastructure::a2a::A2aRustConnector;
use infrastructure::mcp::RmcpConnector;
use presentation::cli::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

async fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let cfg = infrastructure::config::load(&cli.config)?;
    presentation::run(cli, &cfg, &RmcpConnector, &A2aRustConnector).await
}
