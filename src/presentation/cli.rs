use std::{path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};

/// Call MCP servers and A2A agents from one CLI.
///
/// Start with `agentctl list`, then drill down:
/// `mcp <server> tools` -> `mcp <server> schema <tool>` -> `mcp <server> call <tool>`,
/// `a2a <agent> card` -> `a2a <agent> send <text>`.
///
/// Output is JSON on stdout (except `list` and `tools`, which are TSV).
/// Exit codes: 0 ok, 1 error (config/transport/protocol), 2 tool error or task failed.
#[derive(Parser)]
#[command(name = "agentctl", version, verbatim_doc_comment)]
pub struct Cli {
    /// Config file path
    #[arg(
        long,
        env = "AGENTCTL_CONFIG",
        default_value = "agentctl.toml",
        global = true
    )]
    pub config: PathBuf,
    /// Print full protocol responses instead of compact summaries
    #[arg(long, global = true)]
    pub raw: bool,
    /// Give up on a connection or request after this many seconds (0 = no limit).
    /// `a2a task watch` limits only the connection, since it runs until the task ends
    #[arg(
        long,
        env = "AGENTCTL_TIMEOUT",
        default_value_t = 60,
        global = true,
        value_name = "SECONDS"
    )]
    timeout: u64,
    #[command(subcommand)]
    pub cmd: Cmd,
}

impl Cli {
    /// 接続とリクエストの制限時間。`0` は制限なし。
    pub fn timeout(&self) -> Option<Duration> {
        (self.timeout > 0).then(|| Duration::from_secs(self.timeout))
    }
}

#[derive(Subcommand)]
pub enum Cmd {
    /// List configured MCP servers and A2A agents (TSV: kind, name, description)
    List,
    /// Call an MCP server
    Mcp {
        server: String,
        #[command(subcommand)]
        cmd: McpCmd,
    },
    /// Call an A2A agent
    A2a {
        agent: String,
        #[command(subcommand)]
        cmd: A2aCmd,
    },
}

#[derive(Subcommand)]
pub enum McpCmd {
    /// List tools (TSV: name, first line of description)
    Tools,
    /// Show a tool's full description and input JSON schema
    Schema { tool: String },
    /// Call a tool and print its result
    Call {
        tool: String,
        /// Arguments as a JSON object. Use `-` to read from stdin. Default: {}
        #[arg(long)]
        args: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum A2aCmd {
    /// Show the agent card (description and skills)
    Card,
    /// Send a text message. Prints the reply message, or the task if one was created
    Send {
        /// Message text. May start with `-`; use `--` before it to be explicit
        #[arg(allow_hyphen_values = true)]
        text: String,
        /// Continue an existing conversation
        #[arg(long)]
        context_id: Option<String>,
        /// Answer a task that is in `input_required`
        #[arg(long)]
        task_id: Option<String>,
        /// Return as soon as the task is accepted; follow up with `task get` or `task watch`
        #[arg(long)]
        no_wait: bool,
    },
    /// Operate on an existing task
    Task {
        #[command(subcommand)]
        cmd: TaskCmd,
    },
}

#[derive(Subcommand)]
pub enum TaskCmd {
    /// Print the current task state
    Get { id: String },
    /// Stream task updates as NDJSON until the task reaches a final state
    Watch { id: String },
    /// Cancel a task
    Cancel { id: String },
}

#[cfg(test)]
mod tests;
