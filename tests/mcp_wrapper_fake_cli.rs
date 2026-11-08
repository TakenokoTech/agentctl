//! ラッパーを実際に起動し、MCP クライアントから呼ぶ結合テスト。
//!
//! `agentctl` の代わりに、受け取った引数と標準入力を出力する偽のスクリプトを使うので、
//! `agentctl` の実装には依存しない(本物の CLI との結合は、ルートの E2E テストで確かめる)。
#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Stdio, sync::OnceLock};

use rmcp::{
    RoleClient, ServiceExt, model::CallToolRequestParams, service::RunningService,
    transport::TokioChildProcess,
};
use serde_json::{Value, json};
use tokio::process::Command;

type Client = RunningService<RoleClient, ()>;

/// 引数を `ARGS:[a][b]` の形で出力し、環境変数に応じて標準入力・標準エラー・終了コードを返す。
/// `FAKE_QUIET` を設定すると、標準出力に何も出さない。
const FAKE_CLI: &str = r#"#!/bin/sh
if [ -z "$FAKE_QUIET" ]; then
  printf 'ARGS:'
  for a in "$@"; do printf '[%s]' "$a"; done
  printf '\n'
fi
if [ -n "$FAKE_READ_STDIN" ]; then printf 'STDIN:%s\n' "$(cat)"; fi
if [ -n "$FAKE_SHOW_CONFIG" ]; then printf 'CONFIG:%s\n' "$AGENTCTL_CONFIG"; fi
if [ -n "$FAKE_STDERR" ]; then printf '%s\n' "$FAKE_STDERR" >&2; fi
exit "${FAKE_EXIT:-0}"
"#;

fn fake_cli() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fake-agentctl.sh");
        fs::write(&path, FAKE_CLI).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    })
}

async fn connect_with_bin(bin: &str, env: &[(&str, &str)]) -> Client {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_agentctl-mcp"));
    cmd.env("AGENTCTL_BIN", bin).envs(env.iter().copied());
    let (transport, _) = TokioChildProcess::builder(cmd)
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    ().serve(transport).await.unwrap()
}

async fn connect(env: &[(&str, &str)]) -> Client {
    connect_with_bin(fake_cli().to_str().unwrap(), env).await
}

/// ツールを呼び、`(isError か, テキスト)` を返す。
async fn call(client: &Client, tool: &str, args: Value) -> (bool, String) {
    let params = CallToolRequestParams::new(tool.to_owned())
        .with_arguments(args.as_object().cloned().unwrap());
    let res = client.call_tool(params).await.unwrap();
    let text = res.content[0].as_text().unwrap().text.clone();
    (res.is_error == Some(true), text)
}

#[tokio::test]
async fn lists_the_tools() {
    let client = connect(&[]).await;
    let names: Vec<String> = client
        .list_all_tools()
        .await
        .unwrap()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    assert_eq!(names.len(), 8);
    assert!(names.contains(&"mcp_call".to_owned()));
    assert!(!names.contains(&"a2a_task_watch".to_owned()));
    let _ = client.cancel().await;
}

#[tokio::test]
async fn a_tool_call_runs_the_command_and_returns_its_stdout() {
    let client = connect(&[]).await;
    let (is_error, text) = call(&client, "list", json!({})).await;
    assert!(!is_error);
    assert_eq!(text, "ARGS:[list]");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn mcp_call_arguments_are_sent_through_stdin() {
    let client = connect(&[("FAKE_READ_STDIN", "1")]).await;
    let args = json!({"server": "s", "tool": "t", "args": {"message": "hi"}});
    let (is_error, text) = call(&client, "mcp_call", args).await;
    assert!(!is_error, "{text}");
    assert_eq!(
        text,
        "ARGS:[mcp][s][call][t][--args][-]\nSTDIN:{\"message\":\"hi\"}"
    );
    let _ = client.cancel().await;
}

#[tokio::test]
async fn text_that_looks_like_an_option_is_passed_after_a_double_dash() {
    let client = connect(&[]).await;
    let args = json!({"agent": "a", "text": "--config=/evil"});
    let (_, text) = call(&client, "a2a_send", args).await;
    assert_eq!(text, "ARGS:[a2a][a][send][--][--config=/evil]");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn the_environment_reaches_the_command() {
    let client = connect(&[
        ("FAKE_SHOW_CONFIG", "1"),
        ("AGENTCTL_CONFIG", "/etc/agentctl.toml"),
    ])
    .await;
    let (_, text) = call(&client, "list", json!({})).await;
    assert_eq!(text, "ARGS:[list]\nCONFIG:/etc/agentctl.toml");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn exit_code_2_is_an_error_result_with_the_stdout_text() {
    let client = connect(&[("FAKE_EXIT", "2")]).await;
    let (is_error, text) = call(&client, "list", json!({})).await;
    assert!(is_error);
    assert_eq!(text, "ARGS:[list]");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn exit_code_1_is_an_error_result_with_the_stderr_text() {
    let env = [
        ("FAKE_QUIET", "1"),
        ("FAKE_EXIT", "1"),
        ("FAKE_STDERR", "error: no such server"),
    ];
    let client = connect(&env).await;
    let (is_error, text) = call(&client, "list", json!({})).await;
    assert!(is_error);
    assert_eq!(text, "error: no such server");
    let _ = client.cancel().await;
}

#[tokio::test]
async fn a_missing_command_is_an_error_result() {
    let client = connect_with_bin("/nonexistent/agentctl", &[]).await;
    let (is_error, text) = call(&client, "list", json!({})).await;
    assert!(is_error);
    assert!(
        text.contains("failed to run `/nonexistent/agentctl`"),
        "{text}"
    );
    let _ = client.cancel().await;
}

#[tokio::test]
async fn invalid_arguments_are_protocol_errors_and_never_start_the_command() {
    let client = connect(&[]).await;
    let bad: [(&str, Value); 4] = [
        ("mcp_tools", json!({})),
        ("mcp_tools", json!({"server": "s", "srever": "x"})),
        ("mcp_tools", json!({"server": "--config=/evil"})),
        ("a2a_task_watch", json!({})),
    ];
    for (tool, args) in bad {
        let params = CallToolRequestParams::new(tool.to_owned())
            .with_arguments(args.as_object().cloned().unwrap());
        assert!(client.call_tool(params).await.is_err(), "{tool} {args}");
    }
    let _ = client.cancel().await;
}
