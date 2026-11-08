//! MCP ツールの定義。ツール名は `agentctl` のサブコマンドに対応する(`mcp call` → `mcp_call`)。
//! `a2a task watch` はストリームを返す操作で、ツール呼び出しの1回の応答に収まらないため公開しない。
use std::sync::Arc;

use rmcp::model::Tool;
use serde_json::{Value, json};

fn tool(
    name: &'static str,
    description: &'static str,
    properties: Value,
    required: &[&str],
) -> Tool {
    let schema = json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    });
    Tool::new(
        name,
        description,
        Arc::new(schema.as_object().cloned().unwrap_or_default()),
    )
}

fn string(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

fn boolean(description: &str) -> Value {
    json!({"type": "boolean", "description": description})
}

fn raw_property() -> Value {
    boolean("Return the full protocol response instead of a compact summary")
}

pub fn tools() -> Vec<Tool> {
    let server = string("MCP server name (see `list`)");
    let agent = string("A2A agent name (see `list`)");
    vec![
        tool(
            "list",
            "List configured MCP servers and A2A agents (TSV: kind, name, description)",
            json!({}),
            &[],
        ),
        tool(
            "mcp_tools",
            "List an MCP server's tools (TSV: name, first line of description)",
            json!({"server": server}),
            &["server"],
        ),
        tool(
            "mcp_schema",
            "Show a tool's full description and input JSON Schema",
            json!({"server": server, "tool": string("Tool name (see `mcp_tools`)")}),
            &["server", "tool"],
        ),
        tool(
            "mcp_call",
            "Call a tool on an MCP server and print its result",
            json!({
                "server": server,
                "tool": string("Tool name (see `mcp_tools`)"),
                "args": {"type": "object", "description": "Arguments for the tool. Default: {}"},
                "raw": raw_property(),
            }),
            &["server", "tool"],
        ),
        tool(
            "a2a_card",
            "Show an A2A agent's card (description, skills, streaming support)",
            json!({"agent": agent, "raw": raw_property()}),
            &["agent"],
        ),
        tool(
            "a2a_send",
            "Send a text message to an A2A agent. Returns the reply message, or the task if one was created",
            json!({
                "agent": agent,
                "text": string("Message text"),
                "context_id": string("Continue an existing conversation"),
                "task_id": string("Answer a task that is in `input_required`"),
                "no_wait": boolean("Return as soon as the task is accepted; follow up with `a2a_task_get`"),
                "raw": raw_property(),
            }),
            &["agent", "text"],
        ),
        tool(
            "a2a_task_get",
            "Show an A2A task's current state",
            json!({"agent": agent, "id": string("Task id"), "raw": raw_property()}),
            &["agent", "id"],
        ),
        tool(
            "a2a_task_cancel",
            "Cancel an A2A task",
            json!({"agent": agent, "id": string("Task id"), "raw": raw_property()}),
            &["agent", "id"],
        ),
    ]
}

#[cfg(test)]
mod tests;
