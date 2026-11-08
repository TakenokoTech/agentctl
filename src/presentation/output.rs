//! 標準出力への書き込み。
//!
//! `println!` は、パイプが閉じられた(`agentctl … | head` など)と panic する。
//! 書き込みの失敗を `Result` で返し、標準出力の切断は `main` で静かに終了させる。
use std::{fmt, fmt::Display, io, io::Write};

use anyhow::Result;

/// 標準出力の書き込み先が閉じられたことを表す目印。
/// HTTP の書き込みなど、ほかの `BrokenPipe` と区別するために使う。
#[derive(Debug)]
pub struct ClosedOutput;

impl fmt::Display for ClosedOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("standard output was closed")
    }
}

impl std::error::Error for ClosedOutput {}

/// 1行を標準出力に書く。書き込み先が閉じられていたら `ClosedOutput` を返す。
pub fn line(text: impl Display) -> Result<()> {
    writeln!(io::stdout().lock(), "{text}").map_err(stdout_error)
}

/// 標準出力の書き込みエラーを、切断なら `ClosedOutput` に、それ以外はそのまま `anyhow::Error` にする。
fn stdout_error(err: io::Error) -> anyhow::Error {
    if err.kind() == io::ErrorKind::BrokenPipe {
        ClosedOutput.into()
    } else {
        err.into()
    }
}

/// TSV の1列の値にする。タブと改行は列と行を壊すので、空白1つにまとめる。
pub fn tsv_field(text: &str) -> String {
    text.split(['\t', '\r', '\n'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 標準出力が閉じられたことによるエラーか。
pub fn is_closed_output(err: &anyhow::Error) -> bool {
    err.chain()
        .any(|cause| cause.downcast_ref::<ClosedOutput>().is_some())
}

#[cfg(test)]
mod tests;
