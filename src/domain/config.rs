use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Deserialize;

/// 登録済みの MCP サーバと A2A エージェント。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    mcp: BTreeMap<String, McpServer>,
    #[serde(default)]
    a2a: BTreeMap<String, A2aAgent>,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "RawMcpServer")]
pub struct McpServer {
    pub description: Option<String>,
    pub transport: McpTransport,
}

#[derive(Debug)]
pub enum McpTransport {
    Stdio {
        command: Vec<String>,
        env: BTreeMap<String, String>,
    },
    Http {
        url: String,
        token_env: Option<String>,
    },
}

/// `transport` の値で、使えるキーが変わる。`flatten` では未知のキーを拒否できないため、
/// すべてのキーを受け取ったうえで、`transport` に合うかを検証する。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMcpServer {
    description: Option<String>,
    transport: TransportKind,
    command: Option<Vec<String>>,
    env: Option<BTreeMap<String, String>>,
    url: Option<String>,
    token_env: Option<String>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum TransportKind {
    Stdio,
    Http,
}

impl TransportKind {
    fn name(self) -> &'static str {
        match self {
            TransportKind::Stdio => "stdio",
            TransportKind::Http => "http",
        }
    }
}

fn reject<T>(kind: TransportKind, key: &str, value: &Option<T>) -> Result<(), String> {
    match value {
        Some(_) => Err(format!(
            "`{key}` is not valid for transport = \"{}\"",
            kind.name()
        )),
        None => Ok(()),
    }
}

fn require<T>(kind: TransportKind, key: &str, value: Option<T>) -> Result<T, String> {
    value.ok_or_else(|| format!("`{key}` is required for transport = \"{}\"", kind.name()))
}

impl TryFrom<RawMcpServer> for McpServer {
    type Error = String;

    fn try_from(raw: RawMcpServer) -> Result<Self, String> {
        let kind = raw.transport;
        let transport = match kind {
            TransportKind::Stdio => {
                reject(kind, "url", &raw.url)?;
                reject(kind, "token_env", &raw.token_env)?;
                McpTransport::Stdio {
                    command: require(kind, "command", raw.command)?,
                    env: raw.env.unwrap_or_default(),
                }
            }
            TransportKind::Http => {
                reject(kind, "command", &raw.command)?;
                reject(kind, "env", &raw.env)?;
                McpTransport::Http {
                    url: require(kind, "url", raw.url)?,
                    token_env: raw.token_env,
                }
            }
        };
        Ok(McpServer {
            description: raw.description,
            transport,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct A2aAgent {
    pub url: String,
    pub description: Option<String>,
    pub token_env: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Mcp,
    A2a,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Mcp => "mcp",
            Kind::A2a => "a2a",
        }
    }
}

/// `list` の1行分。
#[derive(Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    pub kind: Kind,
    pub name: &'a str,
    pub description: &'a str,
}

impl Config {
    pub fn mcp(&self, name: &str) -> Result<&McpServer> {
        self.mcp
            .get(name)
            .with_context(|| format!("unknown MCP server '{name}' (see `agentctl list`)"))
    }

    pub fn a2a(&self, name: &str) -> Result<&A2aAgent> {
        self.a2a
            .get(name)
            .with_context(|| format!("unknown A2A agent '{name}' (see `agentctl list`)"))
    }

    /// MCP、A2A の順に、名前順で返す。
    pub fn entries(&self) -> Vec<Entry<'_>> {
        let mcp = self.mcp.iter().map(|(n, s)| (Kind::Mcp, n, &s.description));
        let a2a = self.a2a.iter().map(|(n, a)| (Kind::A2a, n, &a.description));
        mcp.chain(a2a)
            .map(|(kind, name, desc)| Entry {
                kind,
                name,
                description: desc.as_deref().unwrap_or(""),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
