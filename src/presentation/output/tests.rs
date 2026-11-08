use std::io;

use anyhow::anyhow;

use super::*;

fn io_error(kind: io::ErrorKind) -> io::Error {
    io::Error::from(kind)
}

#[test]
fn a_broken_pipe_on_stdout_becomes_the_closed_output_marker() {
    assert!(is_closed_output(&stdout_error(io_error(
        io::ErrorKind::BrokenPipe
    ))));
}

#[test]
fn the_marker_is_recognized_through_added_context() {
    let err = stdout_error(io_error(io::ErrorKind::BrokenPipe))
        .context("writing output")
        .context("running command");
    assert!(is_closed_output(&err));
}

#[test]
fn other_stdout_errors_are_not_the_closed_output_marker() {
    assert!(!is_closed_output(&stdout_error(io_error(
        io::ErrorKind::PermissionDenied
    ))));
    assert!(!is_closed_output(&anyhow!("unknown MCP server 'x'")));
}

#[test]
fn a_broken_pipe_from_elsewhere_is_not_the_closed_output_marker() {
    // HTTP の書き込み失敗など、標準出力以外の BrokenPipe を成功として扱わない
    let err = anyhow::Error::from(io_error(io::ErrorKind::BrokenPipe)).context("sending request");
    assert!(!is_closed_output(&err));
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
