//! stdio MCP サーバーの標準エラーを読み続け、末尾の数行だけを保持する。
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::ChildStderr,
    task::JoinHandle,
};

/// 保持する標準エラーの行数と、1行あたりの最大文字数。
const TAIL_LINES: usize = 20;
const MAX_LINE_CHARS: usize = 500;

/// 標準エラーの末尾の数行。
#[derive(Default)]
struct Tail(VecDeque<String>);

impl Tail {
    fn push(&mut self, line: &str) {
        if self.0.len() == TAIL_LINES {
            self.0.pop_front();
        }
        self.0
            .push_back(line.chars().take(MAX_LINE_CHARS).collect());
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn text(&self) -> String {
        self.0
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// stdio サーバーの標準エラーを読み続け、末尾だけを保持する。
/// 読み続けるのは、読まないとパイプが詰まってサーバーが止まるため。
pub(super) struct StderrTail {
    tail: Arc<Mutex<Tail>>,
    reader: JoinHandle<()>,
}

impl StderrTail {
    pub(super) fn start(stderr: ChildStderr) -> Self {
        let tail = Arc::new(Mutex::new(Tail::default()));
        let sink = tail.clone();
        let reader = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                sink.lock().unwrap().push(&line);
            }
        });
        Self { tail, reader }
    }

    /// 接続失敗の原因に、サーバーの標準エラーの末尾を添える。
    pub(super) async fn explain(self, err: anyhow::Error) -> anyhow::Error {
        // サーバーが終了していれば読み終わる。終了していなければ、そこまでの分を使う
        let _ = tokio::time::timeout(Duration::from_millis(500), self.reader).await;
        let tail = self.tail.lock().unwrap();
        if tail.is_empty() {
            return err;
        }
        anyhow::anyhow!(
            "{err:#}\n--- server stderr (last {} lines) ---\n{}",
            tail.0.len(),
            tail.text()
        )
    }
}

#[cfg(test)]
mod tests;
