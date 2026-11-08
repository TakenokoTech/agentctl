use super::*;
use crate::domain::config::{Entry, Kind, McpTransport};

const SAMPLE: &str = r#"
[mcp.everything]
description = "MCP reference server"
transport = "stdio"
command = ["npx", "-y", "@modelcontextprotocol/server-everything"]

[mcp.remote]
transport = "http"
url = "https://mcp.example.com/mcp"
token_env = "REMOTE_MCP_TOKEN"

[a2a.test]
description = "Echo agent"
url = "http://127.0.0.1:3000"
"#;

#[test]
fn parses_both_transports_and_agents() {
    let cfg = parse(SAMPLE).unwrap();
    assert!(matches!(
        cfg.mcp("everything").unwrap().transport,
        McpTransport::Stdio { .. }
    ));
    assert!(matches!(
        cfg.mcp("remote").unwrap().transport,
        McpTransport::Http { .. }
    ));
    assert_eq!(cfg.a2a("test").unwrap().url, "http://127.0.0.1:3000");
}

#[test]
fn entries_list_mcp_then_a2a_with_empty_description_default() {
    let cfg = parse(SAMPLE).unwrap();
    let entries = cfg.entries();
    assert_eq!(entries.len(), 3);
    assert_eq!(
        entries[1],
        Entry {
            kind: Kind::Mcp,
            name: "remote",
            description: ""
        }
    );
    assert_eq!(entries[2].kind, Kind::A2a);
}

#[test]
fn no_token_env_means_no_token() {
    assert_eq!(read_token_with(&None, |_| unreachable!()).unwrap(), None);
}

#[test]
fn token_is_read_from_the_named_variable() {
    let lookup = |k: &str| (k == "MY_TOKEN").then(|| "s3cret".to_owned());
    let token = read_token_with(&Some("MY_TOKEN".into()), lookup).unwrap();
    assert_eq!(token.as_deref(), Some("s3cret"));
}

#[test]
fn unset_variable_is_an_error_naming_it() {
    let err = read_token_with(&Some("MY_TOKEN".into()), |_| None).unwrap_err();
    assert_eq!(err.to_string(), "env var MY_TOKEN is not set");
}

#[test]
fn unknown_names_are_reported() {
    let cfg = parse(SAMPLE).unwrap();
    assert!(cfg.mcp("nope").is_err());
    assert!(cfg.a2a("nope").is_err());
}

#[test]
fn missing_or_unknown_transport_is_rejected() {
    assert!(parse("[mcp.x]\nurl = \"http://x\"\n").is_err());
    assert!(parse("[mcp.x]\ntransport = \"ws\"\n").is_err());
}

#[test]
fn unknown_top_level_keys_are_rejected() {
    assert!(parse("[mpc.x]\ntransport = \"stdio\"\n").is_err());
}

#[test]
fn unknown_mcp_server_keys_are_rejected() {
    let http = "[mcp.x]\ntransport = \"http\"\nurl = \"https://x\"\ntoken_en = \"T\"\n";
    assert!(parse(http).is_err(), "misspelled token_env");
    let stdio = "[mcp.x]\ntransport = \"stdio\"\ncommand = [\"x\"]\nenvv = {}\n";
    assert!(parse(stdio).is_err(), "misspelled env");
    let mixed = "[mcp.x]\ntransport = \"stdio\"\ncommand = [\"x\"]\nurl = \"https://x\"\n";
    assert!(parse(mixed).is_err(), "url belongs to http");
}

#[test]
fn http_server_may_omit_token_env() {
    let cfg = parse("[mcp.x]\ntransport = \"http\"\nurl = \"https://x\"\n").unwrap();
    assert!(matches!(
        cfg.mcp("x").unwrap().transport,
        McpTransport::Http {
            token_env: None,
            ..
        }
    ));
}

#[test]
fn token_needs_https_or_loopback_unless_insecure_is_allowed() {
    let check = |url: &str, token: bool, allow: bool| ensure_secure_transport(url, token, allow);
    for url in [
        "https://mcp.example.com/mcp",
        "http://127.0.0.1:3000",
        "http://localhost/x",
        "http://[::1]:8080/",
    ] {
        assert!(check(url, true, false).is_ok(), "{url}");
    }
    for url in [
        "http://mcp.example.com/mcp",
        "http://192.168.0.5",
        "http://localhost.example.com",
    ] {
        let err = check(url, true, false).unwrap_err().to_string();
        assert!(err.contains("allow_insecure"), "{url}: {err}");
        assert!(check(url, false, false).is_ok(), "{url} without a token");
        assert!(check(url, true, true).is_ok(), "{url} when allowed");
    }
    assert!(check("not a url", true, false).is_err());
}

#[test]
fn allow_insecure_defaults_to_false_and_is_http_only() {
    let cfg = parse(
        "[mcp.x]\ntransport = \"http\"\nurl = \"http://x\"\nallow_insecure = true\n\
         [mcp.y]\ntransport = \"http\"\nurl = \"http://y\"\n\
         [a2a.z]\nurl = \"http://z\"\nallow_insecure = true\n",
    )
    .unwrap();
    assert!(matches!(
        cfg.mcp("x").unwrap().transport,
        McpTransport::Http {
            allow_insecure: true,
            ..
        }
    ));
    assert!(matches!(
        cfg.mcp("y").unwrap().transport,
        McpTransport::Http {
            allow_insecure: false,
            ..
        }
    ));
    assert!(cfg.a2a("z").unwrap().allow_insecure);
    let stdio = "[mcp.s]\ntransport = \"stdio\"\ncommand = [\"x\"]\nallow_insecure = true\n";
    assert!(parse(stdio).is_err());
}
