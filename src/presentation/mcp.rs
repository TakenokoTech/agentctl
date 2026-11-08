use std::process::ExitCode;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

use crate::application::mcp as app;
use crate::domain::config::McpServer;
use crate::domain::mcp::{Content, JsonObject, McpConnector, ToolInfo, ToolOutput};
use crate::presentation::EXIT_TOOL_FAILED;
use crate::presentation::cli::McpCmd;

pub async fn run<C: McpConnector>(
    connector: &C,
    server: &McpServer,
    cmd: McpCmd,
    raw: bool,
) -> Result<ExitCode> {
    match cmd {
        McpCmd::Tools => {
            for t in app::list_tools(connector, server).await? {
                println!("{}", tool_line(&t));
            }
            Ok(ExitCode::SUCCESS)
        }
        McpCmd::Schema { tool } => {
            let t = app::tool_schema(connector, server, &tool).await?;
            println!("{}", schema_json(&t));
            Ok(ExitCode::SUCCESS)
        }
        McpCmd::Call { tool, args } => {
            let arguments = read_args(args).await?;
            let out = app::call_tool(connector, server, &tool, arguments).await?;
            for line in render_output(&out, raw) {
                println!("{line}");
            }
            Ok(exit_code(&out))
        }
    }
}

/// 「呼び方が悪い」(1)と「tool が失敗を返した」(2)を区別する。
fn exit_code(out: &ToolOutput) -> ExitCode {
    if out.is_error {
        ExitCode::from(EXIT_TOOL_FAILED)
    } else {
        ExitCode::SUCCESS
    }
}

/// `-` のときだけ stdin から読む。省略時は `{}`。
async fn read_args(args: Option<String>) -> Result<JsonObject> {
    let text = match args.as_deref() {
        None => return Ok(JsonObject::new()),
        Some("-") => {
            let mut s = String::new();
            tokio::io::stdin().read_to_string(&mut s).await?;
            s
        }
        Some(s) => s.to_owned(),
    };
    parse_args(&text)
}

fn parse_args(text: &str) -> Result<JsonObject> {
    serde_json::from_str(text).context("--args must be a JSON object")
}

fn tool_line(t: &ToolInfo) -> String {
    let desc = t.description.as_deref().unwrap_or("");
    format!("{}\t{}", t.name, desc.lines().next().unwrap_or(""))
}

fn schema_json(t: &ToolInfo) -> Value {
    json!({
        "name": t.name,
        "description": t.description,
        "inputSchema": t.input_schema,
    })
}

/// structuredContent があればそれを、なければ text を素のまま出す。
/// text 以外の content(画像など)は JSON のまま出す。
fn render_output(out: &ToolOutput, raw: bool) -> Vec<String> {
    if raw {
        vec![out.raw.to_string()]
    } else if let Some(v) = &out.structured {
        vec![v.to_string()]
    } else {
        out.contents
            .iter()
            .map(|c| match c {
                Content::Text(t) => t.clone(),
                Content::Other(v) => v.to_string(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
