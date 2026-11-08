//! A2A サブコマンドの E2E テスト。
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Stdio,
    sync::mpsc,
    time::Duration,
};

use crate::common::{Out, a2a_url, command, finish, run, run_with, write_config};

fn task_id(out: &Out) -> String {
    out.json()["id"].as_str().unwrap().to_owned()
}

#[test]
fn a2a_card_prints_name_skills_and_streaming() {
    let out = run(&["a2a", "test", "card"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = out.json();
    assert_eq!(v["name"], "Test Agent");
    assert_eq!(v["streaming"], true);
    assert_eq!(v["skills"][0]["id"], "echo");
}

#[test]
fn a2a_send_returns_the_reply_message() {
    let out = run(&["a2a", "test", "send", "hello"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = out.json();
    assert_eq!(v["type"], "message");
    assert_eq!(v["text"], "echo: hello");
}

#[test]
fn a2a_send_keeps_the_given_context_id() {
    let out = run(&["a2a", "test", "send", "hello", "--context-id", "ctx-1"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.json()["contextId"], "ctx-1");
}

#[test]
fn a2a_send_no_wait_returns_a_working_task() {
    let out = run(&["a2a", "test", "send", "slow report", "--no-wait"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = out.json();
    assert_eq!(v["type"], "task");
    assert_eq!(v["state"], "working");
}

#[test]
fn a2a_task_get_returns_the_task() {
    let id = task_id(&run(&["a2a", "test", "send", "slow a", "--no-wait"]));
    let out = run(&["a2a", "test", "task", "get", &id]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = out.json();
    assert_eq!(v["type"], "task");
    assert_eq!(v["id"], id.as_str());
}

#[test]
fn a2a_task_watch_streams_until_completed() {
    let id = task_id(&run(&["a2a", "test", "send", "slow b", "--no-wait"]));
    let out = run(&["a2a", "test", "task", "watch", &id]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let events = out.json_lines();
    assert_eq!(events.first().unwrap()["type"], "task");
    assert_eq!(events.last().unwrap()["state"], "completed");
}

#[test]
fn a2a_task_cancel_marks_the_task_canceled() {
    let id = task_id(&run(&["a2a", "test", "send", "slow c", "--no-wait"]));
    let out = run(&["a2a", "test", "task", "cancel", &id]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.json()["state"], "canceled");
}

#[test]
fn a2a_failed_task_exits_2() {
    let out = run(&["a2a", "test", "send", "fail it"]);
    assert_eq!(out.code, 2);
    assert_eq!(out.json()["state"], "failed");
}

#[test]
fn a2a_watching_a_failed_task_exits_2() {
    let id = task_id(&run(&["a2a", "test", "send", "fail it"]));
    let out = run(&["a2a", "test", "task", "watch", &id]);
    assert_eq!(out.code, 2);
    assert_eq!(out.json_lines().last().unwrap()["state"], "failed");
}

#[test]
fn a2a_raw_send_prints_the_protocol_message() {
    let out = run(&["--raw", "a2a", "test", "send", "hello"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v = out.json();
    assert!(v.get("messageId").is_some(), "{v}");
    assert_eq!(v["parts"][0]["text"], "echo: hello");
}

#[test]
fn a2a_unknown_task_exits_1() {
    let out = run(&["a2a", "test", "task", "get", "nope"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("task not found"), "{}", out.stderr);
}

#[test]
fn a2a_unknown_agent_exits_1() {
    let out = run(&["a2a", "nope", "card"]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains("unknown A2A agent 'nope'"),
        "{}",
        out.stderr
    );
}

#[test]
fn a2a_unreachable_agent_exits_1() {
    let path = write_config("unreachable", "[a2a.gone]\nurl = \"http://127.0.0.1:1\"\n");
    let out = run_with(&path, &["a2a", "gone", "card"]);
    assert_eq!(out.code, 1);
    assert!(out.stdout.is_empty());
}

#[test]
fn a2a_missing_token_env_exits_1() {
    let path = write_config(
        "missing-token",
        &format!(
            "[a2a.t]\nurl = \"{}\"\ntoken_env = \"AGENTCTL_E2E_TOKEN_THAT_IS_NOT_SET\"\n",
            a2a_url()
        ),
    );
    let out = run_with(&path, &["a2a", "t", "card"]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .contains("env var AGENTCTL_E2E_TOKEN_THAT_IS_NOT_SET is not set"),
        "{}",
        out.stderr
    );
}

#[test]
fn a2a_sends_the_token_from_env_as_a_bearer_header() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buf = [0u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
        }
        tx.send(String::from_utf8_lossy(&request).to_lowercase())
            .unwrap();
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
            .unwrap();
    });

    let path = write_config(
        "bearer",
        &format!("[a2a.t]\nurl = \"http://{addr}\"\ntoken_env = \"AGENTCTL_E2E_TOKEN\"\n"),
    );
    let out = finish(
        command(&path, &["a2a", "t", "card"])
            .env("AGENTCTL_E2E_TOKEN", "s3cret")
            .stdin(Stdio::null())
            .output()
            .unwrap(),
    );

    assert_eq!(out.code, 1); // 404 なのでカード取得は失敗する
    let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(
        request.contains("authorization: bearer s3cret"),
        "{request}"
    );
}

#[test]
fn a2a_refuses_to_send_a_token_over_plain_http_to_a_remote_host() {
    let path = write_config(
        "insecure-token",
        "[a2a.t]\nurl = \"http://example.invalid:1\"\ntoken_env = \"AGENTCTL_E2E_TOKEN\"\n",
    );
    let out = finish(
        command(&path, &["a2a", "t", "card"])
            .env("AGENTCTL_E2E_TOKEN", "s3cret")
            .stdin(Stdio::null())
            .output()
            .unwrap(),
    );
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("allow_insecure"), "{}", out.stderr);
}

#[test]
fn a2a_does_not_send_the_token_to_an_endpoint_on_another_origin() {
    let card = crate::common::card_advertising("http://other.invalid:1");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
            }
            let line = String::from_utf8_lossy(&request);
            tx.send(line.lines().next().unwrap_or("").to_owned()).ok();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{card}",
                card.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });

    let path = write_config(
        "cross-origin",
        &format!("[a2a.t]\nurl = \"http://{addr}\"\ntoken_env = \"AGENTCTL_E2E_TOKEN\"\n"),
    );
    let out = finish(
        command(&path, &["a2a", "t", "send", "hello"])
            .env("AGENTCTL_E2E_TOKEN", "s3cret")
            .stdin(Stdio::null())
            .output()
            .unwrap(),
    );

    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stderr.contains("different origin"), "{}", out.stderr);
    let requests: Vec<String> = rx.try_iter().collect();
    assert!(
        requests.iter().any(|r| r.starts_with("GET ")),
        "{requests:?}"
    );
    assert!(
        requests.iter().all(|r| r.starts_with("GET ")),
        "{requests:?}"
    );
}

/// 受けたリクエストの1行目を `seen` に送り、`handler(1行目)` が返す応答を返すサーバ。
fn serve(handler: impl Fn(&str) -> String + Send + 'static) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
            }
            let text = String::from_utf8_lossy(&request).to_lowercase();
            tx.send(text.clone()).ok();
            let line = text.lines().next().unwrap_or("").to_owned();
            stream.write_all(handler(&line).as_bytes()).unwrap();
        }
    });
    (format!("http://{addr}"), rx)
}

fn redirect_to(location: &str) -> String {
    format!(
        "HTTP/1.1 302 Found\r\nlocation: {location}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
    )
}

fn json_response(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn card_with_token(url: &str, token: bool) -> Out {
    let token_line = if token {
        "token_env = \"AGENTCTL_E2E_TOKEN\"\n"
    } else {
        ""
    };
    let path = write_config(
        &format!("redirect-{}", url.replace([':', '/'], "_")),
        &format!("[a2a.t]\nurl = \"{url}\"\n{token_line}"),
    );
    finish(
        command(&path, &["a2a", "t", "card"])
            .env("AGENTCTL_E2E_TOKEN", "s3cret")
            .stdin(Stdio::null())
            .output()
            .unwrap(),
    )
}

#[test]
fn a2a_with_a_token_does_not_follow_a_redirect_to_another_origin() {
    let (other, other_seen) = serve(|_| json_response(&crate::common::card_advertising("")));
    let target = format!("{other}/.well-known/agent-card.json");
    let (url, _) = serve(move |_| redirect_to(&target));
    let out = card_with_token(&url, true);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stderr.contains("origin"), "{}", out.stderr);
    assert!(
        other_seen.try_recv().is_err(),
        "the other origin was contacted"
    );
}

#[test]
fn a2a_with_a_token_follows_a_redirect_on_the_same_origin() {
    let (url, seen) = serve(|line| {
        if line.contains("/moved") {
            json_response(&crate::common::card_advertising(""))
        } else {
            redirect_to("/moved")
        }
    });
    let out = card_with_token(&url, true);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let requests: Vec<String> = seen.try_iter().collect();
    assert!(
        requests
            .iter()
            .all(|r| r.contains("authorization: bearer s3cret")),
        "{requests:?}"
    );
}

#[test]
fn a2a_without_a_token_still_follows_a_redirect_to_another_origin() {
    let (other, _) = serve(|_| json_response(&crate::common::card_advertising("")));
    let target = format!("{other}/.well-known/agent-card.json");
    let (url, _) = serve(move |_| redirect_to(&target));
    let out = card_with_token(&url, false);
    assert_eq!(out.code, 0, "{}", out.stderr);
}
