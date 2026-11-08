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
