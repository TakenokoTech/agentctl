[← Documentation](../index.md)

# Configuration

`agentctl` reads a TOML file. Unknown keys are rejected.

## MCP servers

Each server is a `[mcp.<name>]` table with a `transport` of `stdio` or `http`.

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

The stderr of stdio servers is discarded so that server logs do not leak into the AI agent's context.

## A2A agents

```toml
[a2a.helper]
description = "Shown by `agentctl list`"
url = "http://127.0.0.1:3000"
token_env = "HELPER_TOKEN"           # optional
```

## Authentication

`token_env` names an environment variable. Its value is sent as a Bearer token. If the variable is not set, `agentctl` exits with an error instead of sending an unauthenticated request.
