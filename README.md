# agentctl

English | [日本語](README.ja.md)

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

The output above comes from the test servers used by this repository's test suite. [Quick start](#quick-start) shows how to reproduce the MCP part.

## Installation

`agentctl` is not published to crates.io. Build it from source with Rust 1.85 or later:

```sh
git clone <this repository>
cd agentctl
cargo install --path .
```

## Quick start

Build the test MCP server that ships with the repository, and describe it in `agentctl.toml` (run the commands from the repository root):

```sh
cargo build --example test_mcp_server
```

```toml
[mcp.demo]
description = "Demo MCP server"
transport = "stdio"
command = ["target/debug/examples/test_mcp_server"]
```

Then explore and call:

```sh
agentctl list
agentctl mcp demo tools
agentctl mcp demo schema echo
agentctl mcp demo call echo --args '{"message":"hi"}'
```

To use your own servers and agents, add them to the config as described in [Configuration](docs/manual/configuration.md).

## Documentation

- [Overview](docs/index.md): features, examples, and how to use it from an AI agent
- Manual: [Usage](docs/manual/usage.md) · [Configuration](docs/manual/configuration.md) · [Reference](docs/manual/reference.md) · [Development](docs/manual/development.md)
