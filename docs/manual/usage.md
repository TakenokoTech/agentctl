[← Documentation](../index.md)

# Usage

```
agentctl [OPTIONS] <COMMAND>
```

| Command | Description | Output |
|---|---|---|
| `list` | List configured MCP servers and A2A agents | TSV: kind, name, description |
| `mcp <server> tools` | List a server's tools | TSV: name, first line of description |
| `mcp <server> schema <tool>` | Show a tool's description and input JSON Schema | JSON |
| `mcp <server> call <tool> [--args JSON\|-]` | Call a tool | `structuredContent` if present, otherwise text |
| `a2a <agent> card` | Show the agent card (description, skills, streaming support) | JSON |
| `a2a <agent> send <text> [--context-id ID] [--task-id ID] [--no-wait]` | Send a message | JSON (`type`: `message` or `task`) |
| `a2a <agent> task get <id>` | Show a task's current state | JSON |
| `a2a <agent> task watch <id>` | Stream state changes until the task finishes | NDJSON |
| `a2a <agent> task cancel <id>` | Cancel a task | JSON |

In TSV output, tabs and newlines inside a description are replaced with a single space, so each entry stays on one line.

The text of `a2a send` may start with `-` (for example `agentctl a2a helper send "-1 is the answer"`).

## Options

| Option | Description |
|---|---|
| `--config <PATH>` | Config file path. Defaults to `agentctl.toml`. Can also be set with `AGENTCTL_CONFIG`. |
| `--raw` | Print the full protocol response instead of a compact summary. Applies to `mcp call` and the `a2a` commands; `list`, `mcp tools`, and `mcp schema` ignore it. |
| `--timeout <SECONDS>` | Give up on a connection or request after this many seconds. Defaults to `60`; `0` means no limit. Can also be set with `AGENTCTL_TIMEOUT`. For `a2a task watch`, it limits only the connection, because the command runs until the task ends. |

## Passing tool arguments

`--args` takes a JSON object. When omitted it defaults to `{}`. Pass `-` to read the object from stdin; stdin is read only in that case, so a call without `--args` never blocks waiting for input.

```sh
echo '{"message":"from stdin"}' | agentctl mcp demo call echo --args -
```

## Long-running A2A tasks

`send --no-wait` returns as soon as the A2A agent accepts the task. Follow up with `task get`, `task watch`, or `task cancel`. Task states are printed in short snake case, for example `input_required`; to answer a task in that state, run `send` again with `--task-id`.

`task watch` prints each event and stops when the task reaches a final state (`completed`, `failed`, `canceled`, or `rejected`), even if the agent keeps the stream open. The exit code follows that state: `0`, except `2` for `failed` and `rejected`. If the stream closes before the task reaches a final state, the events received so far are printed and the command exits with `1`, unless the task was waiting for input (`input_required` or `auth_required`), in which case the exit code follows that state (`0`).

## Running as an MCP server

`agentctl-mcp` starts an MCP server on stdio, so an MCP client can use `agentctl` without a shell. It is a separate program: for every tool call it runs `agentctl` and returns the result, and it does not depend on `agentctl`'s internals. It is installed together with `agentctl` by `cargo install --path .`.

```json
{
  "mcpServers": {
    "agentctl": {
      "command": "agentctl-mcp",
      "env": { "AGENTCTL_CONFIG": "/path/to/agentctl.toml" }
    }
  }
}
```

| Environment variable | Description |
|---|---|
| `AGENTCTL_BIN` | Path to `agentctl`. Defaults to `agentctl` on the `PATH`. |
| `AGENTCTL_CONFIG` | Config file for `agentctl` (see [Options](#options)). The server passes its environment to `agentctl`. |

Each command is a tool. The arguments are the command's arguments, and `raw` is accepted wherever the CLI accepts `--raw`.

| Tool | Same as | Arguments |
|---|---|---|
| `list` | `list` | none |
| `mcp_tools` | `mcp <server> tools` | `server` |
| `mcp_schema` | `mcp <server> schema <tool>` | `server`, `tool` |
| `mcp_call` | `mcp <server> call <tool>` | `server`, `tool`, `args` (object), `raw` |
| `a2a_card` | `a2a <agent> card` | `agent`, `raw` |
| `a2a_send` | `a2a <agent> send <text>` | `agent`, `text`, `context_id`, `task_id`, `no_wait`, `raw` |
| `a2a_task_get` | `a2a <agent> task get <id>` | `agent`, `id`, `raw` |
| `a2a_task_cancel` | `a2a <agent> task cancel <id>` | `agent`, `id`, `raw` |

The result text is the output of `agentctl`. The result is marked as an error (`isError`) whenever `agentctl` exits with a non-zero code: for exit code `2` the text is the tool's or task's output, and for exit code `1` it is the message that starts with `error: `. If `agentctl` cannot be started, the text also starts with `error: `.

Missing, unknown, or mistyped arguments are rejected as protocol errors, and so are `server`, `tool`, `agent`, and `id` values that are empty or start with `-`, so a tool argument can never be read as an option of `agentctl`.

`a2a task watch` is not available because a tool call returns a single result. Poll `a2a_task_get` instead.
