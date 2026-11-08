//! MCP ラッパー(`agentctl-mcp`)と本物の `agentctl` を結合する E2E テスト。
//!
//! rmcp のクライアントで実際に接続し、ラッパーが起動する `agentctl` を経由して、
//! その先のテスト用 MCP サーバーと A2A エージェントまで通す。
use std::{path::PathBuf, process::Stdio};

use rmcp::{
    RoleClient, ServiceExt, model::CallToolRequestParams, service::RunningService,
    transport::TokioChildProcess,
};
use serde_json::{Value, json};
use tokio::process::Command;

use crate::common::{config, exe};

type Client = RunningService<RoleClient, ()>;

fn wrapper_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_agentctl-mcp"))
}

async fn connect() -> Client {
    let mut cmd = Command::new(wrapper_bin());
    cmd.env("AGENTCTL_BIN", exe())
        .env("AGENTCTL_CONFIG", config());
    let (transport, _) = TokioChildProcess::builder(cmd)
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    ().serve(transport).await.unwrap()
}

/// ツールを呼び、`(isError か, テキスト)` を返す。
async fn call(client: &Client, tool: &str, args: Value) -> (bool, String) {
    let params = CallToolRequestParams::new(tool.to_owned())
        .with_arguments(args.as_object().cloned().unwrap());
    let res = client.call_tool(params).await.unwrap();
    let text = res.content[0].as_text().unwrap().text.clone();
    (res.is_error == Some(true), text)
}

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("not JSON ({e}): {text:?}"))
}

#[tokio::test]
async fn wrapper_exposes_the_tools_but_not_watch() {
    let client = connect().await;
    let names: Vec<String> = client
        .list_all_tools()
        .await
        .unwrap()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    assert_eq!(
        names,
        [
            "list",
            "mcp_tools",
            "mcp_schema",
            "mcp_call",
            "a2a_card",
            "a2a_send",
            "a2a_task_get",
            "a2a_task_cancel",
        ]
    );
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_list_returns_the_configured_servers_and_agents() {
    let client = connect().await;
    let (is_error, text) = call(&client, "list", json!({})).await;
    assert!(!is_error);
    assert_eq!(
        text,
        "mcp\ttest\ttest MCP server\na2a\ttest\ttest A2A agent"
    );
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_mcp_tools_lists_the_tools_of_a_server() {
    let client = connect().await;
    let (is_error, text) = call(&client, "mcp_tools", json!({"server": "test"})).await;
    assert!(!is_error, "{text}");
    assert_eq!(
        text,
        "echo\tEcho the message\nstructured\tReturn structured content\nfail\tAlways return an error result"
    );
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_mcp_schema_shows_the_input_schema() {
    let client = connect().await;
    let (is_error, text) = call(
        &client,
        "mcp_schema",
        json!({"server": "test", "tool": "echo"}),
    )
    .await;
    assert!(!is_error, "{text}");
    let v = json_of(&text);
    assert_eq!(v["name"], "echo");
    assert_eq!(v["inputSchema"]["required"], json!(["message"]));
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_mcp_call_returns_the_tool_text() {
    let client = connect().await;
    let args = json!({"server": "test", "tool": "echo", "args": {"message": "hi"}});
    let (is_error, text) = call(&client, "mcp_call", args).await;
    assert!(!is_error, "{text}");
    assert_eq!(text, "echo: hi");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_mcp_call_prefers_structured_content() {
    let client = connect().await;
    let args = json!({"server": "test", "tool": "structured", "args": {"x": 1}});
    let (is_error, text) = call(&client, "mcp_call", args).await;
    assert!(!is_error, "{text}");
    assert_eq!(text, r#"{"args":{"x":1}}"#);
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_mcp_call_raw_returns_the_protocol_response() {
    let client = connect().await;
    let args = json!({"server": "test", "tool": "echo", "args": {"message": "hi"}, "raw": true});
    let (is_error, text) = call(&client, "mcp_call", args).await;
    assert!(!is_error, "{text}");
    assert_eq!(json_of(&text)["content"][0]["text"], "echo: hi");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_mcp_tool_failure_is_an_error_result_with_its_text() {
    let client = connect().await;
    let (is_error, text) = call(
        &client,
        "mcp_call",
        json!({"server": "test", "tool": "fail"}),
    )
    .await;
    assert!(is_error);
    assert_eq!(text, "boom");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_unknown_server_is_an_error_result() {
    let client = connect().await;
    let (is_error, text) = call(&client, "mcp_tools", json!({"server": "nope"})).await;
    assert!(is_error);
    assert!(text.starts_with("error: "), "{text}");
    assert!(text.contains("unknown MCP server 'nope'"), "{text}");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_a2a_card_shows_the_agent() {
    let client = connect().await;
    let (is_error, text) = call(&client, "a2a_card", json!({"agent": "test"})).await;
    assert!(!is_error, "{text}");
    assert_eq!(json_of(&text)["name"], "Test Agent");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_a2a_send_returns_the_reply_message() {
    let client = connect().await;
    let args = json!({"agent": "test", "text": "hello", "context_id": "ctx-1"});
    let (is_error, text) = call(&client, "a2a_send", args).await;
    assert!(!is_error, "{text}");
    let v = json_of(&text);
    assert_eq!(v["type"], "message");
    assert_eq!(v["text"], "echo: hello");
    assert_eq!(v["contextId"], "ctx-1");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_a2a_send_accepts_text_that_starts_with_a_hyphen() {
    let client = connect().await;
    let args = json!({"agent": "test", "text": "-1 is the answer"});
    let (is_error, text) = call(&client, "a2a_send", args).await;
    assert!(!is_error, "{text}");
    assert_eq!(json_of(&text)["text"], "echo: -1 is the answer");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_a2a_failed_task_is_an_error_result() {
    let client = connect().await;
    let (is_error, text) = call(
        &client,
        "a2a_send",
        json!({"agent": "test", "text": "fail it"}),
    )
    .await;
    assert!(is_error);
    assert_eq!(json_of(&text)["state"], "failed");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_a2a_task_can_be_followed_up_and_cancelled() {
    let client = connect().await;
    let args = json!({"agent": "test", "text": "slow it", "no_wait": true});
    let (_, text) = call(&client, "a2a_send", args).await;
    let id = json_of(&text)["id"].as_str().unwrap().to_owned();

    let (is_error, text) = call(&client, "a2a_task_get", json!({"agent": "test", "id": id})).await;
    assert!(!is_error, "{text}");
    assert_eq!(json_of(&text)["id"], id.as_str());

    let (is_error, text) = call(
        &client,
        "a2a_task_cancel",
        json!({"agent": "test", "id": id}),
    )
    .await;
    assert!(!is_error, "{text}");
    assert_eq!(json_of(&text)["state"], "canceled");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_a2a_unknown_task_is_an_error_result() {
    let client = connect().await;
    let (is_error, text) = call(
        &client,
        "a2a_task_get",
        json!({"agent": "test", "id": "nope"}),
    )
    .await;
    assert!(is_error);
    assert!(text.contains("task not found"), "{text}");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_rejects_invalid_arguments_as_a_protocol_error() {
    let client = connect().await;
    let missing = CallToolRequestParams::new("mcp_tools".to_owned());
    assert!(client.call_tool(missing).await.is_err());

    let typo = CallToolRequestParams::new("mcp_tools".to_owned()).with_arguments(
        json!({"server": "test", "srever": "x"})
            .as_object()
            .cloned()
            .unwrap(),
    );
    assert!(client.call_tool(typo).await.is_err());
    let _ = client.cancel().await;
}

#[tokio::test]
async fn wrapper_rejects_unknown_tools_including_watch() {
    let client = connect().await;
    for name in ["a2a_task_watch", "nope"] {
        let params = CallToolRequestParams::new(name.to_owned());
        assert!(client.call_tool(params).await.is_err(), "{name}");
    }
    let _ = client.cancel().await;
}
