# agentctl documentation

[← README](../README.md) | English | [日本語](ja/index.md)

Call [MCP](https://modelcontextprotocol.io) servers and [A2A](https://a2a-protocol.org) agents from a single command-line tool.

`agentctl` is built for AI agents that work through a shell. Instead of loading every tool definition into the AI agent's context, it starts with `agentctl list` and drills down only as far as it needs.

```console
$ agentctl list
mcp	demo	Demo MCP server
a2a	helper	Demo A2A agent

$ agentctl mcp demo tools
echo	Echo the message
structured	Return structured content
fail	Always return an error result

$ agentctl mcp demo call echo --args '{"message":"hi"}'
echo: hi
```

The output above comes from the test servers used by this repository's test suite. [Quick start](../README.md#quick-start) shows how to reproduce the MCP part.

> **Terms.** An *AI agent* is the program that runs `agentctl`. An *A2A agent* is a remote agent that `agentctl` talks to.

## Features

- **One entry point** for MCP servers (stdio and Streamable HTTP) and A2A agents
- **Progressive disclosure**: `list` → `tools` → `schema` → `call`, so only the needed definitions reach the AI agent
- **Compact output by default**: TSV for listings, one-line JSON otherwise (except `mcp call`, which prints text content as is), and `--raw` for the full protocol response of `mcp call` and the A2A commands
- **Distinct exit codes** that tell "bad invocation" apart from "the tool failed"
- **Long-running A2A tasks**: send without waiting, then `get`, `watch` (streamed as NDJSON), or `cancel`

## Examples

### A2A example

With an A2A agent configured as below, `send` returns the reply:

```toml
[a2a.helper]
description = "Demo A2A agent"
url = "http://127.0.0.1:3000"
```

```console
$ agentctl a2a helper send hello
{"contextId":"152860f9-0f62-4e2d-aaae-b54324b88064","text":"echo: hello","type":"message"}
```

A long-running request returns a task. Send it without waiting, then watch it (one JSON event per line):

```console
$ agentctl a2a helper send "slow report" --no-wait
{"artifacts":[],"contextId":"d1bf1619-2ba3-436d-aed8-6f0b2083e33a","id":"8253770c-a237-43c7-8742-e1b1ac096ff6","message":null,"state":"working","type":"task"}
$ agentctl a2a helper task watch 8253770c-a237-43c7-8742-e1b1ac096ff6
{"artifacts":[],"contextId":"d1bf1619-2ba3-436d-aed8-6f0b2083e33a","id":"8253770c-a237-43c7-8742-e1b1ac096ff6","message":null,"state":"working","type":"task"}
{"message":null,"state":"completed","taskId":"8253770c-a237-43c7-8742-e1b1ac096ff6","type":"status"}
```

### Exit codes in practice

A tool that reports an error exits with `2`, and the error text is still printed:

```console
$ agentctl mcp demo call fail
boom
$ echo $?
2
```

A task that ends as `failed` also exits with `2`:

```console
$ agentctl a2a helper send "fail it"
{"artifacts":[],"contextId":"03528ca0-a394-4643-9f1b-737c612b91b2","id":"4dd01a05-b4d8-4b13-b978-7199799df979","message":null,"state":"failed","type":"task"}
$ echo $?
2
```

The A2A outputs above come from the test agent used by this repository's test suite.

## Using it from an AI agent

Tell the AI agent to start with `agentctl list` and to go one level deeper only when it needs to:

```text
Use `agentctl` to reach external tools and agents.
1. `agentctl list` shows what is available.
2. `agentctl mcp <server> tools`, then `agentctl mcp <server> schema <tool>`, then `agentctl mcp <server> call <tool> --args '<json>'`.
3. `agentctl a2a <agent> card`, then `agentctl a2a <agent> send "<text>"`.
Exit code 2 means the tool or task itself failed; 1 means the call could not be made; 64 means the command line was invalid.
```

## Manual

- [Usage](manual/usage.md): commands, options, and passing tool arguments
- [Configuration](manual/configuration.md): the config file and authentication
- [Reference](manual/reference.md): exit codes, troubleshooting, and known limitations
- [Development](manual/development.md): building, testing, and the code structure
