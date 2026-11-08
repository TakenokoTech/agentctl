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
| `a2a <agent> task watch <id>` | Stream state changes until the task ends | NDJSON |
| `a2a <agent> task cancel <id>` | Cancel a task | JSON |

## Options

| Option | Description |
|---|---|
| `--config <PATH>` | Config file path. Defaults to `agentctl.toml`. Can also be set with `AGENTCTL_CONFIG`. |
| `--raw` | Print the full protocol response instead of a compact summary. |

## Passing tool arguments

`--args` takes a JSON object. When omitted it defaults to `{}`. Pass `-` to read the object from stdin; stdin is read only in that case, so a call without `--args` never blocks waiting for input.

```sh
echo '{"message":"from stdin"}' | agentctl mcp demo call echo --args -
```

## Long-running A2A tasks

`send --no-wait` returns as soon as the A2A agent accepts the task. Follow up with `task get`, `task watch`, or `task cancel`. Task states are printed in short snake case, for example `input_required`; to answer a task in that state, run `send` again with `--task-id`.
