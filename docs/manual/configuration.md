[← Documentation](../index.md)

# Configuration

`agentctl` reads a TOML file. Unknown keys are rejected.

## MCP servers

Each server is a `[mcp.<name>]` table with a `transport` of `stdio` or `http`. Each transport has its own keys: `stdio` takes `command` (required) and `env`; `http` takes `url` (required) and `token_env`. A key that belongs to the other transport (for example `token_env` on a `stdio` server) is an error.

```toml
[mcp.local]
description = "Shown by `agentctl list`"
transport = "stdio"
command = ["path/to/server", "--flag"]
env = { API_KEY = "value" }          # optional, passed to the server process

[mcp.remote]
transport = "http"                   # Streamable HTTP
url = "https://mcp.example.com/mcp"
token_env = "REMOTE_MCP_TOKEN"       # optional
```

The stderr of stdio servers is discarded so that server logs do not leak into the AI agent's context. If a server fails to start or to answer, the last lines of its stderr are added to the error message.

## A2A agents

```toml
[a2a.helper]
description = "Shown by `agentctl list`"
url = "http://127.0.0.1:3000"
token_env = "HELPER_TOKEN"           # optional
```

## Authentication

`token_env` names an environment variable. Its value is sent as a Bearer token. If the variable is not set or is empty, `agentctl` exits with an error instead of sending an unauthenticated request.
