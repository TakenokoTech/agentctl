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
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            return presentation::usage_exit_code(&e);
        }
    };
    match run(cli).await {
        Ok(code) => code,
        // `| head` などで出力先が閉じられた。エラーではなく、静かに終える
        Err(e) if presentation::output::is_broken_pipe(&e) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

async fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let cfg = infrastructure::config::load(&cli.config)?;
    let timeout = cli.timeout();
    presentation::run(
        cli,
        &cfg,
        &RmcpConnector::new(timeout),
        &A2aRustConnector::new(timeout),
    )
    .await
}
