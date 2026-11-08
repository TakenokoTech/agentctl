//! 実バイナリを起動し、標準出力と終了コードを検証する E2E テスト。
//!
//! - MCP: `tests/fixtures/mcp_server.rs`(stdio)を子プロセスとして使う
//! - A2A: `tests/fixtures/a2a_agent.rs` をテスト内のランダムポートで起動する
//!
//! MCP サーバは `[[example]]` として宣言してあり、`cargo test` が自動でビルドする。
//! `--test e2e` だけを指定する場合は、先に `cargo build --examples` が必要。
mod a2a;
mod common;
mod mcp;
mod mcp_http;
mod mcp_wrapper;

use std::path::Path;

use common::{run, run_with, write_config};

#[test]
fn list_prints_tsv_of_kind_name_description() {
    let out = run(&["list"]);
    assert_eq!(out.code, 0);
    assert_eq!(
        out.lines(),
        vec!["mcp\ttest\ttest MCP server", "a2a\ttest\ttest A2A agent"]
    );
}

#[test]
fn missing_config_file_exits_1() {
    let out = run_with(Path::new("/nonexistent/agentctl.toml"), &["list"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("cannot read config"), "{}", out.stderr);
}

#[test]
fn invalid_config_exits_1() {
    let path = write_config("invalid", "[mcp.x]\nurl = \"http://x\"\n");
    let out = run_with(&path, &["list"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("invalid config"), "{}", out.stderr);
}

#[test]
fn unknown_subcommand_is_a_usage_error() {
    let out = run(&["nope"]);
    assert_eq!(out.code, 2);
    assert!(out.stdout.is_empty());
}

#[test]
fn a_closed_output_pipe_ends_quietly_instead_of_panicking() {
    // パイプの容量を超える量の出力にして、読み手が先に閉じるようにする
    let body: String = (0..4000)
        .map(|i| {
            let desc = "d".repeat(60);
            format!(
                "[mcp.s{i}]\ntransport = \"stdio\"\ncommand = [\"x\"]\ndescription = \"{desc}\"\n"
            )
        })
        .collect();
    let path = write_config("closed-pipe", &body);

    let mut child = common::command(&path, &["list"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take()); // 読み手が閉じる
    let out = child.wait_with_output().unwrap();

    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_misspelled_key_in_an_mcp_entry_is_reported_with_its_name() {
    let path = write_config(
        "mcp-typo",
        "[mcp.x]\ntransport = \"http\"\nurl = \"http://x\"\ntoken_en = \"T\"\n",
    );
    let out = run_with(&path, &["list"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("invalid config"), "{}", out.stderr);
    assert!(out.stderr.contains("token_en"), "{}", out.stderr);
}

#[test]
fn a_key_that_does_not_belong_to_the_transport_is_reported() {
    let path = write_config(
        "mcp-wrong-key",
        "[mcp.x]\ntransport = \"stdio\"\ncommand = [\"a\"]\ntoken_env = \"T\"\n",
    );
    let out = run_with(&path, &["list"]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .contains("`token_env` is not valid for transport = \"stdio\""),
        "{}",
        out.stderr
    );
}

#[test]
fn list_stays_three_columns_when_a_description_has_tabs_and_newlines() {
    let path = write_config(
        "tsv-description",
        "[mcp.x]\ndescription = \"\"\"first line\nsecond\tline\"\"\"\ntransport = \"stdio\"\ncommand = [\"a\"]\n",
    );
    let out = run_with(&path, &["list"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.lines(), vec!["mcp\tx\tfirst line second line"]);
}

// ---------- タイムアウト ----------

/// 制限時間(1秒)を超えても終わらないときに、テストを失敗にする締め切り。
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(20);

/// 接続は受け付けるが、何も返さないサーバー。応答しないサーバーの代わり。
fn silent_server() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        // 接続を閉じずに持ち続ける
        let mut held = Vec::new();
        for stream in listener.incoming().flatten() {
            held.push(stream);
        }
    });
    format!("http://{addr}")
}

fn assert_times_out(out: &common::Out, elapsed: std::time::Duration, what: &str) {
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr.contains("timed out after 1s") && out.stderr.contains("--timeout"),
        "{}",
        out.stderr
    );
    assert!(
        elapsed < std::time::Duration::from_secs(15),
        "{what}: {elapsed:?}"
    );
}

#[test]
fn an_unresponsive_a2a_agent_times_out() {
    let path = write_config(
        "timeout-a2a",
        &format!("[a2a.t]\nurl = \"{}\"\n", silent_server()),
    );
    let start = std::time::Instant::now();
    let out = common::run_with_deadline(&path, &["--timeout", "1", "a2a", "t", "card"], DEADLINE);
    assert_times_out(&out, start.elapsed(), "a2a card");
}

#[test]
fn an_unresponsive_http_mcp_server_times_out() {
    let path = write_config(
        "timeout-mcp-http",
        &format!(
            "[mcp.t]\ntransport = \"http\"\nurl = \"{}/mcp\"\n",
            silent_server()
        ),
    );
    let start = std::time::Instant::now();
    let out = common::run_with_deadline(&path, &["--timeout", "1", "mcp", "t", "tools"], DEADLINE);
    assert_times_out(&out, start.elapsed(), "mcp tools (http)");
    assert!(
        out.stderr.contains("connecting to the MCP server"),
        "{}",
        out.stderr
    );
}

#[test]
fn a_stdio_mcp_server_that_never_answers_times_out() {
    let path = write_config(
        "timeout-mcp-stdio",
        "[mcp.t]\ntransport = \"stdio\"\ncommand = [\"sleep\", \"30\"]\n",
    );
    let start = std::time::Instant::now();
    let out = common::run_with_deadline(&path, &["--timeout", "1", "mcp", "t", "tools"], DEADLINE);
    assert_times_out(&out, start.elapsed(), "mcp tools (stdio)");
}

#[test]
fn the_timeout_can_be_set_with_an_environment_variable() {
    let path = write_config(
        "timeout-env",
        &format!("[a2a.t]\nurl = \"{}\"\n", silent_server()),
    );
    let start = std::time::Instant::now();
    let mut cmd = common::command(&path, &["a2a", "t", "card"]);
    cmd.env("AGENTCTL_TIMEOUT", "1")
        .stdin(std::process::Stdio::null());
    let out = common::run_by(cmd, DEADLINE);
    assert_times_out(&out, start.elapsed(), "AGENTCTL_TIMEOUT");
}

// ---------- stdio サーバーの起動失敗 ----------

#[test]
fn a_stdio_server_that_fails_to_start_shows_the_end_of_its_stderr() {
    let path = write_config(
        "stdio-fails",
        "[mcp.t]\ntransport = \"stdio\"\ncommand = [\"sh\", \"-c\", \"echo 'boom: missing dependency' >&2; exit 3\"]\n",
    );
    let out = run_with(&path, &["mcp", "t", "tools"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("server stderr"), "{}", out.stderr);
    assert!(
        out.stderr.contains("boom: missing dependency"),
        "{}",
        out.stderr
    );
}

#[test]
fn a_working_stdio_server_does_not_leak_its_stderr() {
    // 正常に動くときは、サーバーの標準エラーを出力に混ぜない
    let out = run(&["mcp", "test", "tools"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stderr.is_empty(), "{}", out.stderr);
}
