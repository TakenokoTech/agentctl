use std::io;

use anyhow::{Context, anyhow};

use super::*;

fn io_error(kind: io::ErrorKind) -> anyhow::Error {
    io::Error::from(kind).into()
}

#[test]
fn a_broken_pipe_is_recognized() {
    assert!(is_broken_pipe(&io_error(io::ErrorKind::BrokenPipe)));
}

#[test]
fn a_broken_pipe_is_recognized_through_added_context() {
    let err = io_error(io::ErrorKind::BrokenPipe)
        .context("writing output")
        .context("running command");
    assert!(is_broken_pipe(&err));
}

#[test]
fn other_errors_are_not_a_broken_pipe() {
    assert!(!is_broken_pipe(&io_error(io::ErrorKind::PermissionDenied)));
    assert!(!is_broken_pipe(&anyhow!("unknown MCP server 'x'")));
    let err: anyhow::Result<()> = Err(anyhow!("boom"));
    assert!(!is_broken_pipe(&err.context("ctx").unwrap_err()));
}

// ---------- TSV の1列 ----------

#[test]
fn tabs_and_newlines_in_a_field_become_single_spaces() {
    assert_eq!(tsv_field("a\tb"), "a b");
    assert_eq!(tsv_field("line1\nline2\n"), "line1 line2");
    assert_eq!(tsv_field("x\r\ny"), "x y");
    assert_eq!(tsv_field("\n\t"), "");
}

#[test]
fn a_plain_field_is_unchanged() {
    assert_eq!(tsv_field("Echo the message"), "Echo the message");
    assert_eq!(tsv_field(""), "");
    assert_eq!(tsv_field("日本語 の 説明"), "日本語 の 説明");
}
