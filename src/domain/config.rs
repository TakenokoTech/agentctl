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
#[serde(from = "RawMcpServer")]
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

/// 設定ファイル上の MCP サーバ。`flatten` では `deny_unknown_fields` が効かないため、
/// キーを全部1か所に並べて未知のキーを弾き、`McpServer` に変換する。
#[derive(Deserialize)]
#[serde(tag = "transport", rename_all = "lowercase", deny_unknown_fields)]
enum RawMcpServer {
    Stdio {
        description: Option<String>,
        command: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
    },
    Http {
        description: Option<String>,
        url: String,
        token_env: Option<String>,
    },
}

impl From<RawMcpServer> for McpServer {
    fn from(raw: RawMcpServer) -> Self {
        match raw {
            RawMcpServer::Stdio {
                description,
                command,
                env,
            } => McpServer {
                description,
                transport: McpTransport::Stdio { command, env },
            },
            RawMcpServer::Http {
                description,
                url,
                token_env,
            } => McpServer {
                description,
                transport: McpTransport::Http { url, token_env },
            },
        }
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
