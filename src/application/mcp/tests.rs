use std::cell::Cell;
use std::rc::Rc;

use serde_json::{Value, json};

use super::*;
use crate::domain::config::McpTransport;

struct FakeConnector {
    closed: Rc<Cell<bool>>,
}

struct FakeSession {
    closed: Rc<Cell<bool>>,
}

impl McpConnector for FakeConnector {
    type Session = FakeSession;

    async fn connect(&self, _: &McpServer) -> Result<FakeSession> {
        Ok(FakeSession {
            closed: self.closed.clone(),
        })
    }
}

impl McpSession for FakeSession {
    async fn list_tools(&self) -> Result<Vec<ToolInfo>> {
        Ok(vec![ToolInfo {
            name: "echo".into(),
            description: Some("Echo".into()),
            input_schema: Value::Null,
        }])
    }

    async fn call_tool(&self, name: &str, args: JsonObject) -> Result<ToolOutput> {
        if name == "broken" {
            anyhow::bail!("transport closed");
        }
        Ok(ToolOutput {
            is_error: false,
            structured: Some(json!({"name": name, "args": args})),
            contents: Vec::new(),
            raw: Value::Null,
        })
    }

    async fn close(self) {
        self.closed.set(true);
    }
}

fn server() -> McpServer {
    McpServer {
        description: None,
        transport: McpTransport::Http {
            url: "http://localhost".into(),
            token_env: None,
            allow_insecure: false,
        },
    }
}

#[tokio::test]
async fn schema_of_known_tool_is_returned_and_session_closed() {
    let closed = Rc::new(Cell::new(false));
    let c = FakeConnector {
        closed: closed.clone(),
    };
    let t = tool_schema(&c, &server(), "echo").await.unwrap();
    assert_eq!(t.name, "echo");
    assert!(closed.get());
}

#[tokio::test]
async fn schema_of_unknown_tool_fails_but_session_is_still_closed() {
    let closed = Rc::new(Cell::new(false));
    let c = FakeConnector {
        closed: closed.clone(),
    };
    let err = tool_schema(&c, &server(), "nope").await.unwrap_err();
    assert!(err.to_string().contains("unknown tool 'nope'"));
    assert!(closed.get());
}

#[tokio::test]
async fn list_tools_returns_the_tools_and_closes_the_session() {
    let closed = Rc::new(Cell::new(false));
    let c = FakeConnector {
        closed: closed.clone(),
    };
    let tools = list_tools(&c, &server()).await.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
    assert!(closed.get());
}

#[tokio::test]
async fn failing_call_propagates_the_error_and_closes_the_session() {
    let closed = Rc::new(Cell::new(false));
    let c = FakeConnector {
        closed: closed.clone(),
    };
    let err = call_tool(&c, &server(), "broken", JsonObject::new())
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "transport closed");
    assert!(closed.get());
}

#[tokio::test]
async fn connect_failure_is_reported_without_a_session() {
    struct Refusing;
    impl McpConnector for Refusing {
        type Session = FakeSession;
        async fn connect(&self, _: &McpServer) -> Result<FakeSession> {
            anyhow::bail!("refused")
        }
    }
    let err = list_tools(&Refusing, &server()).await.unwrap_err();
    assert_eq!(err.to_string(), "refused");
}

#[tokio::test]
async fn call_passes_name_and_args_through() {
    let c = FakeConnector {
        closed: Rc::new(Cell::new(false)),
    };
    let mut args = JsonObject::new();
    args.insert("a".into(), json!(1));
    let out = call_tool(&c, &server(), "echo", args).await.unwrap();
    assert_eq!(
        out.structured,
        Some(json!({"name": "echo", "args": {"a": 1}}))
    );
}
