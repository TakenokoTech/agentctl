[← Documentation](../index.md)

# Development

```sh
git config core.hooksPath .agent/hooks   # run the checks below before every commit
cargo test                               # unit tests and end-to-end tests
```

The end-to-end tests start the real binary against a test MCP server and a test A2A agent from `tests/fixtures/`. No network access or external servers are needed.

The code follows a four-layer DDD structure (`domain`, `application`, `infrastructure`, `presentation`). See [`AGENTS.md`](../../AGENTS.md) for the conventions.
