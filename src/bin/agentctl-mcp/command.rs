//! ツール名と引数を、`agentctl` の引数列に変換する。
//!
//! 引数は LLM などの呼び出し側が与える。`agentctl` のオプションとして解釈されないよう、
//! 名前や ID は `-` で始まるものを拒否し、自由なテキストは `--` の後ろに置く。
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Map, Value};

pub type JsonObject = Map<String, Value>;

/// `agentctl` の起動方法。
#[derive(Debug, PartialEq)]
pub struct Invocation {
    pub args: Vec<String>,
    /// 標準入力に渡す内容(`mcp call` の `--args -` 用)。
    pub stdin: Option<String>,
}

impl Invocation {
    fn new(args: &[&str]) -> Self {
        Self {
            args: args.iter().map(|s| (*s).to_owned()).collect(),
            stdin: None,
        }
    }

    fn push(&mut self, arg: impl Into<String>) {
        self.args.push(arg.into());
    }

    /// 先頭に `--raw` を付ける(グローバルオプション)。
    fn raw(mut self, raw: bool) -> Self {
        if raw {
            self.args.insert(0, "--raw".to_owned());
        }
        self
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArgs {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpToolsArgs {
    server: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpSchemaArgs {
    server: String,
    tool: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpCallArgs {
    server: String,
    tool: String,
    #[serde(default)]
    args: JsonObject,
    #[serde(default)]
    raw: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct A2aCardArgs {
    agent: String,
    #[serde(default)]
    raw: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct A2aSendArgs {
    agent: String,
    text: String,
    context_id: Option<String>,
    task_id: Option<String>,
    #[serde(default)]
    no_wait: bool,
    #[serde(default)]
    raw: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct A2aTaskArgs {
    agent: String,
    id: String,
    #[serde(default)]
    raw: bool,
}

fn from_args<T: DeserializeOwned>(args: JsonObject) -> Result<T, String> {
    serde_json::from_value(Value::Object(args)).map_err(|e| e.to_string())
}

/// 名前や ID が、オプションとして解釈される形でないことを確かめる。
fn name<'a>(field: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() || value.starts_with('-') {
        return Err(format!("`{field}` must not be empty or start with '-'"));
    }
    Ok(value)
}

/// ツール名と引数から、`agentctl` の起動方法を作る。失敗は呼び出し側の誤りとして扱う。
pub fn to_invocation(tool: &str, args: JsonObject) -> Result<Invocation, String> {
    match tool {
        "list" => from_args::<NoArgs>(args).map(|_| Invocation::new(&["list"])),
        "mcp_tools" => {
            let a: McpToolsArgs = from_args(args)?;
            Ok(Invocation::new(&[
                "mcp",
                name("server", &a.server)?,
                "tools",
            ]))
        }
        "mcp_schema" => {
            let a: McpSchemaArgs = from_args(args)?;
            let (server, tool) = (name("server", &a.server)?, name("tool", &a.tool)?);
            Ok(Invocation::new(&["mcp", server, "schema", tool]))
        }
        "mcp_call" => mcp_call(from_args(args)?),
        "a2a_card" => {
            let a: A2aCardArgs = from_args(args)?;
            Ok(Invocation::new(&["a2a", name("agent", &a.agent)?, "card"]).raw(a.raw))
        }
        "a2a_send" => a2a_send(from_args(args)?),
        "a2a_task_get" => a2a_task("get", from_args(args)?),
        "a2a_task_cancel" => a2a_task("cancel", from_args(args)?),
        other => Err(format!("unknown tool '{other}'")),
    }
}

fn mcp_call(a: McpCallArgs) -> Result<Invocation, String> {
    let (server, tool) = (name("server", &a.server)?, name("tool", &a.tool)?);
    let mut inv = Invocation::new(&["mcp", server, "call", tool]);
    if !a.args.is_empty() {
        inv.push("--args");
        inv.push("-");
        inv.stdin = Some(Value::Object(a.args).to_string());
    }
    Ok(inv.raw(a.raw))
}

fn a2a_send(a: A2aSendArgs) -> Result<Invocation, String> {
    let mut inv = Invocation::new(&["a2a", name("agent", &a.agent)?, "send"]);
    // 値は `--opt=value` の形で渡し、`-` で始まる値もオプションと誤解されないようにする
    if let Some(c) = &a.context_id {
        inv.push(format!("--context-id={c}"));
    }
    if let Some(t) = &a.task_id {
        inv.push(format!("--task-id={t}"));
    }
    if a.no_wait {
        inv.push("--no-wait");
    }
    inv.push("--");
    inv.push(a.text);
    Ok(inv.raw(a.raw))
}

fn a2a_task(action: &str, a: A2aTaskArgs) -> Result<Invocation, String> {
    let (agent, id) = (name("agent", &a.agent)?, name("id", &a.id)?);
    Ok(Invocation::new(&["a2a", agent, "task", action, id]).raw(a.raw))
}

#[cfg(test)]
mod tests;
