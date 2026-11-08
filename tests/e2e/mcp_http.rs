//! MCP の Streamable HTTP トランスポートの E2E テスト。
use crate::common::{
    Out,
    mcp_http_server::{HttpServer, start},
    run_with, write_config,
};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::common::{command, finish};

fn config(name: &str, url: &str, token_env: Option<&str>) -> std::path::PathBuf {
    let token = token_env
        .map(|t| format!("token_env = \"{t}\"\n"))
        .unwrap_or_default();
    write_config(
        name,
        &format!("[mcp.remote]\ntransport = \"http\"\nurl = \"{url}\"\n{token}"),
    )
}

/// テストは並列に走るので、設定ファイルはテストごとに別名にする。
fn server() -> (HttpServer, std::path::PathBuf) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let server = start();
    let name = format!("http-plain-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let path = config(&name, &server.url, None);
    (server, path)
}

fn with_token(path: &std::path::Path, token: &str, args: &[&str]) -> Out {
    finish(
        command(path, args)
            .env("AGENTCTL_E2E_HTTP_TOKEN", token)
            .stdin(Stdio::null())
            .output()
            .unwrap(),
    )
}

#[test]
fn http_tools_lists_the_tools() {
    let (_server, path) = server();
    let out = run_with(&path, &["mcp", "remote", "tools"]);
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
fn http_call_passes_args_and_prints_text() {
    let (_server, path) = server();
    let out = run_with(
        &path,
        &[
            "mcp",
            "remote",
            "call",
            "echo",
            "--args",
            r#"{"message":"hi"}"#,
        ],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.lines(), vec!["echo: hi"]);
}

#[test]
fn http_tool_error_result_exits_2() {
    let (_server, path) = server();
    let out = run_with(&path, &["mcp", "remote", "call", "fail"]);
    assert_eq!(out.code, 2);
    assert_eq!(out.lines(), vec!["boom"]);
}

#[test]
fn http_without_token_env_sends_no_authorization_header() {
    let (server, path) = server();
    let out = run_with(&path, &["mcp", "remote", "tools"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(server.seen_auth.lock().unwrap().is_empty());
}

#[test]
fn http_sends_the_token_from_env_as_a_bearer_header() {
    let server = start();
    let path = config("http-token", &server.url, Some("AGENTCTL_E2E_HTTP_TOKEN"));
    let out = with_token(&path, "s3cret", &["mcp", "remote", "tools"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let seen = server.seen_auth.lock().unwrap();
    assert!(!seen.is_empty());
    assert!(seen.iter().all(|v| v == "Bearer s3cret"), "{seen:?}");
}

#[test]
fn http_missing_token_env_exits_1() {
    let server = start();
    let path = config(
        "http-missing-token",
        &server.url,
        Some("AGENTCTL_E2E_HTTP_TOKEN_THAT_IS_NOT_SET"),
    );
    let out = run_with(&path, &["mcp", "remote", "tools"]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .contains("env var AGENTCTL_E2E_HTTP_TOKEN_THAT_IS_NOT_SET is not set"),
        "{}",
        out.stderr
    );
    assert!(server.seen_auth.lock().unwrap().is_empty());
}

#[test]
fn http_unreachable_server_exits_1() {
    let path = config("http-unreachable", "http://127.0.0.1:1/mcp", None);
    let out = run_with(&path, &["mcp", "remote", "tools"]);
    assert_eq!(out.code, 1);
    assert!(out.stdout.is_empty());
}
