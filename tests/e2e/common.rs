//! E2E テスト共通のハーネス。実バイナリの起動、設定ファイルの生成、テスト用サーバの用意をする。
#[path = "../fixtures/mcp_http_server.rs"]
pub mod mcp_http_server;
#[path = "../fixtures/a2a_agent.rs"]
mod test_agent;

use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
    time::{Duration, Instant},
};

use serde_json::Value;
use test_agent::TestAgent;

pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Out {
    pub fn lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(self.stdout.trim())
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {:?}", self.stdout))
    }

    pub fn json_lines(&self) -> Vec<Value> {
        self.lines()
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }
}

pub fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_agentctl"))
}

pub fn example(name: &str) -> PathBuf {
    let path = exe().parent().unwrap().join("examples").join(name);
    assert!(
        path.exists(),
        "{} がない。`cargo build --examples` を実行する",
        path.display()
    );
    path
}

pub fn write_config(name: &str, body: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("e2e-{name}.toml"));
    fs::write(&path, body).unwrap();
    path
}

/// テスト用 A2A エージェントを一度だけ起動し、その URL を返す。
pub fn a2a_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                    axum::serve(listener, a2a_rust::server::router(TestAgent::default()))
                        .await
                        .unwrap();
                })
        });
        format!("http://{addr}")
    })
}

/// MCP(`test`)と A2A(`test`)を登録した標準の設定。
pub fn config() -> &'static Path {
    static CONFIG: OnceLock<PathBuf> = OnceLock::new();
    CONFIG.get_or_init(|| {
        write_config(
            "standard",
            &format!(
                r#"
[mcp.test]
description = "test MCP server"
transport = "stdio"
command = [{mcp:?}]

[a2a.test]
description = "test A2A agent"
url = "{url}"
"#,
                mcp = example("test_mcp_server").display().to_string(),
                url = a2a_url(),
            ),
        )
    })
}

pub fn command(config: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(exe());
    cmd.env("AGENTCTL_CONFIG", config).args(args);
    cmd
}

pub fn finish(output: std::process::Output) -> Out {
    Out {
        code: output.status.code().expect("terminated by signal"),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

pub fn run_with(config: &Path, args: &[&str]) -> Out {
    finish(command(config, args).stdin(Stdio::null()).output().unwrap())
}

pub fn run(args: &[&str]) -> Out {
    run_with(config(), args)
}

pub fn run_stdin(args: &[&str], input: &str) -> Out {
    let mut child = command(config(), args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    finish(child.wait_with_output().unwrap())
}

/// stdin を開いたまま閉じずに実行し、`timeout` 内に終了するかを確かめる。
pub fn run_with_open_stdin(args: &[&str], timeout: Duration) -> Out {
    let mut child = command(config(), args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _stdin = child.stdin.take(); // drop しない
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > timeout {
            child.kill().unwrap();
            panic!("{args:?} は stdin を待って止まった");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    Out {
        code: status.code().unwrap(),
        stdout,
        stderr,
    }
}
