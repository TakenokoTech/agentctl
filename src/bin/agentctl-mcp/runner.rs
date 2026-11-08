//! `agentctl` をサブプロセスとして実行し、結果を MCP のツール結果に対応する形にする。
use std::{ffi::OsString, process::Stdio};

use tokio::{io::AsyncWriteExt, process::Command};

use crate::command::Invocation;

/// `agentctl` の実行ファイルを指す環境変数。未設定なら `PATH` 上の `agentctl` を使う。
pub const BIN_ENV: &str = "AGENTCTL_BIN";

/// ツールの実行結果。`is_error` は `agentctl` が 0 以外で終了したことを表す。
#[derive(Debug, PartialEq)]
pub struct Outcome {
    pub text: String,
    pub is_error: bool,
}

pub struct Runner {
    bin: OsString,
}

impl Runner {
    pub fn new(bin: impl Into<OsString>) -> Self {
        Self { bin: bin.into() }
    }

    pub fn from_env() -> Self {
        Self::new(std::env::var_os(BIN_ENV).unwrap_or_else(|| "agentctl".into()))
    }

    pub async fn run(&self, inv: &Invocation) -> Outcome {
        match self.spawn(inv).await {
            Ok(output) => interpret(
                output.status.code(),
                &String::from_utf8_lossy(&output.stdout),
                &String::from_utf8_lossy(&output.stderr),
            ),
            Err(e) => Outcome {
                text: format!(
                    "error: failed to run `{}`: {e} (set {BIN_ENV} to the path of agentctl)",
                    self.bin.to_string_lossy()
                ),
                is_error: true,
            },
        }
    }

    async fn spawn(&self, inv: &Invocation) -> std::io::Result<std::process::Output> {
        let mut child = Command::new(&self.bin)
            .args(&inv.args)
            .stdin(if inv.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        if let (Some(input), Some(mut stdin)) = (&inv.stdin, child.stdin.take()) {
            // 子が入力を読まずに終了した場合の書き込みエラーは、終了コードで扱う
            let _ = stdin.write_all(input.as_bytes()).await;
        }
        child.wait_with_output().await
    }
}

/// 終了コードと出力から、結果を決める。
///
/// 0 は成功。それ以外は `isError` にし、本文は標準出力(終了コード 2 は tool や task の失敗内容を
/// 標準出力に出す)、なければ標準エラー(終了コード 1 の `error: …`)を使う。
pub fn interpret(code: Option<i32>, stdout: &str, stderr: &str) -> Outcome {
    let stdout = stdout.strip_suffix('\n').unwrap_or(stdout);
    let stderr = stderr.trim_end();
    if code == Some(0) {
        return Outcome {
            text: stdout.to_owned(),
            is_error: false,
        };
    }
    let text = if !stdout.trim().is_empty() {
        stdout.to_owned()
    } else if !stderr.is_empty() {
        stderr.to_owned()
    } else {
        match code {
            Some(c) => format!("error: agentctl exited with status {c}"),
            None => "error: agentctl was terminated by a signal".to_owned(),
        }
    };
    Outcome {
        text,
        is_error: true,
    }
}

#[cfg(test)]
mod tests;
