[← Documentation](../index.md)

# Reference

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Configuration, transport, or protocol error (including a malformed `--args`) |
| `2` | The tool returned an error result, or the A2A task ended as `failed` or `rejected` |

Command-line usage errors reported by the argument parser also exit with `2`.

## Troubleshooting

Errors are printed to stderr as `error: …` and exit with `1`.

| Message | Cause and fix |
|---|---|
| `cannot read config agentctl.toml` | There is no config file in the current directory. Pass `--config <PATH>` or set `AGENTCTL_CONFIG`. |
| `invalid config …` | The file has an unknown key, or an MCP server lacks a `transport` of `stdio` or `http`. |
| `unknown MCP server 'x'` / `unknown A2A agent 'x'` | The name is not in the config. Run `agentctl list`. |
| `unknown tool 'x'` | The server has no such tool. Run `agentctl mcp <server> tools`. |
| `env var NAME is not set` | `token_env` names a variable that is not set in the environment. |
| `failed to spawn …` / `` `command` is empty `` | The `command` of a stdio server cannot be started or is empty. |
| `--args must be a JSON object` | `--args` is not valid JSON, or is not an object. |

## Known limitations

- Every `mcp … call` starts a new connection. For stdio servers that also means starting the server process each time, which can take over a second for servers launched through `npx`.
- Credentials are read from environment variables of the process that runs `agentctl`, so an AI agent that can run the CLI can also read them.
