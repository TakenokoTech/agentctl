//! 標準出力への書き込み。
//!
//! `println!` は、パイプが閉じられた(`agentctl … | head` など)と panic する。
//! 書き込みの失敗を `Result` で返し、パイプの切断は `main` で静かに終了させる。
use std::{fmt::Display, io::Write};

use anyhow::Result;

/// 1行を標準出力に書く。
pub fn line(text: impl Display) -> Result<()> {
    writeln!(std::io::stdout().lock(), "{text}")?;
    Ok(())
}

/// TSV の1列の値にする。タブと改行は列と行を壊すので、空白1つにまとめる。
pub fn tsv_field(text: &str) -> String {
    text.split(['\t', '\r', '\n'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 書き込み先が閉じられたことによるエラーか。
pub fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|e| e.kind() == std::io::ErrorKind::BrokenPipe)
    })
}

#[cfg(test)]
mod tests;
