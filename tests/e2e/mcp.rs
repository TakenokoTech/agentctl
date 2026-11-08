//! MCP サブコマンドの E2E テスト。
use std::time::Duration;

use serde_json::json;

use crate::common::{run, run_stdin, run_with, run_with_open_stdin, write_config};

#[test]
fn mcp_tools_lists_name_and_first_line_of_description() {
    let out = run(&["mcp", "test", "tools"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.lines(),
        vec![
            "echo\tEcho the message",
            "structured\tReturn structured content",
            "fail\tAlways return an error result",
        ]
    );
}

#[test]
fn mcp_schema_prints_full_description_and_input_schema() {
    let out = run(&["mcp", "test", "schema", "echo"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = out.json();
    assert_eq!(v["name"], "echo");
    assert_eq!(v["description"], "Echo the message\nSecond line");
    assert_eq!(v["inputSchema"]["required"], json!(["message"]));
}

#[test]
fn mcp_schema_of_unknown_tool_exits_1() {
    let out = run(&["mcp", "test", "schema", "nope"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("unknown tool 'nope'"), "{}", out.stderr);
}

#[test]
fn mcp_call_passes_args_and_prints_text() {
    let out = run(&[
        "mcp",
        "test",
        "call",
        "echo",
        "--args",
        r#"{"message":"hi"}"#,
    ]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.lines(), vec!["echo: hi"]);
}

#[test]
fn mcp_call_reads_args_from_stdin_when_dash() {
    let out = run_stdin(
        &["mcp", "test", "call", "echo", "--args", "-"],
        r#"{"message":"from stdin"}"#,
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.lines(), vec!["echo: from stdin"]);
}

#[test]
fn mcp_call_without_args_does_not_wait_for_stdin() {
    let out = run_with_open_stdin(
        &["mcp", "test", "call", "structured"],
        Duration::from_secs(20),
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.json(), json!({"args": {}}));
}

#[test]
fn mcp_call_prefers_structured_content_over_text() {
    let out = run(&["mcp", "test", "call", "structured", "--args", r#"{"x":1}"#]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.lines(), vec![r#"{"args":{"x":1}}"#]);
}

#[test]
fn mcp_tool_error_result_exits_2_and_still_prints_it() {
    let out = run(&["mcp", "test", "call", "fail"]);
    assert_eq!(out.code, 2);
    assert_eq!(out.lines(), vec!["boom"]);
}

#[test]
fn mcp_call_with_wrong_argument_type_exits_2() {
    let out = run(&[
        "mcp",
        "test",
        "call",
        "echo",
        "--args",
        r#"{"message":123}"#,
    ]);
    assert_eq!(out.code, 2);
    assert_eq!(out.lines(), vec!["message must be a string"]);
}

#[test]
fn mcp_call_with_invalid_json_exits_1() {
    let out = run(&["mcp", "test", "call", "echo", "--args", "oops"]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("--args must be a JSON object"),
        "{}",
        out.stderr
    );
}

#[test]
fn mcp_call_raw_prints_the_protocol_response() {
    let out = run(&[
        "--raw",
        "mcp",
        "test",
        "call",
        "echo",
        "--args",
        r#"{"message":"hi"}"#,
    ]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.json()["content"][0]["text"], "echo: hi");
}

#[test]
fn mcp_unknown_server_exits_1() {
    let out = run(&["mcp", "nope", "tools"]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("unknown MCP server 'nope'"),
        "{}",
        out.stderr
    );
}

#[test]
fn mcp_server_that_cannot_be_spawned_exits_1() {
    let path = write_config(
        "bad-command",
        "[mcp.bad]\ntransport = \"stdio\"\ncommand = [\"/nonexistent/server\"]\n",
    );
    let out = run_with(&path, &["mcp", "bad", "tools"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("failed to spawn"), "{}", out.stderr);
}

#[test]
fn mcp_empty_command_exits_1() {
    let path = write_config(
        "empty-command",
        "[mcp.bad]\ntransport = \"stdio\"\ncommand = []\n",
    );
    let out = run_with(&path, &["mcp", "bad", "tools"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("`command` is empty"), "{}", out.stderr);
}
