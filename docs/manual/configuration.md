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
allow_insecure = false               # optional, see Authentication
```

The stderr of stdio servers is discarded so that server logs do not leak into the AI agent's context.

## A2A agents

```toml
[a2a.helper]
description = "Shown by `agentctl list`"
url = "http://127.0.0.1:3000"
token_env = "HELPER_TOKEN"           # optional
allow_insecure = false               # optional, see Authentication
```

## Authentication

`token_env` names an environment variable. Its value is sent as a Bearer token. If the variable is not set, `agentctl` exits with an error instead of sending an unauthenticated request.

The token is sent only over `https://`, or over `http://` to a loopback host (`localhost`, `127.0.0.1`, `[::1]`). For any other `http://` URL, `agentctl` exits with an error before sending anything. If you accept the risk of sending the token in clear text, for example inside a closed network, set `allow_insecure = true` on that entry. `allow_insecure` is accepted only on `http` MCP servers and A2A agents.

For an A2A agent, the token is also sent only to the origin (scheme, host, and port) of the configured `url`. If the agent card advertises a JSON-RPC or HTTP+JSON endpoint on another origin, `agentctl` exits with an error before the request. Set `url` to the origin that the card advertises.

With a token, `agentctl` also refuses to follow an HTTP redirect to a different origin, including a change of scheme or port alone.
