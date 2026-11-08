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
    assert_eq!(out.code, 64);
    assert!(out.stdout.is_empty());
}

#[test]
fn help_and_version_exit_0() {
    for flag in ["--help", "--version"] {
        let out = run(&[flag]);
        assert_eq!(out.code, 0, "{flag}");
        assert!(!out.stdout.is_empty(), "{flag}");
    }
}
